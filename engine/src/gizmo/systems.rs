//! ECS systems for gizmo management.

use crate::gizmo::{
    GizmoManager, GizmoParameters, GizmoSet, GizmoType, GizmoVisibility,
    components::GizmosComp,
    model::{
        COLLIDER_GIZMO_PLANE_MODEL_IDX, COLLIDER_GIZMO_SPHERE_MODEL_IDX,
        COLLIDER_GIZMO_VOXEL_SPHERE_MODEL_IDX, SHADOW_CUBEMAP_FACES_GIZMO_OUTLINES_MODEL_IDX,
        SHADOW_CUBEMAP_FACES_GIZMO_PLANES_MODEL_IDX,
        VOXEL_CHUNKS_GIZMO_NON_OBSCURABLE_NON_UNIFORM_MODEL_IDX,
        VOXEL_CHUNKS_GIZMO_NON_OBSCURABLE_UNIFORM_MODEL_IDX,
        VOXEL_CHUNKS_GIZMO_OBSCURABLE_NON_UNIFORM_MODEL_IDX,
        VOXEL_CHUNKS_GIZMO_OBSCURABLE_UNIFORM_MODEL_IDX,
    },
};
use approx::abs_diff_ne;
use impact_camera::gpu_resource::BufferableCamera;
use impact_ecs::{query, world::World as ECSWorld};
use impact_geometry::{ModelTransform, ReferenceFrame};
use impact_light::{
    LightManager, OmnidirectionalLightID, ShadowableOmnidirectionalLightID,
    ShadowableUnidirectionalLightID,
};
use impact_math::Angle;
use impact_model::transform::{InstanceModelViewTransform, InstanceModelViewTransformWithPrevious};
use impact_physics::{
    anchor::AnchorManager,
    collision::{CollidableID, CollidableKind},
    quantities::Motion,
    rigid_body::{DynamicRigidBodyID, KinematicRigidBodyID, RigidBodyManager, TypedRigidBodyID},
};
use impact_scene::{
    SceneEntityFlags, SceneGraphModelInstanceNodeHandle, SceneGraphParentNodeHandle,
    camera::SceneCamera,
    graph::{ModelInstanceNode, ModelInstanceNodeID, SceneGraph},
    model::ModelInstanceManager,
};
use impact_voxel::{
    VoxelObjectID, VoxelObjectManager,
    chunks::{CHUNK_SIZE, ChunkedVoxelObject, VoxelChunk},
    collidable::{Collidable, CollisionWorld},
};
use nalgebra::{
    Isometry3, Point3, Similarity3, Translation3, UnitQuaternion, UnitVector3, Vector3, vector,
};
use parking_lot::RwLock;
use std::iter;
use tinyvec::TinyVec;

/// Updates the appropriate gizmo visibility flags for all applicable
/// entities based on which gizmos have been newly configured to be
/// globally visible or hidden.
pub fn update_visibility_flags_for_gizmos(
    gizmo_manager: &mut GizmoManager,
    ecs_world: &RwLock<ECSWorld>,
) {
    if !gizmo_manager.global_visibility_changed_for_any_of_gizmos(GizmoSet::all()) {
        return;
    }

    let ecs_world = ecs_world.read();

    for gizmo in GizmoType::all() {
        if gizmo_manager.global_visibility_changed_for_any_of_gizmos(gizmo.as_set()) {
            update_visibility_flags_for_gizmo(&ecs_world, gizmo_manager, gizmo);
        }
    }
    gizmo_manager.declare_visibilities_synchronized();
}

fn update_visibility_flags_for_gizmo(
    ecs_world: &ECSWorld,
    gizmo_manager: &GizmoManager,
    gizmo: GizmoType,
) {
    let globally_visible = match gizmo_manager.visibilities().get_for(gizmo) {
        GizmoVisibility::Hidden => false,
        GizmoVisibility::VisibleForAll => true,
        GizmoVisibility::VisibleForSelected => {
            return;
        }
    };
    query!(ecs_world, |gizmos: &mut GizmosComp| {
        gizmos.visible_gizmos.set(gizmo.as_set(), globally_visible);
    });
}

