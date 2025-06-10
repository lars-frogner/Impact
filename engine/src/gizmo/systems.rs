//! ECS systems for gizmo management.

use crate::{
    gizmo::{self, GizmoManager, GizmoSet, GizmoVisibility, components::GizmosComp},
    model::{InstanceFeatureManager, transform::InstanceModelViewTransformWithPrevious},
    scene::{
        SceneGraph,
        components::{SceneEntityFlagsComp, SceneGraphModelInstanceNodeComp},
    },
};
use impact_ecs::{query, world::World as ECSWorld};

pub fn update_visibility_flags_for_reference_frame_gizmo(
    ecs_world: &ECSWorld,
    gizmo_manager: &GizmoManager,
) {
    let globally_visible = match gizmo_manager.config().reference_frame_visibility {
        GizmoVisibility::Hidden => false,
        GizmoVisibility::VisibleForAll => true,
        GizmoVisibility::VisibleForSelected => {
            return;
        }
    };
    query!(ecs_world, |gizmos: &mut GizmosComp| {
        gizmos
            .visible_gizmos
            .set(GizmoSet::REFERENCE_FRAME_AXES, globally_visible);
    });
}

/// Finds entities for which the reference frame axes gizmo should be displayed
/// and copies their model-view transforms to the gizmo's dedicated buffer.
pub fn buffer_transforms_for_reference_frame_gizmos(
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
            if !gizmos
                .visible_gizmos
                .contains(GizmoSet::REFERENCE_FRAME_AXES)
                || flags.is_disabled()
            {
                return;
            }

            let node = scene_graph
                .model_instance_nodes()
                .node(model_instance_node.id);

            if node.frame_count_when_last_visible() != current_frame_count {
                return;
            }

            let transform = instance_feature_manager
                .feature::<InstanceModelViewTransformWithPrevious>(
                    node.model_view_transform_feature_id(),
                )
                .current;

            instance_feature_manager
                .buffer_instance_feature(gizmo::reference_frame_axes_model_id(), &transform);
        }
    );
}
