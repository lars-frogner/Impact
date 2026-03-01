//! Light sources.

use crate::{
    SceneEntityFlags,
    graph::{ModelInstanceFlags, ModelInstanceNode, SceneGraph},
    model::ModelInstanceManager,
};
use impact_alloc::{Allocator, arena::ArenaPool};
use impact_camera::Camera;
use impact_containers::NoHashMap;
use impact_geometry::{
    AxisAlignedBox, AxisAlignedBoxC, Frustum, OrientedBox, Sphere,
    projection::{CubemapFace, CubemapFaces, PerspectiveTransform},
};
use impact_id::EntityID;
use impact_intersection::{IntersectionManager, bounding_volume::BoundingVolumeID};
use impact_light::{
    LightFlags, LightManager, ShadowableOmnidirectionalLight, shadow_map::CascadeIdx,
};
use impact_math::{
    angle::{Angle, Radians},
    bounds::UpperExclusiveBounds,
    consts::f32::FRAC_PI_2,
    point::Point3,
    random::splitmix,
    transform::{Isometry3, Projective3, Similarity3, Similarity3C},
};
use impact_model::{
    InstanceFeatureBufferRangeID, ModelInstanceID, transform::InstanceModelLightTransform,
};

#[derive(Debug, Clone)]
struct ShadowingModel {
    model_to_camera_transform: Similarity3C,
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
/// Make sure to call [`Self::buffer_model_instances_for_rendering`] before
/// calling this method, so that the ranges of model to cubemap face
/// transforms in the model instance buffers come after the initial range
/// containing model to camera transforms.
pub fn bound_omnidirectional_lights_and_buffer_shadow_casting_model_instances(
    light_manager: &mut LightManager,
    model_instance_manager: &mut ModelInstanceManager,
    intersection_manager: &IntersectionManager,
    scene_graph: &SceneGraph,
    camera: &Camera,
    camera_space_aabb_for_visible_models: &AxisAlignedBox,
    shadow_mapping_enabled: bool,
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

        omnidirectional_light
            .orient_cubemap_based_on_visible_models(camera_space_aabb_for_visible_models);

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
            determine_light_space_culling_frustum_transform(
                &camera_to_light_transform,
                camera_space_aabb_for_visible_models,
            )
        {
            let world_space_culling_frustum = create_world_space_frustum_for_culling(
                &light_space_culling_frustum_transform,
                &world_to_light_transform,
            );

            intersection_manager.for_each_bounding_volume_maybe_in_frustum(
                &world_space_culling_frustum,
                |id, aabb| {
                    register_primitive_in_omnidirectional_light_culling_frustum(
                        scene_graph,
                        world_to_camera_transform,
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

        if !shadow_mapping_enabled {
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

fn determine_light_space_culling_frustum_transform(
    camera_to_light_space_transform: &Isometry3,
    camera_space_aabb_for_visible_models: &AxisAlignedBox,
) -> Option<PerspectiveTransform> {
    let light_space_obb_for_visible_models =
        OrientedBox::from_axis_aligned_box(camera_space_aabb_for_visible_models)
            .iso_transformed(camera_to_light_space_transform);

    determine_perspective_transform_encompassing_box_in_negative_z_halfspace(
        &light_space_obb_for_visible_models,
    )
}

fn create_world_space_frustum_for_culling(
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
    world_space_light_position: &Point3,
    squared_max_reach: f32,
    shadowing_models: &mut NoHashMap<ModelInstanceID, ShadowingModel, A>,
    min_squared_dist: &mut f32,
    max_squared_dist: &mut f32,
    id: BoundingVolumeID,
    aabb: &AxisAlignedBoxC,
) {
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

/// Goes through all unidirectional lights in the given light manager and
/// updates their orthographic transforms to encompass model instances that
/// may cast visible shadows inside the corresponding cascades in the view
/// frustum. Then the model to light transform of every such shadow casting
/// model instance is computed for each light and copied to the model's
/// instance transform buffer in a new range dedicated to the particular
/// light and cascade.
///
/// # Warning
/// Make sure to call [`Self::buffer_model_instances_for_rendering`] before
/// calling this method, so that the ranges of model to light transforms in
/// the model instance buffers come after the initial range containing model
/// to camera transforms.
pub fn bound_unidirectional_lights_and_buffer_shadow_casting_model_instances(
    light_manager: &mut LightManager,
    model_instance_manager: &mut ModelInstanceManager,
    intersection_manager: &IntersectionManager,
    scene_graph: &SceneGraph,
    camera: &Camera,
    shadow_mapping_enabled: bool,
) {
    let world_space_view_frustum = camera.compute_world_space_view_frustum();
    let world_space_scene_aabb = intersection_manager.total_bounding_volume().aligned();

    let Some(world_space_scene_aabb_for_visible_models) = world_space_view_frustum
        .compute_aabb()
        .compute_overlap_with(&world_space_scene_aabb)
    else {
        return;
    };

    let camera_space_view_frustum = camera.projection().view_frustum();

    let world_to_camera_transform = camera.view_transform();
    let camera_to_world_transform = world_to_camera_transform.inverted();

    let world_space_camera_position = camera.compute_world_space_position();
    let camera_view_direction = camera.view_direction();

    for (light_id, unidirectional_light) in
        light_manager.shadowable_unidirectional_lights_with_ids_mut()
    {
        if unidirectional_light
            .flags()
            .contains(LightFlags::IS_DISABLED)
        {
            continue;
        }

        unidirectional_light.update_cascade_partition_depths(
            camera_space_view_frustum,
            &world_space_camera_position,
            &camera_view_direction,
            &world_space_scene_aabb_for_visible_models,
        );

        let cascade_may_have_models = unidirectional_light
            .bound_orthographic_transforms_to_cascaded_view_frustum(
                world_to_camera_transform,
                camera_space_view_frustum,
                &world_space_scene_aabb,
            );

        if !shadow_mapping_enabled {
            continue;
        }

        let light_to_world_transform =
            unidirectional_light.create_light_to_world_space_transform(&camera_to_world_transform);

        for cascade_idx in cascade_may_have_models
            .iter()
            .enumerate()
            .filter_map(|(idx, may_have_models)| may_have_models.then_some(idx as CascadeIdx))
        {
            // We will begin a new range dedicated for tranforms to the
            // current light's space for instances casting shadows in he
            // current cascade at the end of each transform buffer,
            // identified by the light's ID plus a cascade index offset
            let range_id = light_entity_id_to_instance_feature_buffer_range_id(
                light_id.as_entity_id(),
                u64::from(cascade_idx),
            );

            let light_space_orthographic_aabb =
                unidirectional_light.create_light_space_orthographic_aabb_for_cascade(cascade_idx);

            let world_space_orthographic_aabb = light_space_orthographic_aabb
                .aabb_of_transformed(&light_to_world_transform.to_matrix());

            intersection_manager.for_each_bounding_volume_in_axis_aligned_box(
                &world_space_orthographic_aabb,
                |id, _| {
                    let model_instance_id = ModelInstanceID::from_entity_id(id.as_entity_id());
                    let Some(model_instance_node) = scene_graph
                        .model_instance_nodes()
                        .get_node(model_instance_id)
                    else {
                        return;
                    };

                    if model_instance_node.flags().intersects(
                        ModelInstanceFlags::IS_HIDDEN | ModelInstanceFlags::CASTS_NO_SHADOWS,
                    ) || model_instance_node
                        .feature_ids_for_shadow_mapping()
                        .is_empty()
                    {
                        return;
                    }

                    let model_to_parent_transform =
                        model_instance_node.model_to_parent_transform().aligned();

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

                    let model_to_camera_transform =
                        world_to_camera_transform * model_to_world_transform;

                    let instance_model_light_transform = unidirectional_light
                        .create_transform_to_light_space(&model_to_camera_transform);

                    ensure_ranges_in_feature_buffers_for_model(
                        model_instance_manager,
                        range_id,
                        model_instance_node,
                    );

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