/// Finds entities for which each gizmo should be displayed and copies
/// appropriately transformed versions of their model-view transforms to the
/// gizmo's dedicated buffer.
pub fn buffer_transforms_for_gizmos(
    ecs_world: &ECSWorld,
    rigid_body_manager: &RigidBodyManager,
    anchor_manager: &AnchorManager,
    model_instance_manager: &mut ModelInstanceManager,
    gizmo_manager: &GizmoManager,
    collision_world: &CollisionWorld,
    voxel_object_manager: &VoxelObjectManager,
    scene_graph: &SceneGraph,
    light_manager: &LightManager,
    scene_camera: Option<&SceneCamera>,
    current_frame_count: u32,
) {
    let Some(scene_camera) = scene_camera else {
        return;
    };
    let camera_position = scene_camera.compute_world_space_position();

    query!(
        ecs_world,
        |gizmos: &GizmosComp,
         model_instance_node: &SceneGraphModelInstanceNodeHandle,
         flags: &SceneEntityFlags| {
            if !gizmos
                .visible_gizmos
                .intersects(GizmoSet::REFERENCE_FRAME_AXES.union(GizmoSet::BOUNDING_SPHERE))
                || flags.is_disabled()
            {
                return;
            }
            buffer_transforms_for_model_instance_gizmos(
                model_instance_manager,
                scene_graph,
                current_frame_count,
                gizmos.visible_gizmos,
                model_instance_node.id,
            );
        }
    );

    query!(
        ecs_world,
        |gizmos: &GizmosComp,
         omnidirectional_light_id: &OmnidirectionalLightID,
         flags: &SceneEntityFlags| {
            if !gizmos.visible_gizmos.contains(GizmoSet::LIGHT_SPHERE) || flags.is_disabled() {
                return;
            }
            buffer_transform_for_light_sphere_gizmo(
                model_instance_manager,
                light_manager,
                *omnidirectional_light_id,
            );
        }
    );

    query!(
        ecs_world,
        |gizmos: &GizmosComp,
         omnidirectional_light_id: &ShadowableOmnidirectionalLightID,
         flags: &SceneEntityFlags| {
            if flags.is_disabled() {
                return;
            }
            if gizmos.visible_gizmos.contains(GizmoSet::LIGHT_SPHERE) {
                buffer_transform_for_shadowable_light_sphere_gizmo(
                    model_instance_manager,
                    light_manager,
                    *omnidirectional_light_id,
                );
            }
            if gizmos
                .visible_gizmos
                .contains(GizmoSet::SHADOW_CUBEMAP_FACES)
            {
                buffer_transforms_for_shadow_cubemap_faces_gizmo(
                    model_instance_manager,
                    light_manager,
                    *omnidirectional_light_id,
                );
            }
        }
    );

    query!(
        ecs_world,
        |gizmos: &GizmosComp,
         unidirectional_light_id: &ShadowableUnidirectionalLightID,
         flags: &SceneEntityFlags| {
            if !gizmos
                .visible_gizmos
                .contains(GizmoSet::SHADOW_MAP_CASCADES)
                || flags.is_disabled()
            {
                return;
            }
            buffer_transforms_for_shadow_map_cascades_gizmo(
                model_instance_manager,
                light_manager,
                scene_camera,
                *unidirectional_light_id,
            );
        }
    );

    query!(ecs_world, |gizmos: &GizmosComp,
                       frame: &ReferenceFrame,
                       rigid_body_id: &DynamicRigidBodyID,
                       flags: &SceneEntityFlags| {
        if !gizmos.visible_gizmos.contains(GizmoSet::ANCHORS) || flags.is_disabled() {
            return;
        }
        buffer_transforms_for_anchor_gizmos(
            anchor_manager,
            model_instance_manager,
            scene_camera,
            frame,
            TypedRigidBodyID::Dynamic(*rigid_body_id),
        );
    });

    query!(ecs_world, |gizmos: &GizmosComp,
                       frame: &ReferenceFrame,
                       rigid_body_id: &KinematicRigidBodyID,
                       flags: &SceneEntityFlags| {
        if !gizmos.visible_gizmos.contains(GizmoSet::ANCHORS) || flags.is_disabled() {
            return;
        }
        buffer_transforms_for_anchor_gizmos(
            anchor_manager,
            model_instance_manager,
            scene_camera,
            frame,
            TypedRigidBodyID::Kinematic(*rigid_body_id),
        );
    });

    query!(ecs_world, |gizmos: &GizmosComp,
                       frame: &ReferenceFrame,
                       motion: &Motion,
                       flags: &SceneEntityFlags| {
        if !gizmos
            .visible_gizmos
            .intersects(GizmoSet::LINEAR_VELOCITY.union(GizmoSet::ANGULAR_VELOCITY))
            || flags.is_disabled()
        {
            return;
        }
        buffer_transforms_for_kinematics_gizmos(
            model_instance_manager,
            gizmo_manager.parameters(),
            scene_camera,
            &camera_position,
            frame,
            motion,
            gizmos.visible_gizmos,
        );
    });

    query!(ecs_world, |gizmos: &GizmosComp,
                       frame: &ReferenceFrame,
                       rigid_body_id: &DynamicRigidBodyID,
                       flags: &SceneEntityFlags| {
        if !gizmos.visible_gizmos.intersects(
            GizmoSet::CENTER_OF_MASS
                .union(GizmoSet::ANGULAR_MOMENTUM)
                .union(GizmoSet::FORCE)
                .union(GizmoSet::TORQUE),
        ) || flags.is_disabled()
        {
            return;
        }
        buffer_transforms_for_dynamics_gizmos(
            rigid_body_manager,
            model_instance_manager,
            gizmo_manager.parameters(),
            scene_camera,
            &camera_position,
            frame,
            *rigid_body_id,
            gizmos.visible_gizmos,
        );
    });

    query!(ecs_world, |gizmos: &GizmosComp,
                       collidable: &CollidableID,
                       flags: &SceneEntityFlags| {
        if !gizmos.visible_gizmos.intersects(
            GizmoSet::DYNAMIC_COLLIDER
                .union(GizmoSet::STATIC_COLLIDER)
                .union(GizmoSet::PHANTOM_COLLIDER),
        ) || flags.is_disabled()
        {
            return;
        }
        buffer_transforms_for_collider_gizmos(
            model_instance_manager,
            collision_world,
            voxel_object_manager,
            scene_camera,
            &camera_position,
            *collidable,
            gizmos.visible_gizmos,
        );
    });

    query!(
        ecs_world,
        |gizmos: &GizmosComp,
         voxel_object_id: &VoxelObjectID,
         model_instance_node: &SceneGraphModelInstanceNodeHandle,
         flags: &SceneEntityFlags| {
            if !gizmos.visible_gizmos.contains(GizmoSet::VOXEL_CHUNKS) || flags.is_disabled() {
                return;
            }
            buffer_transforms_for_voxel_chunks_gizmo(
                model_instance_manager,
                voxel_object_manager,
                scene_graph,
                gizmo_manager.parameters(),
                current_frame_count,
                model_instance_node.id,
                *voxel_object_id,
            );
        }
    );

    let mut voxel_objects = Vec::new();

    query!(
        ecs_world,
        |gizmos: &GizmosComp,
         frame: &ReferenceFrame,
         model_transform: &ModelTransform,
         voxel_object_id: &VoxelObjectID,
         flags: &SceneEntityFlags| {
            if !gizmos
                .visible_gizmos
                .contains(GizmoSet::VOXEL_INTERSECTIONS)
                || flags.is_disabled()
            {
                return;
            }
            let model_to_world_transform = frame.create_transform_to_parent_space()
                * model_transform.crate_transform_to_entity_space().isometry;

            voxel_objects.push((*voxel_object_id, model_to_world_transform));
        },
        ![SceneGraphParentNodeHandle]
    );
    query!(ecs_world, |gizmos: &GizmosComp,
                       frame: &ReferenceFrame,
                       model_transform: &ModelTransform,
                       voxel_object_id: &VoxelObjectID,
                       parent: &SceneGraphParentNodeHandle,
                       flags: &SceneEntityFlags| {
        if !gizmos
            .visible_gizmos
            .contains(GizmoSet::VOXEL_INTERSECTIONS)
            || flags.is_disabled()
        {
            return;
        }
        let parent_group_node = scene_graph.group_nodes().node(parent.id);

        let model_to_world_transform = parent_group_node.group_to_root_transform()
            * frame.create_transform_to_parent_space()
            * model_transform.crate_transform_to_entity_space().isometry;

        voxel_objects.push((*voxel_object_id, model_to_world_transform));
    });

    for (i, (object_a, a_to_world)) in voxel_objects.iter().enumerate() {
        for (object_b, b_to_world) in &voxel_objects[i + 1..] {
            buffer_transforms_for_voxel_intersections_gizmo(
                model_instance_manager,
                voxel_object_manager,
                scene_camera,
                *object_a,
                *object_b,
                a_to_world,
                b_to_world,
            );
        }
    }
}

