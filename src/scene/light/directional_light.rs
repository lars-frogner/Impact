//! Unidirectional light sources.

use crate::{
    geometry::{AxisAlignedBox, Frustum, OrthographicTransform, Sphere, UpperExclusiveBounds},
    rendering::{fre, CascadeIdx},
    scene::{
        DirectionComp, DirectionalLightComp, LightDirection, LightStorage, Radiance, RadianceComp,
        RenderResourcesDesynchronized, SceneCamera,
    },
};
use bytemuck::{Pod, Zeroable};
use impact_ecs::{archetype::ArchetypeComponentStorage, setup, world::EntityEntry};
use nalgebra::{Scale3, Similarity3, Translation3, UnitQuaternion, UnitVector3, Vector3};
use std::{iter, sync::RwLock};

/// Maximum number of cascades supported in a cascaded shadow map for
/// directional lights.
///
/// # Warning
/// Increasing this above 5 will require changes to the [`DirectionalLight`]
/// struct and associated shader code to meet uniform padding requirements.
pub const MAX_SHADOW_MAP_CASCADES: u32 = 4;

/// An directional light source represented by a camera space direction and an
/// RGB radiance. The struct also includes a rotation quaternion that defines
/// the orientation of the light's local coordinate system with respect to
/// camera space, orthographic transformations that map the light's space to
/// clip space in such a way as to include all objects in the scene that may
/// cast shadows inside or into specific cascades (partitions) of the camera
/// view frustum, and the camera clip space depths representing the boundaries
/// between the cascades.
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
    near_partition_depth: fre,
    radiance: Radiance,
    // Padding to obtain 16-byte alignment for next field
    far_partition_depth: fre,
    orthographic_transforms: [OrthographicTranslationAndScaling; MAX_SHADOW_MAP_CASCADES_USIZE],
    partition_depths: [fre; MAX_SHADOW_MAP_CASCADES_USIZE - 1],
    // Padding to make size multiple of 16-bytes
    _padding_3: [fre; 5 - MAX_SHADOW_MAP_CASCADES_USIZE],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
struct OrthographicTranslationAndScaling {
    translation: Translation3<fre>,
    // Padding to obtain 16-byte alignment for next field
    _padding_1: fre,
    scaling: Scale3<fre>,
    // Padding to make size multiple of 16-bytes
    _padding_2: fre,
}

const MAX_SHADOW_MAP_CASCADES_USIZE: usize = MAX_SHADOW_MAP_CASCADES as usize;

