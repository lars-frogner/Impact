//! ECS systems for gizmo management.

use crate::{
    gizmo::{GizmoManager, GizmoSet, GizmoType, GizmoVisibility, components::GizmosComp},
    light::{
        LightID, LightStorage,
        components::{OmnidirectionalLightComp, ShadowableOmnidirectionalLightComp},
    },
    model::{
        InstanceFeatureManager,
        transform::{InstanceModelViewTransform, InstanceModelViewTransformWithPrevious},
    },
    scene::{
        ModelInstanceNode, ModelInstanceNodeID, SceneGraph,
        components::{SceneEntityFlagsComp, SceneGraphModelInstanceNodeComp},
    },
};
use impact_ecs::{query, world::World as ECSWorld};
use nalgebra::{Similarity3, UnitQuaternion};

pub fn update_visibility_flags_for_gizmo(
    ecs_world: &ECSWorld,
    gizmo_manager: &GizmoManager,
    gizmo: GizmoType,
) {
    let globally_visible = match gizmo_manager.config().visibility(gizmo) {
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
    scene_graph: &SceneGraph<f32>,
    light_storage: &LightStorage,
    current_frame_count: u32,
) {
    query!(
        ecs_world,
        |gizmos: &GizmosComp,
         model_instance_node: &SceneGraphModelInstanceNodeComp,
         flags: &SceneEntityFlagsComp| {
            if gizmos.visible_gizmos.is_empty() || flags.is_disabled() {
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
            if !gizmos.visible_gizmos.contains(GizmoSet::LIGHT_SPHERE) || flags.is_disabled() {
                return;
            }
            buffer_transform_for_light_sphere_gizmo(
                instance_feature_manager,
                light_storage,
                omnidirectional_light.id,
                true,
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

    for gizmo in GizmoType::all_in_set(visible_gizmos) {
        if let Some(gizmo_transform) =
            obtain_transform_for_model_instance_gizmo(node, model_view_transform, gizmo)
        {
            instance_feature_manager.buffer_instance_feature(gizmo.model_id(), &gizmo_transform);
        }
    }
}

fn obtain_transform_for_model_instance_gizmo(
    node: &ModelInstanceNode<f32>,
    model_view_transform: InstanceModelViewTransform,
    gizmo: GizmoType,
) -> Option<InstanceModelViewTransform> {
    match gizmo {
        GizmoType::ReferenceFrameAxes => Some(model_view_transform),
        GizmoType::BoundingSphere => {
            compute_transform_for_bounding_sphere_gizmo(node, model_view_transform)
        }
        GizmoType::LightSphere => None,
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
        GizmoType::LightSphere.model_id(),
        &light_sphere_from_unit_sphere,
    );
}
