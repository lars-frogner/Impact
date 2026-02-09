//! ECS systems for gizmo management.

use crate::gizmo::{
    GizmoManager, GizmoParameters, GizmoSet, GizmoType, GizmoVisibility,
    components::GizmosComp,
    model::{
        COLLIDER_GIZMO_PLANE_MODEL_IDX, COLLIDER_GIZMO_SPHERE_MODEL_IDX,
        COLLIDER_GIZMO_VOXEL_SPHERE_MODEL_IDX, SHADOW_CUBEMAP_FACES_GIZMO_OUTLINES_MODEL_IDX,
        SHADOW_CUBEMAP_FACES_GIZMO_PLANES_MODEL_IDX,
        VOXEL_CHUNKS_GIZMO_NON_OBSCURABLE_EMPTY_MODEL_IDX,
        VOXEL_CHUNKS_GIZMO_NON_OBSCURABLE_NON_UNIFORM_MODEL_IDX,
        VOXEL_CHUNKS_GIZMO_NON_OBSCURABLE_UNIFORM_MODEL_IDX,
        VOXEL_CHUNKS_GIZMO_OBSCURABLE_EMPTY_MODEL_IDX,
        VOXEL_CHUNKS_GIZMO_OBSCURABLE_NON_UNIFORM_MODEL_IDX,
        VOXEL_CHUNKS_GIZMO_OBSCURABLE_UNIFORM_MODEL_IDX,
    },
};
use approx::abs_diff_ne;
use impact_camera::{Camera, CameraManager};
use impact_ecs::{query, world::World as ECSWorld};
use impact_geometry::ReferenceFrame;
use impact_id::EntityID;
use impact_light::{
    LightManager, OmnidirectionalLightID, ShadowableOmnidirectionalLightID,
    ShadowableUnidirectionalLightID,
};
use impact_math::{
    angle::Angle,
    consts::f32::PI,
    point::Point3,
    quaternion::{UnitQuaternion, UnitQuaternionC},
    transform::{Isometry3, Similarity3},
    vector::{UnitVector3, Vector3, Vector3C},
};
use impact_model::{
    HasModel, InstanceFeature, ModelInstanceID,
    transform::{InstanceModelViewTransform, InstanceModelViewTransformWithPrevious},
};
use impact_physics::{
    anchor::AnchorManager,
    collision::{CollidableID, CollidableKind},
    quantities::Motion,
    rigid_body::{DynamicRigidBodyID, KinematicRigidBodyID, RigidBodyManager, TypedRigidBodyID},
};
use impact_scene::{
    SceneEntityFlags,
    graph::{ModelInstanceNode, SceneGraph},
    model::ModelInstanceManager,
};
use impact_voxel::{
    VoxelObjectID, VoxelObjectManager,
    chunks::{CHUNK_SIZE, ChunkedVoxelObject, VoxelChunk},
    collidable::{Collidable, CollisionWorld},
};
use std::iter;
use tinyvec::TinyVec;

