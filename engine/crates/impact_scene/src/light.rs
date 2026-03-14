//! Light sources.

use crate::{
    SceneEntityFlags,
    graph::{ModelInstanceFlags, ModelInstanceNode, SceneGraph},
    model::ModelInstanceManager,
};
use impact_alloc::{AVec, Allocator, arena::ArenaPool};
use impact_camera::Camera;
use impact_containers::NoHashMap;
use impact_geometry::{
    AxisAlignedBox, AxisAlignedBoxC, Frustum, OrientedBox, Sphere,
    projection::{CubemapFace, CubemapFaces, PerspectiveTransform},
};
use impact_id::EntityID;
use impact_intersection::{IntersectionManager, bounding_volume::BoundingVolumeID};
use impact_light::{
    CascadePartitionDepths, LightFlags, LightManager, MAX_SHADOW_MAP_CASCADES_USIZE,
    ShadowableOmnidirectionalLight, ShadowableOmnidirectionalLightID,
    ShadowableUnidirectionalLight, ShadowableUnidirectionalLightID,
    shadow_map::{CascadeIdx, ShadowMappingConfig, UnidirectionalLightShadowMapBoundingMode},
};
use impact_math::{
    angle::{Angle, Radians},
    bounds::{Bounds, UpperExclusiveBounds},
    consts::f32::FRAC_PI_2,
    matrix::Matrix4,
    point::Point3,
    random::splitmix,
    transform::{Isometry3, Projective3, Similarity3, Similarity3C},
    vector::{UnitVector3, Vector3},
};
use impact_model::{
    InstanceFeatureBufferRangeID, ModelInstanceID, transform::InstanceModelLightTransform,
};

#[derive(Debug, Clone)]
struct ShadowingModel {
    model_to_camera_transform: Similarity3C,
}

#[derive(Clone, Debug)]
struct CascadeVolumes {
    light_space_culling_aabb: AxisAlignedBox,
    world_space_culling_obb: OrientedBox,
    light_space_bounding_sphere: Sphere,
}

/// Converts the given entity ID for a light along with an offset (for cascades
/// or cubemap faces) into an [`InstanceFeatureBufferRangeID`].
pub fn light_entity_id_to_instance_feature_buffer_range_id(
    entity_id: EntityID,
    offset: u64,
) -> InstanceFeatureBufferRangeID {
    splitmix::random_u64_from_two_states(entity_id.as_u64(), offset)
}

impl From<SceneEntityFlags> for LightFlags {
    fn from(scene_entity_flags: SceneEntityFlags) -> Self {
        let mut light_flags = Self::empty();
        if scene_entity_flags.contains(SceneEntityFlags::IS_DISABLED) {
            light_flags |= Self::IS_DISABLED;
        }
        light_flags
    }
}

