//! Unidirectional light sources.

use crate::{
    geometry::{AxisAlignedBox, Frustum, OrthographicTransform, Sphere},
    rendering::fre,
    scene::{
        DirectionComp, DirectionalLightComp, LightDirection, LightStorage, Radiance, RadianceComp,
        RenderResourcesDesynchronized, SceneCamera,
    },
};
use bytemuck::{Pod, Zeroable};
use impact_ecs::{archetype::ArchetypeComponentStorage, setup, world::EntityEntry};
use nalgebra::{vector, Scale3, Similarity3, Translation3, UnitQuaternion, UnitVector3, Vector3};
use std::sync::RwLock;

/// An directional light source represented by a camera space direction and an
/// RGB radiance. The struct also includes a rotation quaternion that defines
/// the orientation of the light's local coordinate system with respect to
/// camera space, and an orthographic transformation that maps the light's space
/// to clip space in such a way as to include all objects in the scene that may
/// cast shadows inside or into the camera view frustum.
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
pub struct DirectionalLight {
    camera_to_light_space_rotation: UnitQuaternion<fre>,
    camera_space_direction: LightDirection,
    // Padding to obtain 16-byte alignment for next field
    orthographic_half_extent_x: fre,
    radiance: Radiance,
    // Padding to obtain 16-byte alignment for next field
    orthographic_half_extent_y: fre,
    orthographic_translation: Translation3<fre>,
    // Padding to obtain 16-byte alignment for next field
    orthographic_half_extent_z: fre,
    orthographic_scaling: Scale3<fre>,
    // Padding to make size multiple of 16-bytes
    _padding: fre,
}

impl DirectionalLight {
    fn new(camera_space_direction: LightDirection, radiance: Radiance) -> Self {
        Self {
            camera_to_light_space_rotation: Self::compute_camera_to_light_space_rotation(
                &camera_space_direction,
            ),
            camera_space_direction,
            orthographic_half_extent_x: 0.0,
            radiance,
            orthographic_half_extent_y: 0.0,
            orthographic_translation: Translation3::identity(),
            orthographic_half_extent_z: 0.0,
            orthographic_scaling: Scale3::identity(),
            _padding: 0.0,
        }
    }

    /// Takes a transform into camera space and returns the corresponding
    /// transform into the light's space.
    pub fn create_transform_to_light_space(
        &self,
        transform_to_camera_space: &Similarity3<fre>,
    ) -> Similarity3<fre> {
        self.camera_to_light_space_rotation * transform_to_camera_space
    }

    /// Sets the camera space direction of the light to the given direction.
    pub fn set_camera_space_direction(&mut self, camera_space_direction: LightDirection) {
        self.camera_space_direction = camera_space_direction;
        self.camera_to_light_space_rotation =
            Self::compute_camera_to_light_space_rotation(&camera_space_direction);
    }