/// Updates the appropriate gizmo visibility flags for all applicable
/// entities based on which gizmos have been newly configured to be
/// globally visible or hidden.
pub fn update_visibility_flags_for_gizmos(gizmo_manager: &mut GizmoManager, ecs_world: &ECSWorld) {
    if !gizmo_manager.global_visibility_changed_for_any_of_gizmos(GizmoSet::all()) {
        return;
    }

    for gizmo in GizmoType::all() {
        if gizmo_manager.global_visibility_changed_for_any_of_gizmos(gizmo.as_set()) {
            update_visibility_flags_for_gizmo(ecs_world, gizmo_manager, gizmo);
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
    model_instance_manager: &mut ModelInstanceManager,
    ecs_world: &ECSWorld,
    camera_manager: &CameraManager,
    light_manager: &LightManager,
    voxel_object_manager: &VoxelObjectManager,
    scene_graph: &SceneGraph,
    rigid_body_manager: &RigidBodyManager,
    anchor_manager: &AnchorManager,
    collision_world: &CollisionWorld,
    gizmo_manager: &GizmoManager,
    current_frame_count: u32,
) {
    let Some(camera) = camera_manager.active_camera() else {
        return;
    };
    let camera_position = camera.compute_world_space_position();

    query!(
        ecs_world,
        |entity_id: EntityID, gizmos: &GizmosComp, flags: &SceneEntityFlags| {
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
                entity_id,
            );
        },
        [HasModel]
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
                camera,
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
            camera,
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
            camera,
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
            camera,
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
            camera,
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
            camera,
            &camera_position,
            *collidable,
            gizmos.visible_gizmos,
        );
    });

    query!(
        ecs_world,
        |entity_id: EntityID,
         gizmos: &GizmosComp,
         voxel_object_id: &VoxelObjectID,
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
                entity_id,
                *voxel_object_id,
            );
        },
        [HasModel]
    );

    let mut voxel_objects: Vec<(VoxelObjectID, CollidableID)> = Vec::with_capacity(32);

    query!(ecs_world, |gizmos: &GizmosComp,
                       voxel_object_id: &VoxelObjectID,
                       collidable_id: &CollidableID,
                       flags: &SceneEntityFlags| {
        if !gizmos
            .visible_gizmos
            .contains(GizmoSet::VOXEL_INTERSECTIONS)
            || flags.is_disabled()
        {
            return;
        }

        voxel_objects.push((*voxel_object_id, *collidable_id));
    });

    for (i, (object_b_id, collidable_b_id)) in voxel_objects.iter().enumerate() {
        for (object_a_id, collidable_a_id) in &voxel_objects[i + 1..] {
            buffer_transforms_for_voxel_intersections_gizmo(
                model_instance_manager,
                voxel_object_manager,
                collision_world,
                camera,
                *object_a_id,
                *object_b_id,
                *collidable_a_id,
                *collidable_b_id,
            );
        }
    }
}