/// Goes through all omnidirectional lights in the given light manager and
/// updates their cubemap orientations and distance spans to encompass all
/// model instances that may cast visible shadows. Then the model to cubemap
/// face space transform of every such shadow casting model instance is
/// computed for the relevant cube faces of each light and copied to the
/// model's instance transform buffer in new ranges dedicated to the faces
/// of the cubemap of the particular light.
///
/// # Warning
/// Make sure to call
/// [`buffer_model_instances_for_rendering`](crate::model::buffer_model_instances_for_rendering)
/// before calling this method, so that the ranges of model to cubemap face
/// transforms in the model instance buffers come after the initial range
/// containing model to camera transforms.
pub fn bound_omnidirectional_lights_and_buffer_shadow_casting_model_instances(
    light_manager: &mut LightManager,
    model_instance_manager: &mut ModelInstanceManager,
    intersection_manager: &IntersectionManager,
    scene_graph: &SceneGraph,
    camera: &Camera,
    world_space_aabb_for_visible_models: &AxisAlignedBox,
    shadow_mapping_config: &ShadowMappingConfig,
) {
    let arena = ArenaPool::get_arena();
    let mut shadowing_models =
        NoHashMap::with_capacity_and_hasher_in(0, Default::default(), &arena);

    let world_to_camera_transform = camera.view_transform();
    let camera_to_world_transform = world_to_camera_transform.inverted();

    for (light_id, omnidirectional_light) in
        light_manager.shadowable_omnidirectional_lights_with_ids_mut()
    {
        if omnidirectional_light
            .flags()
            .contains(LightFlags::IS_DISABLED)
        {
            continue;
        }

        omnidirectional_light.orient_light_space_based_on_visible_models(
            world_to_camera_transform,
            world_space_aabb_for_visible_models,
        );

        let camera_to_light_transform =
            omnidirectional_light.create_camera_to_light_space_transform();
        let world_to_light_transform = camera_to_light_transform * world_to_camera_transform;

        let camera_space_light_position = omnidirectional_light.camera_space_position().aligned();
        let world_space_light_position =
            camera_to_world_transform.transform_point(&camera_space_light_position);

        let squared_max_reach = omnidirectional_light.max_reach().powi(2);

        shadowing_models.clear();

        let mut min_squared_dist = f32::INFINITY;
        let mut max_squared_dist = f32::NEG_INFINITY;

        let cubemap_faces_to_include;

        // Light space is oriented so that the negative z-axis points towards
        // the center of the AABB for visible models. If all of the AABB is in
        // the negative z half-space in light space, the light is definitely
        // outside the AABB. In that case, we can create a frustum with apex at
        // the light position that encompasses the AABB and know that all models
        // casting shadows onto visible models will be contained in it. If part
        // of the AABB is not in the negative z half-space, we do not attempt to
        // create a culling frustum, but instead use the light's max reach to
        // create a sphere guaranteed to contain all shadow casting models.

        if let Some(light_space_culling_frustum_transform) =
            determine_light_space_culling_frustum_perspective_transform(
                &world_to_light_transform,
                world_space_aabb_for_visible_models,
            )
        {
            let world_space_culling_frustum =
                create_world_space_omnidirectional_light_culling_frustum(
                    &light_space_culling_frustum_transform,
                    &world_to_light_transform,
                );

            intersection_manager.for_each_bounding_volume_maybe_in_frustum(
                &world_space_culling_frustum,
                |id, aabb| {
                    register_primitive_in_omnidirectional_light_culling_frustum(
                        scene_graph,
                        world_to_camera_transform,
                        light_id,
                        &world_space_light_position,
                        squared_max_reach,
                        &mut shadowing_models,
                        &mut min_squared_dist,
                        &mut max_squared_dist,
                        id,
                        aabb,
                    );
                },
            );

            cubemap_faces_to_include = determine_cubemap_faces_included_in_culling_frustum(
                &light_space_culling_frustum_transform,
            );
        } else {
            let world_space_light_sphere = Sphere::new(
                world_space_light_position,
                omnidirectional_light.max_reach(),
            );

            intersection_manager.for_each_bounding_volume_in_sphere(
                &world_space_light_sphere,
                |id, aabb| {
                    register_primitive_in_omnidirectional_light_culling_frustum(
                        scene_graph,
                        world_to_camera_transform,
                        light_id,
                        &world_space_light_position,
                        squared_max_reach,
                        &mut shadowing_models,
                        &mut min_squared_dist,
                        &mut max_squared_dist,
                        id,
                        aabb,
                    );
                },
            );

            cubemap_faces_to_include = CubemapFaces::all();
        }

        if shadowing_models.is_empty() {
            // We have no models to bound the near and far distance, so we just
            // use the outer bounds
            omnidirectional_light
                .update_near_and_far_distance(0.0, omnidirectional_light.max_reach());
            continue;
        }

        let near_distance = min_squared_dist.sqrt();
        let far_distance = max_squared_dist.sqrt();
        omnidirectional_light.update_near_and_far_distance(near_distance, far_distance);

        if !shadow_mapping_config.enabled {
            // Even with disabled shadow mapping we had to get the appropriate
            // near and far distances, but we can skip the buffering
            continue;
        }

        for face in CubemapFace::all() {
            if !cubemap_faces_to_include.contains(face.into()) {
                continue;
            }

            // We will begin a new range dedicated for tranforms to the
            // current cubemap face space for the current light at the end
            // of each transform buffer, identified by the light's ID plus a
            // face index offset
            let range_id = light_entity_id_to_instance_feature_buffer_range_id(
                light_id.as_entity_id(),
                face.as_idx_u64(),
            );

            let world_space_face_frustum = omnidirectional_light
                .compute_world_space_frustum_for_face(face, world_to_camera_transform);

            let camera_to_cubemap_face_space_transform = omnidirectional_light
                .create_transform_from_camera_space_to_positive_z_cubemap_face_space(face);

            intersection_manager.for_each_bounding_volume_maybe_in_frustum(
                &world_space_face_frustum,
                |id, _| {
                    let model_instance_id = ModelInstanceID::from_entity_id(id.as_entity_id());

                    // Only include models deemed as shadowing
                    let Some(model) = shadowing_models.get(&model_instance_id) else {
                        return;
                    };

                    let model_instance_node =
                        scene_graph.model_instance_nodes().node(model_instance_id);

                    ensure_ranges_in_feature_buffers_for_model(
                        model_instance_manager,
                        range_id,
                        model_instance_node,
                    );

                    let instance_model_light_transform = camera_to_cubemap_face_space_transform
                        * model.model_to_camera_transform.aligned();

                    buffer_features_for_model(
                        model_instance_manager,
                        model_instance_node,
                        &instance_model_light_transform,
                    );
                },
            );
        }
    }
}

