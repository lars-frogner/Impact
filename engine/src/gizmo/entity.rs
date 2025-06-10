//! Management of gizmo visibility for entities.

use crate::{
    gizmo::{GizmoManager, GizmoSet, GizmoVisibility, components::GizmosComp},
    physics::motion::components::ReferenceFrameComp,
};
use impact_ecs::{archetype::ArchetypeComponentStorage, setup};

/// Adds the [`GizmoComp`] component to the new entity if it has any of the
/// relevant components. The component is initialized based on which gizmos are
/// currently configured to be globally visible.
pub fn setup_gizmos_for_new_entity(
    gizmo_manager: &GizmoManager,
    components: &mut ArchetypeComponentStorage,
) {
    setup!(
        components,
        |gizmos: Option<&GizmosComp>| -> GizmosComp {
            let mut visible_gizmos =
                gizmos.map_or_else(GizmoSet::empty, |gizmos| gizmos.visible_gizmos);

            match gizmo_manager.config().reference_frame_visibility {
                GizmoVisibility::Hidden => {
                    visible_gizmos.remove(GizmoSet::REFERENCE_FRAME_AXES);
                }
                GizmoVisibility::VisibleForAll => {
                    visible_gizmos.insert(GizmoSet::REFERENCE_FRAME_AXES);
                }
                GizmoVisibility::VisibleForSelected => {}
            }

            GizmosComp { visible_gizmos }
        },
        [ReferenceFrameComp]
    );
}
