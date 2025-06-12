//! ECS systems for gizmo management.

use crate::{
    gizmo::{GizmoManager, GizmoType, GizmoVisibility, components::GizmosComp},
    model::{
        InstanceFeatureManager,
        transform::{InstanceModelViewTransform, InstanceModelViewTransformWithPrevious},
    },
    scene::{
        ModelInstanceNode, SceneGraph,
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

            let node = scene_graph
                .model_instance_nodes()
                .node(model_instance_node.id);

            if node.frame_count_when_last_visible() != current_frame_count {
                return;
            }

            let model_view_transform = instance_feature_manager
                .feature::<InstanceModelViewTransformWithPrevious>(
                    node.model_view_transform_feature_id(),
                )
                .current;

            for gizmo in GizmoType::all_in_set(gizmos.visible_gizmos) {
                if let Some(gizmo_transform) =
                    obtain_transform_for_gizmo(node, model_view_transform, gizmo)
                {
                    instance_feature_manager
                        .buffer_instance_feature(gizmo.model_id(), &gizmo_transform);
                }
            }
        }
    );
}

fn obtain_transform_for_gizmo(
    node: &ModelInstanceNode<f32>,
    model_view_transform: InstanceModelViewTransform,
    gizmo: GizmoType,
) -> Option<InstanceModelViewTransform> {
    match gizmo {
        GizmoType::ReferenceFrameAxes => Some(model_view_transform),
        GizmoType::BoundingSphere => {
            compute_transform_for_bounding_sphere_gizmo(node, model_view_transform)
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