fn determine_light_space_culling_frustum_perspective_transform(
    world_to_light_space_transform: &Isometry3,
    world_space_aabb_for_visible_models: &AxisAlignedBox,
) -> Option<PerspectiveTransform> {
    let light_space_obb_for_visible_models =
        OrientedBox::from_axis_aligned_box(world_space_aabb_for_visible_models)
            .iso_transformed(world_to_light_space_transform);

    determine_perspective_transform_encompassing_box_in_negative_z_halfspace(
        &light_space_obb_for_visible_models,
    )
}

fn create_world_space_omnidirectional_light_culling_frustum(
    light_space_culling_frustum_transform: &PerspectiveTransform,
    world_to_light_transform: &Isometry3,
) -> Frustum {
    let world_space_perspective_transform_for_culling_frustum = Projective3::from_matrix_unchecked(
        light_space_culling_frustum_transform
            .as_projective()
            .matrix()
            * world_to_light_transform.to_matrix(),
    );

    Frustum::from_transform(&world_space_perspective_transform_for_culling_frustum)
}

fn determine_cubemap_faces_included_in_culling_frustum(
    light_space_culling_frustum_transform: &PerspectiveTransform,
) -> CubemapFaces {
    // The culling frustum points along the negative z-axis in light space, so
    // the negative z face is always included. If the vertical/horizontal field
    // of view is sufficiently large, the vertical/horizontal faces are also
    // included.

    let mut faces = CubemapFaces::NEGATIVE_Z;

    if light_space_culling_frustum_transform
        .horizontal_field_of_view()
        .radians()
        >= FRAC_PI_2
    {
        faces |= CubemapFaces::POSITIVE_X | CubemapFaces::NEGATIVE_X;
    }

    if light_space_culling_frustum_transform
        .vertical_field_of_view()
        .radians()
        >= FRAC_PI_2
    {
        faces |= CubemapFaces::POSITIVE_Y | CubemapFaces::NEGATIVE_Y;
    }

    faces
}