    /// Updates the light's orthographic transform so that all objects in the
    /// scene within or in front of the camera view frustum with respect to the
    /// light, i.e. all objects that may cast visible shadows, will be included
    /// in the light's clip space.
    pub fn bound_orthographic_transform_to_view_frustum(
        &mut self,
        camera_space_view_frustum: &Frustum<fre>,
        camera_space_bounding_sphere: &Sphere<fre>,
    ) {
        // Rotate to light space, where the light direction is -z
        let light_space_view_frustum =
            camera_space_view_frustum.rotated(&self.camera_to_light_space_rotation);
        let light_space_bounding_sphere =
            camera_space_bounding_sphere.rotated(&self.camera_to_light_space_rotation);

        // Use the bounds of the view frustum in light space along with the
        // bounding sphere to constrain limits for orthographic projection
        let light_space_view_frustum_aabb = light_space_view_frustum.compute_aabb();

        let bounding_sphere_center = light_space_bounding_sphere.center();
        let bounding_sphere_radius = light_space_bounding_sphere.radius();

        let view_frustum_aabb_lower_corner = light_space_view_frustum_aabb.lower_corner();
        let view_frustum_aabb_upper_corner = light_space_view_frustum_aabb.upper_corner();

        // Constrain limits using either the view frustum or the bounding
        // volume, depending on which gives the snuggest fit
        let left = fre::max(
            view_frustum_aabb_lower_corner.x,
            bounding_sphere_center.x - bounding_sphere_radius,
        );
        let right = fre::min(
            view_frustum_aabb_upper_corner.x,
            bounding_sphere_center.x + bounding_sphere_radius,
        );

        let bottom = fre::max(
            view_frustum_aabb_lower_corner.y,
            bounding_sphere_center.y - bounding_sphere_radius,
        );
        let top = fre::min(
            view_frustum_aabb_upper_corner.y,
            bounding_sphere_center.y + bounding_sphere_radius,
        );

        // For the near plane we use the point on the bounding sphere farthest
        // towards the light source, as models between the light and the view
        // frustum may cast shadows into the frustum
        let near = light_space_bounding_sphere.center().z + light_space_bounding_sphere.radius();
        let far = fre::max(
            view_frustum_aabb_lower_corner.z,
            bounding_sphere_center.z - bounding_sphere_radius,
        );

        (self.orthographic_translation, self.orthographic_scaling) =
            OrthographicTransform::compute_orthographic_translation_and_scaling(
                left, right, bottom, top, near, far,
            );

        self.orthographic_half_extent_x = 0.5 * (right - left);
        self.orthographic_half_extent_y = 0.5 * (top - bottom);
        self.orthographic_half_extent_z = -0.5 * (far - near);
    }

    /// Determines whether the object with the given camera space bounding
    /// sphere would be included in the light's clip space, meaning that it
    /// could potentially cast a visible shadow.
    pub fn bounding_sphere_may_cast_visible_shadow(
        &self,
        camera_space_bounding_sphere: &Sphere<fre>,
    ) -> bool {
        let light_space_bounding_sphere =
            camera_space_bounding_sphere.rotated(&self.camera_to_light_space_rotation);

        let orthographic_half_extents = vector![
            self.orthographic_half_extent_x,
            self.orthographic_half_extent_y,
            self.orthographic_half_extent_z
        ];

        let orthographic_lower_corner =
            -orthographic_half_extents - self.orthographic_translation.vector;
        let orthographic_upper_corner =
            orthographic_half_extents - self.orthographic_translation.vector;

        let orthographic_aabb = AxisAlignedBox::new(
            orthographic_lower_corner.into(),
            orthographic_upper_corner.into(),
        );

        !light_space_bounding_sphere.is_outside_axis_aligned_box(&orthographic_aabb)
    }

    /// Checks if the entity-to-be with the given components has the right
    /// components for this light source, and if so, adds the corresponding
    /// [`DirectionalLight`] to the light storage and adds a
    /// [`DirectionalLightComp`] with the light's ID to the entity.
    pub fn add_directional_light_component_for_entity(
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
            |direction: &DirectionComp, radiance: &RadianceComp| -> DirectionalLightComp {
                let directional_light = Self::new(
                    // The view transform contains no scaling, so the direction remains normalized
                    LightDirection::new_unchecked(
                        view_transform.transform_vector(&direction.0.cast()),
                    ),
                    radiance.0,
                );
                let id = light_storage.add_directional_light(directional_light);

                DirectionalLightComp { id }
            },
            ![DirectionalLightComp]
        );
    }

    /// Checks if the given entity has a [`DirectionalLightComp`], and if so,
    /// removes the assocated [`DirectionalLight`] from the given
    /// [`LightStorage`].
    pub fn remove_light_from_storage(
        light_storage: &RwLock<LightStorage>,
        entity: &EntityEntry<'_>,
        desynchronized: &mut RenderResourcesDesynchronized,
    ) {
        if let Some(directional_light) = entity.get_component::<DirectionalLightComp>() {
            let light_id = directional_light.access().id;
            light_storage
                .write()
                .unwrap()
                .remove_directional_light(light_id);
            desynchronized.set_yes();
        }
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