fn buffer_transforms_for_model_instance_gizmos(
    model_instance_manager: &mut ModelInstanceManager,
    scene_graph: &SceneGraph,
    current_frame_count: u32,
    visible_gizmos: GizmoSet,
    model_instance_node_id: ModelInstanceNodeID,
) {
    let node = scene_graph
        .model_instance_nodes()
        .node(model_instance_node_id);

    if node.frame_count_when_last_visible() != current_frame_count {
        return;
    }

    let model_view_transform = model_instance_manager
        .feature::<InstanceModelViewTransformWithPrevious>(node.model_view_transform_feature_id())
        .current;

    if visible_gizmos.contains(GizmoType::ReferenceFrameAxes.as_set()) {
        model_instance_manager.buffer_instance_feature(
            GizmoType::ReferenceFrameAxes.only_model_id(),
            &model_view_transform,
        );
    }

    if visible_gizmos.contains(GizmoType::BoundingSphere.as_set())
        && let Some(transform) =
            compute_transform_for_bounding_sphere_gizmo(node, model_view_transform)
    {
        model_instance_manager
            .buffer_instance_feature(GizmoType::BoundingSphere.only_model_id(), &transform);
    }
}

fn compute_transform_for_bounding_sphere_gizmo(
    node: &ModelInstanceNode,
    model_view_transform: InstanceModelViewTransform,
) -> Option<InstanceModelViewTransform> {
    let bounding_sphere = node.get_model_bounding_sphere()?;
    let center = bounding_sphere.center();
    let radius = bounding_sphere.radius();

    let bounding_sphere_from_unit_sphere =
        Similarity3::from_parts(center.coords.into(), UnitQuaternion::identity(), radius);

    let model_view_transform: Similarity3<_> = model_view_transform.into();

    Some(InstanceModelViewTransform::from(
        model_view_transform * bounding_sphere_from_unit_sphere,
    ))
}