fn determine_perspective_transform_encompassing_box_in_negative_z_halfspace(
    oriented_box: &OrientedBox,
) -> Option<PerspectiveTransform> {
    const MIN_DEPTH: f32 = 1e-4;
    const NEAR_DISTANCE: f32 = ShadowableOmnidirectionalLight::MIN_NEAR_DISTANCE;
    const MIN_FAR_DISTANCE: f32 = NEAR_DISTANCE + ShadowableOmnidirectionalLight::MIN_SPAN;
    const MIN_PROJECTED_DIST: f32 = 0.1;

    let mut max_depth = f32::NEG_INFINITY;
    let mut max_width_per_depth = f32::NEG_INFINITY;
    let mut max_height_per_depth = f32::NEG_INFINITY;

    for corner in oriented_box.compute_corners() {
        let depth = -corner.z();
        if depth < MIN_DEPTH {
            return None;
        }

        let width_per_depth = corner.x().abs() / depth;
        let height_per_depth = corner.y().abs() / depth;

        max_depth = max_depth.max(depth);
        max_width_per_depth = max_width_per_depth.max(width_per_depth);
        max_height_per_depth = max_height_per_depth.max(height_per_depth);
    }

    let far_distance = max_depth.max(MIN_FAR_DISTANCE);
    let max_width_per_depth = max_width_per_depth.max(MIN_PROJECTED_DIST);
    let max_height_per_depth = max_height_per_depth.max(MIN_PROJECTED_DIST);

    let aspect_ratio = max_width_per_depth / max_height_per_depth;

    let vertical_fov = 2.0 * f32::atan(max_height_per_depth);

    Some(PerspectiveTransform::new(
        aspect_ratio,
        Radians(vertical_fov),
        UpperExclusiveBounds::new(NEAR_DISTANCE, far_distance),
    ))
}

fn register_primitive_in_omnidirectional_light_culling_frustum<A: Allocator>(
    scene_graph: &SceneGraph,
    world_to_camera_transform: &Isometry3,
    light_id: ShadowableOmnidirectionalLightID,
    world_space_light_position: &Point3,
    squared_max_reach: f32,
    shadowing_models: &mut NoHashMap<ModelInstanceID, ShadowingModel, A>,
    min_squared_dist: &mut f32,
    max_squared_dist: &mut f32,
    id: BoundingVolumeID,
    aabb: &AxisAlignedBoxC,
) {
    if id.as_entity_id() == light_id.as_entity_id() {
        // Ignore self-shadowing
        return;
    }

    let model_instance_id = ModelInstanceID::from_entity_id(id.as_entity_id());
    let Some(model_instance_node) = scene_graph
        .model_instance_nodes()
        .get_node(model_instance_id)
    else {
        return;
    };

    if model_instance_node.flags().intersects(
        ModelInstanceFlags::IS_HIDDEN
            | ModelInstanceFlags::CASTS_NO_SHADOWS
            | ModelInstanceFlags::EXCEEDS_DIST_FOR_DISABLING_SHADOWING,
    ) || model_instance_node
        .feature_ids_for_shadow_mapping()
        .is_empty()
    {
        return;
    }

    let aabb = aabb.aligned();

    let closest_point = aabb.closest_interior_point_to(world_space_light_position);
    let squared_closest_point_dist =
        Point3::squared_distance_between(world_space_light_position, &closest_point);

    if squared_closest_point_dist > squared_max_reach {
        // The light doesn't reach the model
        return;
    }

    *min_squared_dist = min_squared_dist.min(squared_closest_point_dist);

    let farthest_point = aabb.farthest_corner_from(world_space_light_position);
    let squared_farthest_point_dist =
        Point3::squared_distance_between(world_space_light_position, &farthest_point);

    *max_squared_dist = max_squared_dist.max(squared_farthest_point_dist);

    let model_to_camera_transform = compute_model_to_camera_transform(
        scene_graph,
        world_to_camera_transform,
        model_instance_node,
    );

    shadowing_models.insert(
        model_instance_id,
        ShadowingModel {
            model_to_camera_transform: model_to_camera_transform.compact(),
        },
    );
}

