//! Omnidirectional light sources.

use crate::{
    geometry::{AxisAlignedBox, CubeMapper, CubemapFace, Frustum, Sphere},
    physics::ReferenceFrameComp,
    gpu::rendering::fre,
    scene::{
        LightStorage, LuminousIntensity, OmnidirectionalEmissionComp, OmnidirectionalLightComp,
        RenderResourcesDesynchronized, SceneCamera,
    },
};
use bytemuck::{Pod, Zeroable};
use impact_ecs::{archetype::ArchetypeComponentStorage, setup, world::EntityEntry};
use nalgebra::{
    self as na, Point3, Similarity3, Translation3, UnitQuaternion, UnitVector3, Vector3,
};
use std::sync::RwLock;

/// An omnidirectional light source represented by a camera space position, an
/// RGB luminous intensity and an extent. The struct also includes a rotation
/// quaternion that defines the orientation of the light's local coordinate
/// system with respect to camera space, and a near and far distance restricting
/// the distance range in which the light can cast shadows.
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
pub struct OmnidirectionalLight {
    camera_to_light_space_rotation: UnitQuaternion<fre>,
    camera_space_position: Point3<fre>,
    // Padding to obtain 16-byte alignment for next field
    _padding_1: fre,
    // Luminous intensity and emission radius are treated as a single
    // 4-component vector in the shader
    luminous_intensity: LuminousIntensity,
    emission_radius: fre,
    // The `near_distance` and `inverse_distance_span` fields are accessed as a
    // struct in a single field in the shader
    near_distance: fre,
    inverse_distance_span: fre,
    // Padding to make size multiple of 16-bytes
    far_distance: fre,
    _padding_2: fre,
}

impl OmnidirectionalLight {
    const MIN_NEAR_DISTANCE: fre = 1e-2;
    const MAX_FAR_DISTANCE: fre = fre::INFINITY;