fn buffer_transform_for_light_sphere_gizmo(
    model_instance_manager: &mut ModelInstanceManager,
    light_manager: &LightManager,
    light_id: OmnidirectionalLightID,
) {
    let Some(light) = light_manager.get_omnidirectional_light(light_id) else {
        return;
    };

    let light_sphere_from_unit_sphere = InstanceModelViewTransform {
        translation: light.camera_space_position().coords,
        scaling: light.max_reach(),
        rotation: UnitQuaternion::identity(),
    };

    model_instance_manager.buffer_instance_feature(
        GizmoType::LightSphere.only_model_id(),
        &light_sphere_from_unit_sphere,
    );
}

fn buffer_transform_for_shadowable_light_sphere_gizmo(
    model_instance_manager: &mut ModelInstanceManager,
    light_manager: &LightManager,
    light_id: ShadowableOmnidirectionalLightID,
) {
    let Some(light) = light_manager.get_shadowable_omnidirectional_light(light_id) else {
        return;
    };

    let light_sphere_from_unit_sphere = InstanceModelViewTransform {
        translation: light.camera_space_position().coords,
        scaling: light.max_reach(),
        rotation: UnitQuaternion::identity(),
    };

    model_instance_manager.buffer_instance_feature(
        GizmoType::LightSphere.only_model_id(),
        &light_sphere_from_unit_sphere,
    );
}

fn buffer_transforms_for_shadow_cubemap_faces_gizmo(
    model_instance_manager: &mut ModelInstanceManager,
    light_manager: &LightManager,
    light_id: ShadowableOmnidirectionalLightID,
) {
    let Some(light) = light_manager.get_shadowable_omnidirectional_light(light_id) else {
        return;
    };

    let light_space_to_camera_transform = light.create_light_space_to_camera_transform();

    let cubemap_near_plane_transform = InstanceModelViewTransform::from(
        light_space_to_camera_transform.prepend_scaling(light.near_distance()),
    );

    model_instance_manager.buffer_instance_feature(
        &GizmoType::ShadowCubemapFaces.models()[SHADOW_CUBEMAP_FACES_GIZMO_PLANES_MODEL_IDX]
            .model_id,
        &cubemap_near_plane_transform,
    );

    let cubemap_far_plane_transform = InstanceModelViewTransform::from(
        light_space_to_camera_transform.prepend_scaling(light.far_distance()),
    );

    model_instance_manager.buffer_instance_feature(
        &GizmoType::ShadowCubemapFaces.models()[SHADOW_CUBEMAP_FACES_GIZMO_PLANES_MODEL_IDX]
            .model_id,
        &cubemap_far_plane_transform,
    );

    model_instance_manager.buffer_instance_feature(
        &GizmoType::ShadowCubemapFaces.models()[SHADOW_CUBEMAP_FACES_GIZMO_OUTLINES_MODEL_IDX]
            .model_id,
        &cubemap_far_plane_transform,
    );
}

fn buffer_transforms_for_shadow_map_cascades_gizmo(
    model_instance_manager: &mut ModelInstanceManager,
    light_manager: &LightManager,
    scene_camera: &SceneCamera,
    light_id: ShadowableUnidirectionalLightID,
) {
    let Some(light) = light_manager.get_shadowable_unidirectional_light(light_id) else {
        return;
    };

    let view_frustum = scene_camera.camera().view_frustum();

    for (cascade_idx, near_partition_depth_for_cascade) in iter::once(light.near_partition_depth())
        .chain(light.partition_depths().iter().copied())
        .enumerate()
    {
        let plane_distance =
            view_frustum.convert_linear_depth_to_view_distance(near_partition_depth_for_cascade);

        // If the distance equals the near distance, we add a tiny offset to
        // make the plane doesn't get clipped
        let plane_z = -plane_distance.max(view_frustum.near_distance() + 1e-6);

        let plane_height = view_frustum.height_at_distance(plane_distance);
        let scaling = plane_height * scene_camera.camera().aspect_ratio().max(1.0);

        let camera_cascade_from_vertical_square = InstanceModelViewTransform {
            translation: vector![0.0, 0.0, plane_z],
            rotation: UnitQuaternion::identity(),
            scaling,
        };

        model_instance_manager.buffer_instance_feature(
            &GizmoType::ShadowMapCascades.models()[cascade_idx].model_id,
            &camera_cascade_from_vertical_square,
        );
    }
}