/// Goes through all unidirectional lights in the light manager and updates
/// their orthographic transforms to encompass model instances that may cast
/// visible shadows inside the corresponding cascades in the view frustum. Then
/// the model to light transform of every such shadow casting model instance is
/// computed for each light and copied to the model's instance transform buffer
/// in a new range dedicated to the particular light and cascade.
///
/// # Warning
/// Make sure to call
/// [`buffer_model_instances_for_rendering`](crate::model::buffer_model_instances_for_rendering)
/// before calling this method, so that the ranges of model to light transforms
/// in the model instance buffers come after the initial range containing model
/// to camera transforms.
pub fn bound_unidirectional_lights_and_buffer_shadow_casting_model_instances(
    light_manager: &mut LightManager,
    model_instance_manager: &mut ModelInstanceManager,
    intersection_manager: &IntersectionManager,
    scene_graph: &SceneGraph,
    camera: &Camera,
    camera_space_aabb_for_visible_models: &AxisAlignedBox,
    shadow_mapping_config: &ShadowMappingConfig,
) {
    if light_manager
        .shadowable_unidirectional_light_buffer()
        .n_valid_uniforms()
        == 0
    {
        return;
    }

    let arena = ArenaPool::get_arena();
    let mut shadowing_model_ids = AVec::new_in(&arena);

    let world_to_camera_transform = camera.view_transform();
    let camera_to_world_transform = world_to_camera_transform.inverted();

    let camera_space_view_frustum = camera.projection().view_frustum();

    let partition_depths = match shadow_mapping_config.unidirectional_light_shadow_map_bounding_mode
    {
        UnidirectionalLightShadowMapBoundingMode::Tight => CascadePartitionDepths::compute_dynamic(
            camera_space_view_frustum,
            camera_space_aabb_for_visible_models,
        ),
        UnidirectionalLightShadowMapBoundingMode::Stable => {
            CascadePartitionDepths::compute_stable(camera_space_view_frustum)
        }
    };

    let world_space_scene_aabb = intersection_manager.total_bounding_volume().aligned();

    for (light_id, unidirectional_light) in
        light_manager.shadowable_unidirectional_lights_with_ids_mut()
    {
        if unidirectional_light
            .flags()
            .contains(LightFlags::IS_DISABLED)
        {
            continue;
        }

        unidirectional_light.set_cascade_partition_depths(&partition_depths);

        let world_to_light_transform =
            unidirectional_light.create_world_to_light_space_transform(world_to_camera_transform);

        let world_to_light_transform_matrix = world_to_light_transform.to_matrix();

        for (
            cascade_idx,
            CascadeVolumes {
                light_space_culling_aabb,
                world_space_culling_obb,
                light_space_bounding_sphere,
            },
        ) in create_unidirectional_light_cascade_volumes(
            camera,
            &camera_to_world_transform,
            &world_space_scene_aabb,
            unidirectional_light,
        )
        .into_iter()
        .enumerate()
        {
            let cascade_idx = cascade_idx as CascadeIdx;

            shadowing_model_ids.clear();

            let mut light_space_aabb_for_shadowing_models =
                AxisAlignedBox::new(Point3::same(f32::INFINITY), Point3::same(f32::NEG_INFINITY));

            intersection_manager.for_each_bounding_volume_maybe_in_oriented_box(
                &world_space_culling_obb,
                |id, aabb| {
                    register_primitive_in_unidirectional_light_culling_box(
                        scene_graph,
                        &world_to_light_transform_matrix,
                        light_id,
                        &mut shadowing_model_ids,
                        &mut light_space_aabb_for_shadowing_models,
                        id,
                        aabb,
                    );
                },
            );

            let tight_light_space_orthographic_aabb = if shadowing_model_ids.is_empty() {
                // We have no models to bound the cascade, so we just use the
                // original box
                light_space_culling_aabb
            } else {
                // We allow the orthographic AABB to shrink relative to the culling
                // box if the AABB for shadowing models is smaller
                light_space_aabb_for_shadowing_models
                    .compute_overlap_with(&light_space_culling_aabb)
                    .unwrap_or(light_space_culling_aabb)
            };

            let light_space_orthographic_aabb =
                match shadow_mapping_config.unidirectional_light_shadow_map_bounding_mode {
                    UnidirectionalLightShadowMapBoundingMode::Tight => {
                        tight_light_space_orthographic_aabb
                    }
                    UnidirectionalLightShadowMapBoundingMode::Stable => {
                        let mut sphere_aabb = light_space_bounding_sphere.compute_aabb();

                        // Keeping the orthographic transforms stable only requires
                        // fixing the x- and y-bounds to the bounding sphere. The
                        // z-bounds should match the tight AABB, both to make sure
                        // all casters are included and to avoid wasting depth
                        // resolution.
                        *sphere_aabb.lower_corner_mut().z_mut() =
                            tight_light_space_orthographic_aabb.lower_corner().z();
                        *sphere_aabb.upper_corner_mut().z_mut() =
                            tight_light_space_orthographic_aabb.upper_corner().z();

                        snap_light_space_orthographic_aabb_extent_to_texels(
                            &world_to_light_transform,
                            &sphere_aabb,
                            shadow_mapping_config,
                        )
                    }
                };

            unidirectional_light.set_light_space_orthographic_aabb_for_cascade(
                cascade_idx,
                &light_space_orthographic_aabb,
            );

            if !shadow_mapping_config.enabled || shadowing_model_ids.is_empty() {
                continue;
            }

            // We will begin a new range dedicated for tranforms to the current
            // light's space for instances casting shadows in he current cascade
            // at the end of each transform buffer, identified by the light's ID
            // and a cascade index offset
            let range_id = light_entity_id_to_instance_feature_buffer_range_id(
                light_id.as_entity_id(),
                u64::from(cascade_idx),
            );

            for &model_instance_id in &shadowing_model_ids {
                let model_instance_node =
                    scene_graph.model_instance_nodes().node(model_instance_id);

                ensure_ranges_in_feature_buffers_for_model(
                    model_instance_manager,
                    range_id,
                    model_instance_node,
                );

                let model_to_camera_transform = compute_model_to_camera_transform(
                    scene_graph,
                    world_to_camera_transform,
                    model_instance_node,
                );

                let instance_model_light_transform = unidirectional_light
                    .create_model_to_light_space_transform(&model_to_camera_transform);

                buffer_features_for_model(
                    model_instance_manager,
                    model_instance_node,
                    &instance_model_light_transform,
                );
            }
        }
    }
}

