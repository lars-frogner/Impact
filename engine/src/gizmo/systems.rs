//! ECS systems for gizmo management.

use crate::{
    camera::SceneCamera,
    gizmo::{
        GizmoManager, GizmoParameters, GizmoSet, GizmoType, GizmoVisibility,
        components::GizmosComp,
        model::{
            SHADOW_CUBEMAP_FACES_GIZMO_OUTLINES_MODEL_IDX,
            SHADOW_CUBEMAP_FACES_GIZMO_PLANES_MODEL_IDX,
        },
    },
    light::{
        LightID, LightStorage,
        components::{
            OmnidirectionalLightComp, ShadowableOmnidirectionalLightComp,
            ShadowableUnidirectionalLightComp,
        },
    },
    model::{
        InstanceFeatureManager,
        transform::{InstanceModelViewTransform, InstanceModelViewTransformWithPrevious},
    },
    physics::{
        motion::components::{ReferenceFrameComp, VelocityComp},
        rigid_body::{RigidBody, components::RigidBodyComp},
    },
    scene::{
        ModelInstanceNode, ModelInstanceNodeID, SceneGraph,
        components::{SceneEntityFlagsComp, SceneGraphModelInstanceNodeComp},
    },
};
use approx::abs_diff_ne;
use impact_ecs::{query, world::World as ECSWorld};
use impact_math::Angle;
use nalgebra::{Point3, Similarity3, UnitQuaternion, UnitVector3, Vector3, vector};
use std::iter;

