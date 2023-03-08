//! Omnidirectional light sources.

use crate::{
    geometry::{CubeMapper, CubemapFace, Frustum, Sphere},
    physics::PositionComp,
    rendering::fre,
    scene::{
        LightStorage, Omnidirectional, PointLightComp, Radiance, RadianceComp,
        RenderResourcesDesynchronized, SceneCamera,
    },
};
use bytemuck::{Pod, Zeroable};
use impact_ecs::{archetype::ArchetypeComponentStorage, setup, world::EntityEntry};
use nalgebra::{Point3, Similarity3, Translation3, UnitQuaternion};
use std::sync::RwLock;

/// An point light source represented by a camera space position and an RGB
/// radiance. The struct also includes a rotation quaternion that defines the
/// orientation of the light's local coordinate system with respect to camera
/// space, and a near and far distance restricting the distance range in which
/// the light can cast shadows.
///
/// This struct is intended to be stored in a [`LightStorage`], and its data
/// will be passed directly to the GPU in a uniform buffer. Importantly, its
/// size is a multiple of 16 bytes as required for uniforms, and the fields that
/// will be accessed on the GPU are aligned to 16-byte boundaries.
///
/// # Warning
/// The fields must not be reordered, as this ordering is expected by the
/// shader.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
pub struct PointLight {
    camera_to_light_space_rotation: UnitQuaternion<fre>,
    camera_space_position: Point3<fre>,
    // Padding to obtain 16-byte alignment for next field
    _padding_1: fre,
    radiance: Radiance,
    // Padding to obtain 16-byte alignment for next field (the `near_distance`
    // and `inverse_distance_span` fields are accessed as a struct in a single
    // field in the shader)
    _padding_2: fre,
    near_distance: fre,
    inverse_distance_span: fre,
    // Padding to make size multiple of 16-bytes
    far_distance: fre,
    _padding_3: fre,
}

impl PointLight {
    const MIN_NEAR_DISTANCE: fre = 1e-2;
    const MAX_FAR_DISTANCE: fre = fre::INFINITY;

    fn new(camera_space_position: Point3<fre>, radiance: Radiance) -> Self {
        Self {
            camera_to_light_space_rotation: UnitQuaternion::identity(),
            camera_space_position,
            _padding_1: 0.0,
            radiance,
            _padding_2: 0.0,
            near_distance: 0.0,
            inverse_distance_span: 0.0,
            far_distance: 0.0,
            _padding_3: 0.0,
        }
    }

    /// Takes a transform into camera space and returns the corresponding
    /// transform into the space of the positive z face for points lying in
    /// front of the given face.
    pub fn create_transform_to_positive_z_cubemap_face_space(
        &self,
        face: CubemapFace,
        transform_to_camera_space: &Similarity3<fre>,
    ) -> Similarity3<fre> {
        CubeMapper::rotation_to_positive_z_face_from_face(face)
            * self.create_camera_to_light_space_transform()
            * transform_to_camera_space
    }

    /// Sets the camera space position of the light to the given position.
    pub fn set_camera_space_position(&mut self, camera_space_position: Point3<fre>) {
        self.camera_space_position = camera_space_position;
    }

    pub fn orient_and_scale_cubemap_for_view_frustum(
        &mut self,
        camera_space_view_frustum: &Frustum<fre>,
        camera_space_bounding_sphere: &Sphere<fre>,
    ) {
        let bounding_sphere_center_distance = nalgebra::distance(
            &self.camera_space_position,
            camera_space_bounding_sphere.center(),
        );

        self.near_distance = fre::clamp(
            bounding_sphere_center_distance - camera_space_bounding_sphere.radius(),
            Self::MIN_NEAR_DISTANCE,
            Self::MAX_FAR_DISTANCE - 1e-9,
        );

        self.far_distance = fre::clamp(
            bounding_sphere_center_distance + camera_space_bounding_sphere.radius(),
            self.near_distance + 1e-9,
            Self::MAX_FAR_DISTANCE,
        );

        self.inverse_distance_span = 1.0 / (self.far_distance - self.near_distance);
    }

    pub fn compute_camera_space_frustum_for_face(&self, face: CubemapFace) -> Frustum<fre> {
        CubeMapper::compute_frustum_for_face(
            face,
            &self.create_camera_to_light_space_transform(),
            self.near_distance,
            self.far_distance,
        )
    }

    /// Checks if the entity-to-be with the given components has the right
    /// components for this light source, and if so, adds the corresponding
    /// [`PointLight`] to the light storage and adds a [`PointLightComp`] with
    /// the light's ID to the entity.
    pub fn add_point_light_component_for_entity(
        scene_camera: &RwLock<Option<SceneCamera<fre>>>,
        light_storage: &RwLock<LightStorage>,
        components: &mut ArchetypeComponentStorage,
        desynchronized: &mut RenderResourcesDesynchronized,
    ) {
        setup!(
            {
                desynchronized.set_yes();

                let view_transform = scene_camera
                    .read()
                    .unwrap()
                    .as_ref()
                    .map_or_else(Similarity3::identity, |scene_camera| {
                        *scene_camera.view_transform()
                    });

                let mut light_storage = light_storage.write().unwrap();
            },
            components,
            |position: &PositionComp, radiance: &RadianceComp| -> PointLightComp {
                let point_light = Self::new(
                    view_transform.transform_point(&position.0.cast()),
                    radiance.0,
                );
                let id = light_storage.add_point_light(point_light);

                PointLightComp { id }
            },
            [Omnidirectional],
            ![PointLightComp]
        );
    }

    /// Checks if the given entity has a [`PointLightComp`], and if so, removes
    /// the assocated [`PointLight`] from the given [`LightStorage`].
    pub fn remove_light_from_storage(
        light_storage: &RwLock<LightStorage>,
        entity: &EntityEntry<'_>,
        desynchronized: &mut RenderResourcesDesynchronized,
    ) {
        if let Some(point_light) = entity.get_component::<PointLightComp>() {
            let light_id = point_light.access().id;
            light_storage.write().unwrap().remove_point_light(light_id);
            desynchronized.set_yes();
        }
    }

    fn create_camera_to_light_space_transform(&self) -> Similarity3<fre> {
        Similarity3::from_parts(
            Translation3::from(-self.camera_space_position),
            self.camera_to_light_space_rotation,
            1.0,
        )
    }
}