fn create_unidirectional_light_cascade_volumes(
    camera: &Camera,
    camera_to_world_transform: &Isometry3,
    world_space_scene_aabb: &AxisAlignedBox,
    unidirectional_light: &ShadowableUnidirectionalLight,
) -> [CascadeVolumes; MAX_SHADOW_MAP_CASCADES_USIZE] {
    let camera_to_light_space_rotation = unidirectional_light
        .camera_to_light_space_rotation()
        .aligned();

    let light_to_world_space_transform =
        camera_to_world_transform.applied_to_rotation(&camera_to_light_space_rotation.inverse());

    let world_space_light_direction =
        light_to_world_space_transform.transform_unit_vector(&UnitVector3::neg_unit_z());

    let world_space_camera_displacement_along_light_direction = camera_to_world_transform
        .translation()
        .dot(world_space_light_direction.as_vector());

    let world_space_min_scene_displacement_along_light_direction = world_space_scene_aabb
        .displacement_range_along_axis(&world_space_light_direction)
        .0;

    // For the near plane we use the point on the scene bounding box
    // farthest towards the light source, as models between the light and
    // the view frustum may cast shadows into the frustum
    let near_coord = world_space_camera_displacement_along_light_direction
        - world_space_min_scene_displacement_along_light_direction; // Negative sign because light direction is -z in light space

    compute_light_space_cascade_aabbs_and_bounding_spheres(camera, unidirectional_light).map(
        move |(mut light_space_culling_aabb, light_space_bounding_sphere)| {
            // The light points along -z in light space, so to include the full
            // scene against the light direction we expand the upper z-coordinate of
            // the box
            *light_space_culling_aabb.upper_corner_mut().z_mut() =
                light_space_culling_aabb.upper_corner().z().max(near_coord);

            let world_space_culling_obb =
                OrientedBox::from_axis_aligned_box(&light_space_culling_aabb)
                    .iso_transformed(&light_to_world_space_transform);

            CascadeVolumes {
                light_space_culling_aabb,
                world_space_culling_obb,
                light_space_bounding_sphere,
            }
        },
    )
}