pub fn update_visibility_flags_for_gizmo(
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
    instance_feature_manager: &mut InstanceFeatureManager,
    gizmo_manager: &GizmoManager,
    scene_graph: &SceneGraph<f32>,
    light_storage: &LightStorage,
    scene_camera: Option<&SceneCamera<f32>>,
    current_frame_count: u32,
) {
    let Some(scene_camera) = scene_camera else {
        return;
    };
    let camera_position = scene_camera.compute_world_space_position();

    query!(
        ecs_world,
        |gizmos: &GizmosComp,
         model_instance_node: &SceneGraphModelInstanceNodeComp,
         flags: &SceneEntityFlagsComp| {
            if !gizmos
                .visible_gizmos
                .intersects(GizmoSet::REFERENCE_FRAME_AXES.union(GizmoSet::BOUNDING_SPHERE))
                || flags.is_disabled()
            {
                return;
            }
            buffer_transforms_for_model_instance_gizmos(
                instance_feature_manager,
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
         omnidirectional_light: &OmnidirectionalLightComp,
         flags: &SceneEntityFlagsComp| {
            if !gizmos.visible_gizmos.contains(GizmoSet::LIGHT_SPHERE) || flags.is_disabled() {
                return;
            }
            buffer_transform_for_light_sphere_gizmo(
                instance_feature_manager,
                light_storage,
                omnidirectional_light.id,
                false,
            );
        }
    );

    query!(
        ecs_world,
        |gizmos: &GizmosComp,
         omnidirectional_light: &ShadowableOmnidirectionalLightComp,
         flags: &SceneEntityFlagsComp| {
            if flags.is_disabled() {
                return;
            }
            if gizmos.visible_gizmos.contains(GizmoSet::LIGHT_SPHERE) {
                buffer_transform_for_light_sphere_gizmo(
                    instance_feature_manager,
                    light_storage,
                    omnidirectional_light.id,
                    true,
                );
            }
            if gizmos
                .visible_gizmos
                .contains(GizmoSet::SHADOW_CUBEMAP_FACES)
            {
                buffer_transforms_for_shadow_cubemap_faces_gizmo(
                    instance_feature_manager,
                    light_storage,
                    omnidirectional_light.id,
                );
            }
        }
    );

    query!(
        ecs_world,
        |gizmos: &GizmosComp,
         unidirectional_light: &ShadowableUnidirectionalLightComp,
         flags: &SceneEntityFlagsComp| {
            if !gizmos
                .visible_gizmos
                .contains(GizmoSet::SHADOW_MAP_CASCADES)
                || flags.is_disabled()
            {
                return;
            }
            buffer_transforms_for_shadow_map_cascades_gizmo(
                instance_feature_manager,
                light_storage,
                scene_camera,
                unidirectional_light.id,
            );
        }
    );

    query!(
        ecs_world,
        |gizmos: &GizmosComp,
         frame: &ReferenceFrameComp,
         velocity: &VelocityComp,
         flags: &SceneEntityFlagsComp| {
            if !gizmos
                .visible_gizmos
                .intersects(GizmoSet::LINEAR_VELOCITY.union(GizmoSet::ANGULAR_VELOCITY))
                || flags.is_disabled()
            {
                return;
            }
            buffer_transforms_for_kinematics_gizmos(
                instance_feature_manager,
                gizmo_manager.parameters(),
                scene_camera,
                &camera_position,
                frame,
                velocity,
                gizmos.visible_gizmos,
            );
        }
    );

    query!(
        ecs_world,
        |gizmos: &GizmosComp,
         frame: &ReferenceFrameComp,
         rigid_body: &RigidBodyComp,
         flags: &SceneEntityFlagsComp| {
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
                instance_feature_manager,
                gizmo_manager.parameters(),
                scene_camera,
                &camera_position,
                frame,
                &rigid_body.0,
                gizmos.visible_gizmos,
            );
        }
    );
}

fn buffer_transforms_for_model_instance_gizmos(
    instance_feature_manager: &mut InstanceFeatureManager,
    scene_graph: &SceneGraph<f32>,
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

    let model_view_transform = instance_feature_manager
        .feature::<InstanceModelViewTransformWithPrevious>(node.model_view_transform_feature_id())
        .current;

    if visible_gizmos.contains(GizmoType::ReferenceFrameAxes.as_set()) {
        instance_feature_manager.buffer_instance_feature(
            GizmoType::ReferenceFrameAxes.only_model_id(),
            &model_view_transform,
        );
    }

    if visible_gizmos.contains(GizmoType::BoundingSphere.as_set()) {
        if let Some(transform) =
            compute_transform_for_bounding_sphere_gizmo(node, model_view_transform)
        {
            instance_feature_manager
                .buffer_instance_feature(GizmoType::BoundingSphere.only_model_id(), &transform);
        }
    }
}

fn compute_transform_for_bounding_sphere_gizmo(
    node: &ModelInstanceNode<f32>,
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
    instance_feature_manager: &mut InstanceFeatureManager,
    light_storage: &LightStorage,
    light_id: LightID,
    is_shadowable: bool,
) {
    let (camera_space_position, max_reach) = if is_shadowable {
        let Some(light) = light_storage.get_shadowable_omnidirectional_light(light_id) else {
            return;
        };
        (*light.camera_space_position(), light.max_reach())
    } else {
        let Some(light) = light_storage.get_omnidirectional_light(light_id) else {
            return;
        };
        (*light.camera_space_position(), light.max_reach())
    };

    let light_sphere_from_unit_sphere = InstanceModelViewTransform {
        translation: camera_space_position.coords,
        scaling: max_reach,
        rotation: UnitQuaternion::identity(),
    };

    instance_feature_manager.buffer_instance_feature(
        GizmoType::LightSphere.only_model_id(),
        &light_sphere_from_unit_sphere,
    );
}

fn buffer_transforms_for_shadow_cubemap_faces_gizmo(
    instance_feature_manager: &mut InstanceFeatureManager,
    light_storage: &LightStorage,
    light_id: LightID,
) {
    let Some(light) = light_storage.get_shadowable_omnidirectional_light(light_id) else {
        return;
    };

    let light_space_to_camera_transform = light.create_light_space_to_camera_transform();

    let cubemap_near_plane_transform: InstanceModelViewTransform = light_space_to_camera_transform
        .prepend_scaling(light.near_distance())
        .into();

    instance_feature_manager.buffer_instance_feature(
        &GizmoType::ShadowCubemapFaces.models()[SHADOW_CUBEMAP_FACES_GIZMO_PLANES_MODEL_IDX]
            .model_id,
        &cubemap_near_plane_transform,
    );

    let cubemap_far_plane_transform: InstanceModelViewTransform = light_space_to_camera_transform
        .prepend_scaling(light.far_distance())
        .into();

    instance_feature_manager.buffer_instance_feature(
        &GizmoType::ShadowCubemapFaces.models()[SHADOW_CUBEMAP_FACES_GIZMO_PLANES_MODEL_IDX]
            .model_id,
        &cubemap_far_plane_transform,
    );

    instance_feature_manager.buffer_instance_feature(
        &GizmoType::ShadowCubemapFaces.models()[SHADOW_CUBEMAP_FACES_GIZMO_OUTLINES_MODEL_IDX]
            .model_id,
        &cubemap_far_plane_transform,
    );
}

fn buffer_transforms_for_shadow_map_cascades_gizmo(
    instance_feature_manager: &mut InstanceFeatureManager,
    light_storage: &LightStorage,
    scene_camera: &SceneCamera<f32>,
    light_id: LightID,
) {
    let Some(light) = light_storage.get_shadowable_unidirectional_light(light_id) else {
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

        instance_feature_manager.buffer_instance_feature(
            &GizmoType::ShadowMapCascades.models()[cascade_idx].model_id,
            &camera_cascade_from_vertical_square,
        );
    }
}

fn buffer_transforms_for_kinematics_gizmos(
    instance_feature_manager: &mut InstanceFeatureManager,
    parameters: &GizmoParameters,
    scene_camera: &SceneCamera<f32>,
    camera_position: &Point3<f32>,
    frame: &ReferenceFrameComp,
    velocity: &VelocityComp,
    visible_gizmos: GizmoSet,
) {
    if visible_gizmos.contains(GizmoType::LinearVelocity.as_set()) {
        let (direction, speed) = UnitVector3::new_and_get(velocity.linear);

        let length = parameters.linear_velocity_scale * speed;

        if abs_diff_ne!(length, 0.0) {
            instance_feature_manager.buffer_instance_feature(
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
        let length = parameters.angular_velocity_scale * velocity.angular.angular_speed().radians();

        if abs_diff_ne!(length, 0.0) {
            instance_feature_manager.buffer_instance_feature(
                GizmoType::AngularVelocity.only_model_id(),
                &model_view_transform_for_vector_gizmo(
                    scene_camera,
                    camera_position,
                    frame.position,
                    *velocity.angular.axis_of_rotation(),
                    length,
                ),
            );
        }
    }
}

fn buffer_transforms_for_dynamics_gizmos(
    instance_feature_manager: &mut InstanceFeatureManager,
    parameters: &GizmoParameters,
    scene_camera: &SceneCamera<f32>,
    camera_position: &Point3<f32>,
    frame: &ReferenceFrameComp,
    rigid_body: &RigidBody,
    visible_gizmos: GizmoSet,
) {
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

        let view_sphere_from_unit_sphere_transform: InstanceModelViewTransform =
            (scene_camera.view_transform() * world_sphere_from_unit_sphere_transform).into();

        instance_feature_manager.buffer_instance_feature(
            GizmoType::CenterOfMass.only_model_id(),
            &view_sphere_from_unit_sphere_transform,
        );
    }

    if visible_gizmos.contains(GizmoType::AngularMomentum.as_set()) {
        let (axis, magnitude) = UnitVector3::new_and_get(*rigid_body.angular_momentum());

        let length = parameters.angular_momentum_scale * magnitude;

        if abs_diff_ne!(length, 0.0) {
            instance_feature_manager.buffer_instance_feature(
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
            instance_feature_manager.buffer_instance_feature(
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
            instance_feature_manager.buffer_instance_feature(
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
    scene_camera: &SceneCamera<f32>,
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