fn buffer_transforms_for_model_instance_gizmos(
    model_instance_manager: &mut ModelInstanceManager,
    scene_graph: &SceneGraph,
    current_frame_number: u32,
    visible_gizmos: GizmoSet,
    entity_id: EntityID,
) {
    let node = scene_graph
        .model_instance_nodes()
        .node(ModelInstanceID::from_entity_id(entity_id));

    if node.frame_number_when_last_visible() != current_frame_number {
        return;
    }

    let model_view_transform = model_instance_manager
        .feature::<InstanceModelViewTransformWithPrevious>(
            node.get_rendering_feature_id_of_type(
                InstanceModelViewTransformWithPrevious::FEATURE_TYPE_ID,
            )
            .unwrap(),
        )
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
    let bounding_sphere = node.get_model_bounding_sphere()?.aligned();

    let center = bounding_sphere.center();
    let radius = bounding_sphere.radius();

    let bounding_sphere_from_unit_sphere =
        Similarity3::from_parts(*center.as_vector(), UnitQuaternion::identity(), radius);

    let model_view_transform = Similarity3::from(model_view_transform);

    let instance_model_view_transform = model_view_transform * bounding_sphere_from_unit_sphere;

    Some(InstanceModelViewTransform::from(
        &instance_model_view_transform,
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
        translation: *light.camera_space_position().as_vector(),
        scaling: light.max_reach(),
        rotation: UnitQuaternionC::identity(),
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
        translation: *light.camera_space_position().as_vector(),
        scaling: light.far_distance(), // The shader uses the far distance as the light sphere radius
        rotation: UnitQuaternionC::identity(),
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
        &Similarity3::from_isometry(light_space_to_camera_transform)
            .applied_to_scaling(light.near_distance()),
    );

    model_instance_manager.buffer_instance_feature(
        &GizmoType::ShadowCubemapFaces.models()[SHADOW_CUBEMAP_FACES_GIZMO_PLANES_MODEL_IDX]
            .model_id,
        &cubemap_near_plane_transform,
    );

    let cubemap_far_plane_transform = InstanceModelViewTransform::from(
        &Similarity3::from_isometry(light_space_to_camera_transform)
            .applied_to_scaling(light.far_distance()),
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
    camera: &Camera,
    light_id: ShadowableUnidirectionalLightID,
) {
    let Some(light) = light_manager.get_shadowable_unidirectional_light(light_id) else {
        return;
    };

    let view_frustum = camera.projection().view_frustum();

    for (cascade_idx, near_partition_depth_for_cascade) in iter::once(light.near_partition_depth())
        .chain(light.partition_depths().iter().copied())
        .enumerate()
    {
        let plane_distance =
            view_frustum.convert_linear_depth_to_view_distance(near_partition_depth_for_cascade);

        // If the distance equals the near distance, we add a tiny offset to
        // make the plane doesn't get clipped
        let plane_z = -plane_distance.max(view_frustum.near_distance() + 1e-6);

        let plane_height = camera.projection().view_height_at_distance(plane_distance);

        let scaling = plane_height * camera.projection().aspect_ratio().max(1.0);

        let camera_cascade_from_vertical_square = InstanceModelViewTransform {
            translation: Vector3C::new(0.0, 0.0, plane_z),
            rotation: UnitQuaternionC::identity(),
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
    camera: &Camera,
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
        let anchor_point = anchor_point.aligned();

        let world_sphere_from_unit_sphere_transform = frame.create_transform_to_parent_space()
            * Similarity3::from_parts(
                *anchor_point.as_vector(),
                UnitQuaternion::identity(),
                RADIUS,
            );

        let view_sphere_from_unit_sphere_transform =
            camera.view_transform() * world_sphere_from_unit_sphere_transform;

        model_instance_manager.buffer_instance_feature(
            GizmoType::Anchors.only_model_id(),
            &InstanceModelViewTransform::from(&view_sphere_from_unit_sphere_transform),
        );
    }
}

fn buffer_transforms_for_kinematics_gizmos(
    model_instance_manager: &mut ModelInstanceManager,
    parameters: &GizmoParameters,
    camera: &Camera,
    camera_position: &Point3,
    frame: &ReferenceFrame,
    motion: &Motion,
    visible_gizmos: GizmoSet,
) {
    if visible_gizmos.contains(GizmoType::LinearVelocity.as_set()) {
        let linear_velocity = motion.linear_velocity.aligned();

        let (direction, speed) = UnitVector3::normalized_from_and_norm(linear_velocity);

        let length = parameters.linear_velocity_scale * speed;

        if abs_diff_ne!(length, 0.0) {
            model_instance_manager.buffer_instance_feature(
                GizmoType::LinearVelocity.only_model_id(),
                &model_view_transform_for_vector_gizmo(
                    camera,
                    camera_position,
                    frame.position.aligned(),
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
            let axis_of_rotation = motion.angular_velocity.axis_of_rotation().aligned();

            model_instance_manager.buffer_instance_feature(
                GizmoType::AngularVelocity.only_model_id(),
                &model_view_transform_for_vector_gizmo(
                    camera,
                    camera_position,
                    frame.position.aligned(),
                    axis_of_rotation,
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
    camera: &Camera,
    camera_position: &Point3,
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
            frame.position.as_vector().aligned(),
            UnitQuaternion::identity(),
            radius,
        );

        let view_sphere_from_unit_sphere_transform =
            camera.view_transform() * world_sphere_from_unit_sphere_transform;

        model_instance_manager.buffer_instance_feature(
            GizmoType::CenterOfMass.only_model_id(),
            &InstanceModelViewTransform::from(&view_sphere_from_unit_sphere_transform),
        );
    }

    if visible_gizmos.contains(GizmoType::AngularMomentum.as_set()) {
        let angular_momentum = rigid_body.angular_momentum().aligned();

        let (axis, magnitude) = UnitVector3::normalized_from_and_norm(angular_momentum);

        let length = parameters.angular_momentum_scale * magnitude;

        if abs_diff_ne!(length, 0.0) {
            model_instance_manager.buffer_instance_feature(
                GizmoType::AngularMomentum.only_model_id(),
                &model_view_transform_for_vector_gizmo(
                    camera,
                    camera_position,
                    frame.position.aligned(),
                    axis,
                    length,
                ),
            );
        }
    }

    if visible_gizmos.contains(GizmoType::Force.as_set()) {
        let total_force = rigid_body.total_force().aligned();

        let (direction, magnitude) = UnitVector3::normalized_from_and_norm(total_force);

        let length = parameters.force_scale * magnitude;

        if abs_diff_ne!(length, 0.0) {
            model_instance_manager.buffer_instance_feature(
                GizmoType::Force.only_model_id(),
                &model_view_transform_for_vector_gizmo(
                    camera,
                    camera_position,
                    frame.position.aligned(),
                    direction,
                    length,
                ),
            );
        }
    }

    if visible_gizmos.contains(GizmoType::Torque.as_set()) {
        let total_torque = rigid_body.total_torque().aligned();

        let (axis, magnitude) = UnitVector3::normalized_from_and_norm(total_torque);

        let length = parameters.torque_scale * magnitude;

        if abs_diff_ne!(length, 0.0) {
            model_instance_manager.buffer_instance_feature(
                GizmoType::Torque.only_model_id(),
                &model_view_transform_for_vector_gizmo(
                    camera,
                    camera_position,
                    frame.position.aligned(),
                    axis,
                    length,
                ),
            );
        }
    }
}

fn sphere_radius_from_mass_and_density(mass: f32, density: f32) -> f32 {
    f32::cbrt(3.0 * mass / (4.0 * PI * density))
}

fn model_view_transform_for_vector_gizmo(
    camera: &Camera,
    camera_position: &Point3,
    position: Point3,
    direction: UnitVector3,
    length: f32,
) -> InstanceModelViewTransform {
    let rotation = compute_rotation_to_camera_space_for_cylindrical_billboard(
        *camera_position,
        position,
        direction,
    );

    let model_to_world_transform = Similarity3::from_parts(*position.as_vector(), rotation, length);

    let instance_model_view_transform = camera.view_transform() * model_to_world_transform;

    InstanceModelViewTransform::from(&instance_model_view_transform)
}

/// Computes the model-view rotation of a billboard model such that:
/// - The y-axis in the billboard's model space aligns with the given world
///   space billboard axis.
/// - The z-axis in the billboard's model space points as directly as possible
///   towards the camera.
fn compute_rotation_to_camera_space_for_cylindrical_billboard(
    camera_position: Point3,
    billboard_position: Point3,
    billboard_axis: UnitVector3,
) -> UnitQuaternion {
    let y_axis = billboard_axis;

    let to_camera = UnitVector3::normalized_from(camera_position - billboard_position);

    // Project the vector from the billboard to the camera onto the plane
    // perpendicular to the y-axis
    let z_vector = to_camera.as_vector() - y_axis.dot(&to_camera) * y_axis;

    let z_axis = if z_vector.norm_squared() > 1e-6 {
        UnitVector3::normalized_from(z_vector)
    } else {
        // View direction is aligned with the y-axis, use fallback
        let fallback_axis = if y_axis.x().abs() < 0.9 {
            UnitVector3::unit_x()
        } else {
            UnitVector3::unit_z()
        };
        UnitVector3::normalized_from(fallback_axis.cross(&y_axis))
    };

    let x_axis = UnitVector3::normalized_from(y_axis.cross(&z_axis));

    UnitQuaternion::from_basis_unchecked(&[
        *x_axis.as_vector(),
        *y_axis.as_vector(),
        *z_axis.as_vector(),
    ])
}

fn buffer_transforms_for_collider_gizmos(
    model_instance_manager: &mut ModelInstanceManager,
    collision_world: &CollisionWorld,
    voxel_object_manager: &VoxelObjectManager,
    camera: &Camera,
    camera_position: &Point3,
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
            let sphere = sphere_collidable.sphere().aligned();

            let unit_sphere_to_sphere_collider_transform = Similarity3::from_parts(
                *sphere.center().as_vector(),
                UnitQuaternion::identity(),
                sphere.radius(),
            );

            let model_to_camera_transform =
                camera.view_transform() * unit_sphere_to_sphere_collider_transform;

            model_instance_manager.buffer_instance_feature(
                &models[COLLIDER_GIZMO_SPHERE_MODEL_IDX].model_id,
                &InstanceModelViewTransform::from(&model_to_camera_transform),
            );
        }
        Collidable::Plane(plane_collidable) => {
            let plane = plane_collidable.plane().aligned();

            // Make the plane appear infinite by putting the center of the mesh
            // at the camera position (projected so as not to change the plane
            // displacement) and scaling the mesh to reach the camera's far
            // distance
            let translation = plane.project_point_onto_plane(camera_position);
            let rotation =
                UnitQuaternion::rotation_between_axes(&UnitVector3::unit_z(), plane.unit_normal());
            let scaling = camera.projection().view_frustum().far_distance();

            let unit_square_to_plane_collider_transform =
                Similarity3::from_parts(*translation.as_vector(), rotation, scaling);

            let model_to_camera_transform =
                camera.view_transform() * unit_square_to_plane_collider_transform;

            model_instance_manager.buffer_instance_feature(
                &models[COLLIDER_GIZMO_PLANE_MODEL_IDX].model_id,
                &InstanceModelViewTransform::from(&model_to_camera_transform),
            );
        }
        Collidable::VoxelObject(voxel_object_collidable) => {
            let Some(voxel_object) =
                voxel_object_manager.get_voxel_object(voxel_object_collidable.object_id())
            else {
                return;
            };
            let voxel_object = voxel_object.object();

            let transform_to_object_space = voxel_object_collidable
                .transform_to_object_space()
                .aligned();

            let transform_from_object_to_world_space = transform_to_object_space.inverted();

            let transform_from_object_to_camera_space =
                camera.view_transform() * transform_from_object_to_world_space;

            let rotation_from_object_to_camera_space =
                transform_from_object_to_camera_space.rotation();

            let mut transforms = Vec::with_capacity(voxel_object.surface_voxel_count_heuristic());

            voxel_object.for_each_surface_voxel(&mut |[i, j, k], voxel, _| {
                let voxel_center_in_object_space =
                    voxel_object.voxel_center_position_from_object_voxel_indices(i, j, k);

                let voxel_center_in_camera_space = transform_from_object_to_camera_space
                    .transform_point(&voxel_center_in_object_space);

                let voxel_radius = -voxel.signed_distance().to_f32() * voxel_object.voxel_extent();

                let model_to_camera_transform = InstanceModelViewTransform {
                    translation: voxel_center_in_camera_space.as_vector().compact(),
                    rotation: rotation_from_object_to_camera_space.compact(),
                    scaling: voxel_radius,
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

fn buffer_transforms_for_voxel_chunks_gizmo(
    model_instance_manager: &mut ModelInstanceManager,
    voxel_object_manager: &VoxelObjectManager,
    scene_graph: &SceneGraph,
    parameters: &GizmoParameters,
    current_frame_number: u32,
    entity_id: EntityID,
    voxel_object_id: VoxelObjectID,
) {
    let node = scene_graph
        .model_instance_nodes()
        .node(ModelInstanceID::from_entity_id(entity_id));

    if node.frame_number_when_last_visible() != current_frame_number {
        return;
    }

    let Some(voxel_object) = voxel_object_manager.get_voxel_object(voxel_object_id) else {
        return;
    };

    let model_view_transform = Similarity3::from(
        model_instance_manager
            .feature::<InstanceModelViewTransformWithPrevious>(
                node.get_rendering_feature_id_of_type(
                    InstanceModelViewTransformWithPrevious::FEATURE_TYPE_ID,
                )
                .unwrap(),
            )
            .current,
    );

    let models = GizmoType::VoxelChunks.models();

    let (uniform_chunk_model_id, non_uniform_chunk_model_id, empty_chunk_model_id) =
        if parameters.show_interior_chunks {
            (
                &models[VOXEL_CHUNKS_GIZMO_NON_OBSCURABLE_UNIFORM_MODEL_IDX].model_id,
                &models[VOXEL_CHUNKS_GIZMO_NON_OBSCURABLE_NON_UNIFORM_MODEL_IDX].model_id,
                &models[VOXEL_CHUNKS_GIZMO_NON_OBSCURABLE_EMPTY_MODEL_IDX].model_id,
            )
        } else {
            (
                &models[VOXEL_CHUNKS_GIZMO_OBSCURABLE_UNIFORM_MODEL_IDX].model_id,
                &models[VOXEL_CHUNKS_GIZMO_OBSCURABLE_NON_UNIFORM_MODEL_IDX].model_id,
                &models[VOXEL_CHUNKS_GIZMO_OBSCURABLE_EMPTY_MODEL_IDX].model_id,
            )
        };

    let voxel_extent = voxel_object.object().voxel_extent();

    voxel_object
        .object()
        .for_each_chunk(&mut |[chunk_i, chunk_j, chunk_k], chunk| {
            let model_id = match chunk {
                VoxelChunk::Uniform(_) => uniform_chunk_model_id,
                VoxelChunk::NonUniform(_) => non_uniform_chunk_model_id,
                VoxelChunk::Empty => empty_chunk_model_id,
            };

            let chunk_offset_in_voxels =
                CHUNK_SIZE as f32 * Vector3::new(chunk_i as f32, chunk_j as f32, chunk_k as f32);

            let chunk_transform = model_view_transform
                .applied_to_scaling(voxel_extent)
                .applied_to_translation(&chunk_offset_in_voxels);

            model_instance_manager.buffer_instance_feature(
                model_id,
                &InstanceModelViewTransform::from(&chunk_transform),
            );
        });
}

fn buffer_transforms_for_voxel_intersections_gizmo(
    model_instance_manager: &mut ModelInstanceManager,
    voxel_object_manager: &VoxelObjectManager,
    collision_world: &CollisionWorld,
    camera: &Camera,
    object_a_id: VoxelObjectID,
    object_b_id: VoxelObjectID,
    collidable_a_id: CollidableID,
    collidable_b_id: CollidableID,
) {
    let Some(descriptor_a) = collision_world.get_collidable_descriptor(collidable_a_id) else {
        return;
    };
    let Some(collidable_a) = collision_world.get_collidable_with_descriptor(descriptor_a) else {
        return;
    };

    let Some(descriptor_b) = collision_world.get_collidable_descriptor(collidable_b_id) else {
        return;
    };
    let Some(collidable_b) = collision_world.get_collidable_with_descriptor(descriptor_b) else {
        return;
    };

    let (transform_from_world_to_a, transform_from_world_to_b) =
        match (collidable_a.collidable(), collidable_b.collidable()) {
            (Collidable::VoxelObject(voxel_a), Collidable::VoxelObject(voxel_b)) => (
                voxel_a.transform_to_object_space(),
                voxel_b.transform_to_object_space(),
            ),
            _ => return,
        };
    let transform_from_world_to_a = transform_from_world_to_a.aligned();
    let transform_from_world_to_b = transform_from_world_to_b.aligned();

    let Some(object_a) = voxel_object_manager.get_voxel_object(object_a_id) else {
        return;
    };
    let object_a = object_a.object();

    let Some(object_b) = voxel_object_manager.get_voxel_object(object_b_id) else {
        return;
    };
    let object_b = object_b.object();

    let transform_from_b_to_a = transform_from_world_to_a * transform_from_world_to_b.inverted();

    let Some((voxel_ranges_for_a, voxel_ranges_for_b)) =
        ChunkedVoxelObject::determine_voxel_ranges_encompassing_intersection(
            object_a,
            object_b,
            &transform_from_b_to_a,
        )
    else {
        return;
    };

    let transform_from_a_to_camera_space =
        camera.view_transform() * transform_from_world_to_a.inverted();

    let transform_from_b_to_camera_space =
        camera.view_transform() * transform_from_world_to_b.inverted();

    let mut transforms = Vec::with_capacity(256);

    let mut add_transforms = |voxel_object: &ChunkedVoxelObject,
                              transform_from_object_to_camera_space: &Isometry3,
                              i,
                              j,
                              k| {
        let voxel_center_in_object_space =
            voxel_object.voxel_center_position_from_object_voxel_indices(i, j, k);

        let voxel_center_in_camera_space =
            transform_from_object_to_camera_space.transform_point(&voxel_center_in_object_space);

        let model_to_camera_transform = InstanceModelViewTransform {
            translation: voxel_center_in_camera_space.as_vector().compact(),
            rotation: transform_from_object_to_camera_space.rotation().compact(),
            scaling: 0.5 * voxel_object.voxel_extent(),
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