fn compute_light_space_cascade_aabbs_and_bounding_spheres(
    camera: &Camera,
    unidirectional_light: &ShadowableUnidirectionalLight,
) -> [(AxisAlignedBox, Sphere); MAX_SHADOW_MAP_CASCADES_USIZE] {
    let camera_projection = camera.projection();

    let camera_space_view_frustum = camera_projection.view_frustum();
    let camera_space_view_frustum_corners = camera_space_view_frustum.compute_corners();

    let view_frustum_near_distance = camera_projection.near_distance();
    let view_frustum_far_distance = camera_projection.far_distance();
    let view_frustum_distance_span = view_frustum_far_distance - view_frustum_near_distance;

    let camera_to_light_space_rotation = unidirectional_light
        .camera_to_light_space_rotation()
        .aligned();

    let light_space_view_frustum_corners = camera_space_view_frustum_corners
        .map(|corner| camera_to_light_space_rotation.rotate_point(&corner));

    unidirectional_light
        .partition_depth_limits_for_each_cascade()
        .map(move |partition_depth_limits| {
            let cascade_subfrustum_corners = Frustum::compute_corners_of_subfrustum(
                &light_space_view_frustum_corners,
                view_frustum_near_distance,
                view_frustum_far_distance,
                partition_depth_limits,
            );

            let light_space_cascade_subfrustum_aabb =
                AxisAlignedBox::aabb_for_point_array(&cascade_subfrustum_corners);

            let subfrustum_near_distance = view_frustum_near_distance
                + partition_depth_limits.lower() * view_frustum_distance_span;

            let subfrustum_far_distance = view_frustum_near_distance
                + partition_depth_limits.upper() * view_frustum_distance_span;

            let camera_space_cascade_subfrustum_bounding_sphere = camera_projection
                .subfrustum_bounding_sphere(subfrustum_near_distance, subfrustum_far_distance);

            let light_space_cascade_subfrustum_bounding_sphere =
                camera_space_cascade_subfrustum_bounding_sphere
                    .rotated(&camera_to_light_space_rotation);

            (
                light_space_cascade_subfrustum_aabb,
                light_space_cascade_subfrustum_bounding_sphere,
            )
        })
}

fn register_primitive_in_unidirectional_light_culling_box<A: Allocator>(
    scene_graph: &SceneGraph,
    world_to_light_transform_matrix: &Matrix4,
    light_id: ShadowableUnidirectionalLightID,
    shadowing_model_ids: &mut AVec<ModelInstanceID, A>,
    light_space_aabb_for_shadowing_models: &mut AxisAlignedBox,
    id: BoundingVolumeID,
    aabb: &AxisAlignedBoxC,
) {
    if id.as_entity_id() == light_id.as_entity_id() {
        // Ignore self-shadowing
        return;
    }

    let model_instance_id = ModelInstanceID::from_entity_id(id.as_entity_id());
    let Some(model_instance_node) = scene_graph
        .model_instance_nodes()
        .get_node(model_instance_id)
    else {
        return;
    };

    if model_instance_node.flags().intersects(
        ModelInstanceFlags::IS_HIDDEN
            | ModelInstanceFlags::CASTS_NO_SHADOWS
            | ModelInstanceFlags::EXCEEDS_DIST_FOR_DISABLING_SHADOWING,
    ) || model_instance_node
        .feature_ids_for_shadow_mapping()
        .is_empty()
    {
        return;
    }

    let light_space_aabb = aabb
        .aligned()
        .aabb_of_transformed(world_to_light_transform_matrix);

    light_space_aabb_for_shadowing_models.merge_with(&light_space_aabb);

    shadowing_model_ids.push(model_instance_id);
}