fn buffer_transforms_for_anchor_gizmos(
    anchor_manager: &AnchorManager,
    model_instance_manager: &mut ModelInstanceManager,
    scene_camera: &SceneCamera,
    frame: &ReferenceFrame,
    rigid_body_id: TypedRigidBodyID,
) {
    const RADIUS: f32 = 0.1;

    let anchor_points: TinyVec<[_; 8]> = match rigid_body_id {
        TypedRigidBodyID::Dynamic(rigid_body_id) => anchor_manager
            .dynamic()
            .anchors_for_body(rigid_body_id)
            .map(|(_, point)| *point)
            .collect(),
        TypedRigidBodyID::Kinematic(rigid_body_id) => anchor_manager
            .kinematic()
            .anchors_for_body(rigid_body_id)
            .map(|(_, point)| *point)
            .collect(),
    };

    for anchor_point in anchor_points {
        let world_sphere_from_unit_sphere_transform = frame.create_transform_to_parent_space()
            * Similarity3::from_parts(
                anchor_point.coords.cast().into(),
                UnitQuaternion::identity(),
                RADIUS,
            );

        let view_sphere_from_unit_sphere_transform =
            scene_camera.view_transform() * world_sphere_from_unit_sphere_transform;

        model_instance_manager.buffer_instance_feature(
            GizmoType::Anchors.only_model_id(),
            &InstanceModelViewTransform::from(view_sphere_from_unit_sphere_transform),
        );
    }
}

fn buffer_transforms_for_kinematics_gizmos(
    model_instance_manager: &mut ModelInstanceManager,
    parameters: &GizmoParameters,
    scene_camera: &SceneCamera,
    camera_position: &Point3<f32>,
    frame: &ReferenceFrame,
    motion: &Motion,
    visible_gizmos: GizmoSet,
) {
    if visible_gizmos.contains(GizmoType::LinearVelocity.as_set()) {
        let (direction, speed) = UnitVector3::new_and_get(motion.linear_velocity);

        let length = parameters.linear_velocity_scale * speed;

        if abs_diff_ne!(length, 0.0) {
            model_instance_manager.buffer_instance_feature(
                GizmoType::LinearVelocity.only_model_id(),
                &model_view_transform_for_vector_gizmo(
                    scene_camera,
                    camera_position,
                    frame.position,
                    direction,
                    length,
                ),
            );
        }
    }

    if visible_gizmos.contains(GizmoType::AngularVelocity.as_set()) {
        let length =
            parameters.angular_velocity_scale * motion.angular_velocity.angular_speed().radians();

        if abs_diff_ne!(length, 0.0) {
            model_instance_manager.buffer_instance_feature(
                GizmoType::AngularVelocity.only_model_id(),
                &model_view_transform_for_vector_gizmo(
                    scene_camera,
                    camera_position,
                    frame.position,
                    *motion.angular_velocity.axis_of_rotation(),
                    length,
                ),
            );
        }
    }
}

fn buffer_transforms_for_dynamics_gizmos(
    rigid_body_manager: &RigidBodyManager,
    model_instance_manager: &mut ModelInstanceManager,
    parameters: &GizmoParameters,
    scene_camera: &SceneCamera,
    camera_position: &Point3<f32>,
    frame: &ReferenceFrame,
    rigid_body_id: DynamicRigidBodyID,
    visible_gizmos: GizmoSet,
) {
    let Some(rigid_body) = rigid_body_manager.get_dynamic_rigid_body(rigid_body_id) else {
        return;
    };

    if visible_gizmos.contains(GizmoType::CenterOfMass.as_set()) {
        let radius = sphere_radius_from_mass_and_density(
            rigid_body.mass(),
            parameters.center_of_mass_sphere_density,
        );

        let world_sphere_from_unit_sphere_transform = Similarity3::from_parts(
            frame.position.coords.cast().into(),
            UnitQuaternion::identity(),
            radius as f32,
        );

        let view_sphere_from_unit_sphere_transform =
            scene_camera.view_transform() * world_sphere_from_unit_sphere_transform;

        model_instance_manager.buffer_instance_feature(
            GizmoType::CenterOfMass.only_model_id(),
            &InstanceModelViewTransform::from(view_sphere_from_unit_sphere_transform),
        );
    }

    if visible_gizmos.contains(GizmoType::AngularMomentum.as_set()) {
        let (axis, magnitude) = UnitVector3::new_and_get(*rigid_body.angular_momentum());

        let length = parameters.angular_momentum_scale * magnitude;

        if abs_diff_ne!(length, 0.0) {
            model_instance_manager.buffer_instance_feature(
                GizmoType::AngularMomentum.only_model_id(),
                &model_view_transform_for_vector_gizmo(
                    scene_camera,
                    camera_position,
                    frame.position,
                    axis,
                    length,
                ),
            );
        }
    }

    if visible_gizmos.contains(GizmoType::Force.as_set()) {
        let (direction, magnitude) = UnitVector3::new_and_get(*rigid_body.total_force());

        let length = parameters.force_scale * magnitude;

        if abs_diff_ne!(length, 0.0) {
            model_instance_manager.buffer_instance_feature(
                GizmoType::Force.only_model_id(),
                &model_view_transform_for_vector_gizmo(
                    scene_camera,
                    camera_position,
                    frame.position,
                    direction,
                    length,
                ),
            );
        }
    }

    if visible_gizmos.contains(GizmoType::Torque.as_set()) {
        let (axis, magnitude) = UnitVector3::new_and_get(*rigid_body.total_torque());

        let length = parameters.torque_scale * magnitude;

        if abs_diff_ne!(length, 0.0) {
            model_instance_manager.buffer_instance_feature(
                GizmoType::Torque.only_model_id(),
                &model_view_transform_for_vector_gizmo(
                    scene_camera,
                    camera_position,
                    frame.position,
                    axis,
                    length,
                ),
            );
        }
    }
}