    fn new(
        camera_space_position: Point3<fre>,
        luminous_intensity: LuminousIntensity,
        emission_extent: f32,
    ) -> Self {
        Self {
            camera_to_light_space_rotation: UnitQuaternion::identity(),
            camera_space_position,
            _padding_1: 0.0,
            luminous_intensity,
            emission_radius: 0.5 * emission_extent,
            near_distance: 0.0,
            inverse_distance_span: 0.0,
            far_distance: 0.0,
            _padding_2: 0.0,
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
        self.create_transform_from_camera_space_to_positive_z_cubemap_face_space(face)
            * transform_to_camera_space
    }

    /// Computes the transform from camera space into the space of the positive
    /// z face for points lying in front of the given face.
    pub fn create_transform_from_camera_space_to_positive_z_cubemap_face_space(
        &self,
        face: CubemapFace,
    ) -> Similarity3<fre> {
        CubeMapper::rotation_to_positive_z_face_from_face(face)
            * self.create_camera_to_light_space_transform()
    }

    /// Returns a reference to the camera space position of the light.
    pub fn camera_space_position(&self) -> &Point3<fre> {
        &self.camera_space_position
    }

    /// Sets the camera space position of the light to the given position.
    pub fn set_camera_space_position(&mut self, camera_space_position: Point3<fre>) {
        self.camera_space_position = camera_space_position;
    }

    /// Sets the luminous intensity of the light to the given value.
    pub fn set_luminous_intensity(&mut self, luminous_intensity: LuminousIntensity) {
        self.luminous_intensity = luminous_intensity;
    }

    /// Sets the emission extent of the light to the given value.
    pub fn set_emission_extent(&mut self, emission_extent: fre) {
        self.emission_radius = 0.5 * emission_extent;
    }

    /// Updates the cubemap orientation and near and far distances to encompass
    /// all shadow casting models without wasting depth resolution or causing
    /// unnecessary draw calls.
    pub fn orient_and_scale_cubemap_for_shadow_casting_models(
        &mut self,
        camera_space_bounding_sphere: &Sphere<fre>,
        camera_space_aabb_for_visible_models: Option<&AxisAlignedBox<fre>>,
    ) {
        let bounding_sphere_center_distance = na::distance(
            &self.camera_space_position,
            camera_space_bounding_sphere.center(),
        );

        let (camera_to_light_space_rotation, far_distance) = if let Some(
            camera_space_aabb_for_visible_models,
        ) =
            camera_space_aabb_for_visible_models
        {
            // Let the orientation of cubemap space be so that the negative
            // z-axis points towards the center of the volume containing visible
            // models
            let camera_to_light_space_rotation =
                Self::compute_camera_to_light_space_rotation(&UnitVector3::new_normalize(
                    camera_space_aabb_for_visible_models.center() - self.camera_space_position,
                ));

            // Use the farthest point of the volume containing visible models as
            // the far distance
            let far_distance = na::distance(
                &camera_space_aabb_for_visible_models
                    .compute_farthest_corner(&self.camera_space_position),
                &self.camera_space_position,
            );

            (camera_to_light_space_rotation, far_distance)
        } else {
            // In this case no models are visible, so the rotation does not
            // matter
            let camera_to_light_space_rotation = UnitQuaternion::identity();

            let far_distance =
                bounding_sphere_center_distance + camera_space_bounding_sphere.radius();

            (camera_to_light_space_rotation, far_distance)
        };

        self.camera_to_light_space_rotation = camera_to_light_space_rotation;

        // The near distance must never be farther than the closest model to the
        // light source
        self.near_distance = fre::clamp(
            bounding_sphere_center_distance - camera_space_bounding_sphere.radius(),
            Self::MIN_NEAR_DISTANCE,
            Self::MAX_FAR_DISTANCE - 1e-9,
        );

        self.far_distance = fre::clamp(
            far_distance,
            Self::MIN_NEAR_DISTANCE,
            Self::MAX_FAR_DISTANCE - 1e-9,
        );

        self.inverse_distance_span = 1.0 / (self.far_distance - self.near_distance);
    }

    /// Computes the frustum for the given cubemap face in camera space.
    pub fn compute_camera_space_frustum_for_face(&self, face: CubemapFace) -> Frustum<fre> {
        CubeMapper::compute_frustum_for_face(
            face,
            &self.create_camera_to_light_space_transform(),
            self.near_distance,
            self.far_distance,
        )
    }

    /// Whether the given cubemap face frustum may contain any visible models.
    pub fn camera_space_frustum_for_face_may_contain_visible_models(
        camera_space_aabb_for_visible_models: Option<&AxisAlignedBox<fre>>,
        camera_space_face_frustum: &Frustum<fre>,
    ) -> bool {
        if let Some(camera_space_aabb_for_visible_models) = camera_space_aabb_for_visible_models {
            !camera_space_face_frustum
                .compute_aabb()
                .box_lies_outside(camera_space_aabb_for_visible_models)
        } else {
            // In this case no models are visible
            false
        }
    }

    /// Checks if the entity-to-be with the given components has the right
    /// components for this light source, and if so, adds the corresponding
    /// [`OmnidirectionalLight`] to the light storage and adds a
    /// [`OmnidirectionalLightComp`] with the light's ID to the entity.
    pub fn add_omnidirectional_light_component_for_entity(
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
            |frame: &ReferenceFrameComp,
             omnidirectional_emission: &OmnidirectionalEmissionComp|
             -> OmnidirectionalLightComp {
                let omnidirectional_light = Self::new(
                    view_transform.transform_point(&frame.position.cast()),
                    omnidirectional_emission.luminous_intensity,
                    fre::max(omnidirectional_emission.source_extent, 0.0),
                );
                let id = light_storage.add_omnidirectional_light(omnidirectional_light);

                OmnidirectionalLightComp { id }
            },
            ![OmnidirectionalLightComp]
        );
    }

    /// Checks if the given entity has a [`OmnidirectionalLightComp`], and if so, removes
    /// the assocated [`OmnidirectionalLight`] from the given [`LightStorage`].
    pub fn remove_light_from_storage(
        light_storage: &RwLock<LightStorage>,
        entity: &EntityEntry<'_>,
        desynchronized: &mut RenderResourcesDesynchronized,
    ) {
        if let Some(omnidirectional_light) = entity.get_component::<OmnidirectionalLightComp>() {
            let light_id = omnidirectional_light.access().id;
            light_storage
                .write()
                .unwrap()
                .remove_omnidirectional_light(light_id);
            desynchronized.set_yes();
        }
    }

    fn create_camera_to_light_space_transform(&self) -> Similarity3<fre> {
        Similarity3::from_isometry(
            self.camera_to_light_space_rotation * Translation3::from(-self.camera_space_position),
            1.0,
        )
    }

    fn compute_camera_to_light_space_rotation(
        camera_space_direction: &UnitVector3<fre>,
    ) -> UnitQuaternion<fre> {
        let direction_is_very_close_to_vertical =
            fre::abs(camera_space_direction.y.abs() - 1.0) < 1e-3;

        // We orient the light's local coordinate system so that the light
        // direction in camera space maps to the -z-direction in light space,
        // and the y-direction in camera space maps to the y-direction in light
        // space, unless the light direction is nearly vertical in camera space,
        // in which case we map the -z-direction in camera space to the
        // y-direction in light space
        if direction_is_very_close_to_vertical {
            UnitQuaternion::look_at_rh(camera_space_direction, &-Vector3::z())
        } else {
            UnitQuaternion::look_at_rh(camera_space_direction, &Vector3::y())
        }
    }
}
