//! Setup of gizmos for new entities.

use crate::{GizmoManager, GizmoSet, GizmoType, GizmoVisibility, Gizmos};

pub fn prepare_gizmos(gizmo_manager: &GizmoManager, gizmos: Option<&Gizmos>) -> Gizmos {
    let mut visible_gizmos = gizmos.map_or_else(GizmoSet::empty, |gizmos| gizmos.visible_gizmos);

    for gizmo in GizmoType::all() {
        match gizmo_manager.visibilities().get_for(gizmo) {
            GizmoVisibility::Hidden => {
                visible_gizmos.remove(gizmo.as_set());
            }
            GizmoVisibility::VisibleForAll => {
                visible_gizmos.insert(gizmo.as_set());
            }
            GizmoVisibility::VisibleForSelected => {}
        }
    }

    Gizmos { visible_gizmos }
}