fn sphere_radius_from_mass_and_density(mass: f64, density: f64) -> f64 {
    f64::cbrt(3.0 * mass / (4.0 * std::f64::consts::PI * density))
}

fn model_view_transform_for_vector_gizmo(
    scene_camera: &SceneCamera,
    camera_position: &Point3<f32>,
    position: Point3<f64>,
    direction: UnitVector3<f64>,
    length: f64,
) -> InstanceModelViewTransform {
    let rotation = compute_rotation_to_camera_space_for_cylindrical_billboard(
        camera_position.cast(),
        position,
        direction,
    );

    let model_to_world_transform = Similarity3::from_parts(
        position.coords.cast().into(),
        rotation.cast(),
        length as f32,
    );

    (scene_camera.view_transform() * model_to_world_transform).into()
}

/// Computes the model-view rotation of a billboard model such that:
/// - The y-axis in the billboard's model space aligns with the given world
///   space billboard axis.
/// - The z-axis in the billboard's model space points as directly as possible
///   towards the camera.
fn compute_rotation_to_camera_space_for_cylindrical_billboard(
    camera_position: Point3<f64>,
    billboard_position: Point3<f64>,
    billboard_axis: UnitVector3<f64>,
) -> UnitQuaternion<f64> {
    let y_axis = billboard_axis;

    let to_camera = UnitVector3::new_normalize(camera_position - billboard_position);

    // Project the vector from the billboard to the camera onto the plane
    // perpendicular to the y-axis
    let z_vector = to_camera.as_ref() - y_axis.dot(&to_camera) * y_axis.as_ref();

    let z_axis = if z_vector.magnitude_squared() > 1e-6 {
        UnitVector3::new_normalize(z_vector)
    } else {
        // View direction is aligned with the y-axis, use fallback
        let fallback_axis = if y_axis.x.abs() < 0.9 {
            Vector3::x_axis()
        } else {
            Vector3::z_axis()
        };
        UnitVector3::new_normalize(fallback_axis.cross(&y_axis))
    };

    let x_axis = UnitVector3::new_normalize(y_axis.cross(&z_axis));

    UnitQuaternion::from_basis_unchecked(&[
        x_axis.into_inner(),
        y_axis.into_inner(),
        z_axis.into_inner(),
    ])
}

