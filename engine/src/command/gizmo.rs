//! Commands for operating gizmos.

use crate::{
    engine::Engine,
    gizmo::{GizmoParameters, GizmoType, GizmoVisibility},
    lock_order::OrderedRwLock,
};

#[derive(Clone, Debug)]
pub enum GizmoCommand {
    SetVisibility {
        gizmo_type: GizmoType,
        visibility: GizmoVisibility,
    },
    SetParameters(GizmoParameters),
}

pub fn set_gizmo_visibility(engine: &Engine, gizmo_type: GizmoType, visibility: GizmoVisibility) {
    impact_log::info!("Setting {gizmo_type:?} visibility to {visibility:?}");
    let mut gizmo_manager = engine.gizmo_manager().owrite();
    gizmo_manager.set_visibility_for_gizmo(gizmo_type, visibility);
}

pub fn set_gizmo_parameters(engine: &Engine, parameters: GizmoParameters) {
    impact_log::info!("Setting gizmo parameters to {parameters:?}");
    let mut gizmo_manager = engine.gizmo_manager().owrite();
    *gizmo_manager.parameters_mut() = parameters;
}