fn snap_light_space_orthographic_aabb_extent_to_texels(
    world_to_light_transform: &Isometry3,
    aabb: &AxisAlignedBox,
    shadow_mapping_config: &ShadowMappingConfig,
) -> AxisAlignedBox {
    let n_texels = shadow_mapping_config.unidirectional_light_shadow_map_resolution as f32;

    let light_space_world_origin = world_to_light_transform.transform_point(&Point3::origin());

    let lower_offset = aabb.lower_corner() - light_space_world_origin;
    let upper_offset = aabb.upper_corner() - light_space_world_origin;

    let x_min = lower_offset.x();
    let x_max = upper_offset.x();
    let y_min = lower_offset.y();
    let y_max = upper_offset.y();

    let extent_x = x_max - x_min;
    let extent_y = y_max - y_min;

    let texel_size_x = extent_x / n_texels;
    let texel_size_y = extent_y / n_texels;

    let snapped_x_min = (x_min / texel_size_x).floor() * texel_size_x;
    let snapped_y_min = (y_min / texel_size_y).floor() * texel_size_y;

    let snapped_x_max = snapped_x_min + extent_x;
    let snapped_y_max = snapped_y_min + extent_y;

    let snapped_lower_offset = Vector3::new(snapped_x_min, snapped_y_min, lower_offset.z());
    let snapped_upper_offset = Vector3::new(snapped_x_max, snapped_y_max, upper_offset.z());

    AxisAlignedBox::new(
        light_space_world_origin + snapped_lower_offset,
        light_space_world_origin + snapped_upper_offset,
    )
}

fn compute_model_to_camera_transform(
    scene_graph: &SceneGraph,
    world_to_camera_transform: &Isometry3,
    model_instance_node: &ModelInstanceNode,
) -> Similarity3 {
    let model_to_parent_transform = model_instance_node.model_to_parent_transform().aligned();

    let model_to_world_transform =
        if model_instance_node.parent_group_id() == scene_graph.root_node_id() {
            model_to_parent_transform
        } else {
            let parent_to_world_transform = scene_graph
                .group_nodes()
                .node(model_instance_node.parent_group_id())
                .group_to_root_transform()
                .aligned();

            parent_to_world_transform * model_to_parent_transform
        };

    world_to_camera_transform * model_to_world_transform
}

fn ensure_ranges_in_feature_buffers_for_model(
    model_instance_manager: &mut ModelInstanceManager,
    range_id: u64,
    model_instance_node: &ModelInstanceNode,
) {
    let feature_type_ids_for_shadow_mapping = model_instance_node
        .feature_ids_for_shadow_mapping()
        .iter()
        .map(|feature_id| feature_id.feature_type_id());

    model_instance_manager.ensure_ranges_in_feature_buffers_for_model(
        model_instance_node.model_id(),
        feature_type_ids_for_shadow_mapping,
        range_id,
    );
}

fn buffer_features_for_model(
    model_instance_manager: &mut ModelInstanceManager,
    model_instance_node: &ModelInstanceNode,
    instance_model_light_transform: &Similarity3,
) {
    let instance_model_light_transform =
        InstanceModelLightTransform::from(instance_model_light_transform);

    model_instance_manager.buffer_instance_feature(
        model_instance_node.model_id(),
        &instance_model_light_transform,
    );

    let feature_ids_for_shadow_mapping = model_instance_node.feature_ids_for_shadow_mapping();

    if feature_ids_for_shadow_mapping.len() > 1 {
        model_instance_manager.buffer_instance_features_from_storages(
            model_instance_node.model_id(),
            &feature_ids_for_shadow_mapping[1..],
        );
    }
}