fn buffer_transforms_for_collider_gizmos(
    model_instance_manager: &mut ModelInstanceManager,
    collision_world: &CollisionWorld,
    voxel_object_manager: &VoxelObjectManager,
    scene_camera: &SceneCamera,
    camera_position: &Point3<f32>,
    collidable_id: CollidableID,
    visible_gizmos: GizmoSet,
) {
    let Some(descriptor) = collision_world.get_collidable_descriptor(collidable_id) else {
        return;
    };

    let models = match descriptor.kind() {
        CollidableKind::Dynamic if visible_gizmos.contains(GizmoType::DynamicCollider.as_set()) => {
            GizmoType::DynamicCollider.models()
        }
        CollidableKind::Static if visible_gizmos.contains(GizmoType::StaticCollider.as_set()) => {
            GizmoType::StaticCollider.models()
        }
        CollidableKind::Phantom if visible_gizmos.contains(GizmoType::PhantomCollider.as_set()) => {
            GizmoType::PhantomCollider.models()
        }
        _ => {
            return;
        }
    };

    let Some(collidable) = collision_world.get_collidable_with_descriptor(descriptor) else {
        return;
    };

    match collidable.collidable() {
        Collidable::Sphere(sphere_collidable) => {
            let sphere = sphere_collidable.sphere();

            let unit_sphere_to_sphere_collider_transform = Similarity3::from_parts(
                sphere.center().coords.cast().into(),
                UnitQuaternion::identity(),
                sphere.radius() as f32,
            );

            let model_to_camera_transform =
                scene_camera.view_transform() * unit_sphere_to_sphere_collider_transform;

            model_instance_manager.buffer_instance_feature(
                &models[COLLIDER_GIZMO_SPHERE_MODEL_IDX].model_id,
                &InstanceModelViewTransform::from(model_to_camera_transform),
            );
        }
        Collidable::Plane(plane_collidable) => {
            let plane = plane_collidable.plane();

            // Make the plane appear infinite by putting the center of the mesh
            // at the camera position (projected so as not to change the plane
            // displacement) and scaling the mesh to reach the camera's far
            // distance
            let translation = plane.project_point_onto_plane(&camera_position.cast());
            let rotation = rotation_between_axes(&Vector3::z_axis(), plane.unit_normal());
            let scaling = scene_camera.camera().view_frustum().far_distance();

            let unit_square_to_plane_collider_transform =
                Similarity3::from_parts(translation.coords.cast().into(), rotation.cast(), scaling);

            let model_to_camera_transform =
                scene_camera.view_transform() * unit_square_to_plane_collider_transform;

            model_instance_manager.buffer_instance_feature(
                &models[COLLIDER_GIZMO_PLANE_MODEL_IDX].model_id,
                &InstanceModelViewTransform::from(model_to_camera_transform),
            );
        }
        Collidable::VoxelObject(voxel_object_collidable) => {
            let Some(voxel_object) =
                voxel_object_manager.get_voxel_object(voxel_object_collidable.object_id())
            else {
                return;
            };
            let voxel_object = voxel_object.object();

            let voxel_radius = 0.5 * voxel_object.voxel_extent();

            let transform_from_object_to_world_space = voxel_object_collidable
                .transform_to_object_space()
                .inverse();

            let transform_from_object_to_camera_space =
                scene_camera.view_transform().cast() * transform_from_object_to_world_space;

            let rotation_from_object_to_camera_space =
                transform_from_object_to_camera_space.rotation.cast();
            let scaling_from_object_to_camera_space = voxel_radius as f32;

            let mut transforms = Vec::with_capacity(voxel_object.surface_voxel_count_heuristic());

            voxel_object.for_each_surface_voxel(&mut |[i, j, k], _, _| {
                let voxel_center_in_object_space =
                    voxel_object.voxel_center_position_from_object_voxel_indices(i, j, k);

                let voxel_center_in_camera_space = transform_from_object_to_camera_space
                    .transform_point(&voxel_center_in_object_space);

                let model_to_camera_transform = InstanceModelViewTransform {
                    translation: voxel_center_in_camera_space.coords.cast(),
                    rotation: rotation_from_object_to_camera_space,
                    scaling: scaling_from_object_to_camera_space,
                };

                transforms.push(model_to_camera_transform);
            });

            model_instance_manager.buffer_instance_feature_slice(
                &models[COLLIDER_GIZMO_VOXEL_SPHERE_MODEL_IDX].model_id,
                &transforms,
            );
        }
    }
}

fn rotation_between_axes(a: &UnitVector3<f64>, b: &UnitVector3<f64>) -> UnitQuaternion<f64> {
    if let Some(rotation) = UnitQuaternion::rotation_between_axis(a, b) {
        rotation
    } else {
        // If the axes are antiparallel, we pick a suitable axis about which to
        // flip `a`
        let axis_most_orthogonal_to_a = if a.x.abs() < a.y.abs() && a.x.abs() < a.z.abs() {
            Vector3::x()
        } else if a.y.abs() < a.z.abs() {
            Vector3::y()
        } else {
            Vector3::z()
        };
        let axis_perpendicular_to_a =
            UnitVector3::new_normalize(a.cross(&axis_most_orthogonal_to_a));

        UnitQuaternion::from_axis_angle(&axis_perpendicular_to_a, std::f64::consts::PI)
    }
}