impl DirectionalLight {
    fn new(camera_space_direction: LightDirection, radiance: Radiance) -> Self {
        Self {
            camera_to_light_space_rotation: Self::compute_camera_to_light_space_rotation(
                &camera_space_direction,
            ),
            camera_space_direction,
            near_partition_depth: 0.0,
            radiance,
            far_partition_depth: 0.0,
            orthographic_transforms: [OrthographicTranslationAndScaling::zeroed();
                MAX_SHADOW_MAP_CASCADES_USIZE],
            partition_depths: [0.0; MAX_SHADOW_MAP_CASCADES_USIZE - 1],
            _padding_3: [0.0; 5 - MAX_SHADOW_MAP_CASCADES_USIZE],
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

    /// Updates the partition of view frustum cascades for the light based on
    /// the near and far distance required for encompassing visible models.
    pub fn update_cascade_partition_depths(
        &mut self,
        camera_space_view_frustum: &Frustum<fre>,
        camera_space_bounding_sphere: &Sphere<fre>,
    ) {
        const EXPONENTIAL_VS_LINEAR_PARTITION_WEIGHT: fre = 0.5;

        // Find the tightest near and far distance that encompass visible models
        let near_distance = fre::max(
            camera_space_view_frustum.near_distance(),
            -(camera_space_bounding_sphere.center().z + camera_space_bounding_sphere.radius()),
        );

        let far_distance = fre::min(
            camera_space_view_frustum.far_distance(),
            -(camera_space_bounding_sphere.center().z - camera_space_bounding_sphere.radius()),
        );

        // Use a blend between exponential and linear increase in the span of
        // cascades going from the near distance to the far distance

        let distance_ratio =
            (far_distance / near_distance).powf(1.0 / (MAX_SHADOW_MAP_CASCADES as fre));

        let distance_difference = (far_distance - near_distance) / (MAX_SHADOW_MAP_CASCADES as fre);

        let mut exponential_distance = near_distance;
        let mut linear_distance = near_distance;

        for partition_depth in &mut self.partition_depths {
            exponential_distance *= distance_ratio;
            linear_distance += distance_difference;

            let distance = EXPONENTIAL_VS_LINEAR_PARTITION_WEIGHT * exponential_distance
                + (1.0 - EXPONENTIAL_VS_LINEAR_PARTITION_WEIGHT) * linear_distance;

            *partition_depth =
                camera_space_view_frustum.convert_view_distance_to_clip_space_depth(distance);
        }

        self.near_partition_depth =
            camera_space_view_frustum.convert_view_distance_to_clip_space_depth(near_distance);
        self.far_partition_depth =
            camera_space_view_frustum.convert_view_distance_to_clip_space_depth(far_distance);
    }

    /// Updates the light's orthographic transforms so that all objects in the
    /// scene within or in front of each cascade in the camera view frustum with
    /// respect to the light, i.e. all objects that may cast visible shadows
    /// into each cascade, will be included in the clip space for that cascade.
    pub fn bound_orthographic_transforms_to_cascaded_view_frustum(
        &mut self,
        camera_space_view_frustum: &Frustum<fre>,
        camera_space_bounding_sphere: &Sphere<fre>,
    ) {
        // Rotate to light space, where the light direction is -z
        let light_space_view_frustum =
            camera_space_view_frustum.rotated(&self.camera_to_light_space_rotation);
        let light_space_bounding_sphere =
            camera_space_bounding_sphere.rotated(&self.camera_to_light_space_rotation);

        let bounding_sphere_aabb = light_space_bounding_sphere.compute_aabb();

        // For the near plane we use the point on the bounding sphere farthest
        // towards the light source, as models between the light and the view
        // frustum may cast shadows into the frustum
        let near = light_space_bounding_sphere.center().z + light_space_bounding_sphere.radius();

        for (partition_depth_limits, orthographic_transform) in
            (iter::once(&self.near_partition_depth).chain(self.partition_depths.iter()))
                .zip(
                    self.partition_depths
                        .iter()
                        .chain(iter::once(&self.far_partition_depth)),
                )
                .map(|(&lower, &upper)| UpperExclusiveBounds::new(lower, upper))
                .zip(self.orthographic_transforms.iter_mut())
        {
            // Use the bounds of the view frustum in light space along with the
            // bounding sphere to constrain limits for orthographic projection
            let light_space_view_frustum_aabb =
                light_space_view_frustum.compute_aabb_for_subfrustum(partition_depth_limits);

            // Constrain limits using either the view frustum or the bounding
            // volume, depending on which gives the snuggest fit
            let aabb_for_visible_models =
                bounding_sphere_aabb.union_with(&light_space_view_frustum_aabb);

            if let Some(aabb_for_visible_models) = aabb_for_visible_models {
                let visible_models_aabb_lower_corner = aabb_for_visible_models.lower_corner();
                let visible_models_aabb_upper_corner = aabb_for_visible_models.upper_corner();

                let left = visible_models_aabb_lower_corner.x;
                let right = visible_models_aabb_upper_corner.x;

                let bottom = visible_models_aabb_lower_corner.y;
                let top = visible_models_aabb_upper_corner.y;

                // We use lower corner here because smaller (more negative) z is
                // farther away
                let far = visible_models_aabb_lower_corner.z;

                orthographic_transform.set_planes(left, right, bottom, top, near, far);
            }
        }
    }

    /// Determines whether the object with the given camera space bounding
    /// sphere would be included in the clip space for the given cascade,
    /// meaning that it could potentially cast a visible shadow within the
    /// cascade.
    pub fn bounding_sphere_may_cast_visible_shadow_in_cascade(
        &self,
        cascade_idx: CascadeIdx,
        camera_space_bounding_sphere: &Sphere<fre>,
    ) -> bool {
        let light_space_bounding_sphere =
            camera_space_bounding_sphere.rotated(&self.camera_to_light_space_rotation);

        let orthographic_aabb = self.orthographic_transforms[cascade_idx as usize].compute_aabb();

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

impl OrthographicTranslationAndScaling {
    fn set_planes(&mut self, left: fre, right: fre, bottom: fre, top: fre, near: fre, far: fre) {
        (self.translation, self.scaling) =
            OrthographicTransform::compute_orthographic_translation_and_scaling(
                left, right, bottom, top, near, far,
            );
    }

    fn compute_aabb(&self) -> AxisAlignedBox<fre> {
        let (orthographic_center, orthographic_half_extents) =
            OrthographicTransform::compute_center_and_half_extents_from_translation_and_scaling(
                &self.translation,
                &self.scaling,
            );

        let orthographic_lower_corner = orthographic_center - orthographic_half_extents;
        let orthographic_upper_corner = orthographic_center + orthographic_half_extents;

        let orthographic_aabb =
            AxisAlignedBox::new(orthographic_lower_corner, orthographic_upper_corner);

        orthographic_aabb
    }
}