fn buffer_transforms_for_voxel_chunks_gizmo(
    model_instance_manager: &mut ModelInstanceManager,
    voxel_object_manager: &VoxelObjectManager,
    scene_graph: &SceneGraph,
    parameters: &GizmoParameters,
    current_frame_count: u32,
    model_instance_node_id: ModelInstanceNodeID,
    voxel_object_id: VoxelObjectID,
) {
    let node = scene_graph
        .model_instance_nodes()
        .node(model_instance_node_id);

    if node.frame_count_when_last_visible() != current_frame_count {
        return;
    }

    let Some(voxel_object) = voxel_object_manager.get_voxel_object(voxel_object_id) else {
        return;
    };

    let model_view_transform: Similarity3<_> = model_instance_manager
        .feature::<InstanceModelViewTransformWithPrevious>(node.model_view_transform_feature_id())
        .current
        .into();

    let models = GizmoType::VoxelChunks.models();

    let (uniform_chunk_model_id, non_uniform_chunk_model_id) = if parameters.show_interior_chunks {
        (
            &models[VOXEL_CHUNKS_GIZMO_NON_OBSCURABLE_UNIFORM_MODEL_IDX].model_id,
            &models[VOXEL_CHUNKS_GIZMO_NON_OBSCURABLE_NON_UNIFORM_MODEL_IDX].model_id,
        )
    } else {
        (
            &models[VOXEL_CHUNKS_GIZMO_OBSCURABLE_UNIFORM_MODEL_IDX].model_id,
            &models[VOXEL_CHUNKS_GIZMO_OBSCURABLE_NON_UNIFORM_MODEL_IDX].model_id,
        )
    };

    let voxel_extent = voxel_object.object().voxel_extent() as f32;

    voxel_object
        .object()
        .for_each_occupied_chunk(&mut |[chunk_i, chunk_j, chunk_k], chunk| {
            let model_id = match chunk {
                VoxelChunk::Uniform(_) => uniform_chunk_model_id,
                VoxelChunk::NonUniform(_) => non_uniform_chunk_model_id,
                VoxelChunk::Empty => {
                    return;
                }
            };

            let chunk_offset_in_voxels =
                CHUNK_SIZE as f64 * vector![chunk_i as f64, chunk_j as f64, chunk_k as f64];

            let chunk_transform = model_view_transform.prepend_scaling(voxel_extent)
                * Translation3::from(chunk_offset_in_voxels.cast());

            model_instance_manager.buffer_instance_feature(
                model_id,
                &InstanceModelViewTransform::from(chunk_transform),
            );
        });
}

fn buffer_transforms_for_voxel_intersections_gizmo(
    model_instance_manager: &mut ModelInstanceManager,
    voxel_object_manager: &VoxelObjectManager,
    scene_camera: &SceneCamera,
    object_a_id: VoxelObjectID,
    object_b_id: VoxelObjectID,
    transform_from_a_to_world_space: &Isometry3<f32>,
    transform_from_b_to_world_space: &Isometry3<f32>,
) {
    let Some(object_a) = voxel_object_manager.get_voxel_object(object_a_id) else {
        return;
    };
    let object_a = object_a.object();

    let Some(object_b) = voxel_object_manager.get_voxel_object(object_b_id) else {
        return;
    };
    let object_b = object_b.object();

    let transform_from_b_to_a =
        transform_from_a_to_world_space.inverse() * transform_from_b_to_world_space;

    let Some((voxel_ranges_for_a, voxel_ranges_for_b)) =
        ChunkedVoxelObject::determine_voxel_ranges_encompassing_intersection(
            object_a,
            object_b,
            &transform_from_b_to_a.cast(),
        )
    else {
        return;
    };

    let transform_from_a_to_camera_space =
        scene_camera.view_transform() * transform_from_a_to_world_space;

    let transform_from_b_to_camera_space =
        scene_camera.view_transform() * transform_from_b_to_world_space;

    let mut transforms = Vec::with_capacity(256);

    let mut add_transforms = |voxel_object: &ChunkedVoxelObject,
                              transform_from_object_to_camera_space: &Isometry3<f32>,
                              i,
                              j,
                              k| {
        let voxel_center_in_object_space =
            voxel_object.voxel_center_position_from_object_voxel_indices(i, j, k);

        let voxel_center_in_camera_space = transform_from_object_to_camera_space
            .transform_point(&voxel_center_in_object_space.cast());

        let model_to_camera_transform = InstanceModelViewTransform {
            translation: voxel_center_in_camera_space.coords,
            rotation: transform_from_object_to_camera_space.rotation,
            scaling: 0.5 * voxel_object.voxel_extent() as f32,
        };

        transforms.push(model_to_camera_transform);
    };

    object_a.for_each_surface_voxel_in_voxel_ranges(voxel_ranges_for_a, &mut |[i, j, k], _, _| {
        add_transforms(object_a, &transform_from_a_to_camera_space, i, j, k);
    });

    object_b.for_each_surface_voxel_in_voxel_ranges(voxel_ranges_for_b, &mut |[i, j, k], _, _| {
        add_transforms(object_b, &transform_from_b_to_camera_space, i, j, k);
    });

    model_instance_manager
        .buffer_instance_feature_slice(GizmoType::VoxelIntersections.only_model_id(), &transforms);
}
