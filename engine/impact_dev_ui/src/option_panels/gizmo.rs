use super::{option_checkbox, option_group, option_panel};
use crate::UserInterfaceConfig;
use impact::{
    egui::{Context, Ui},
    engine::Engine,
    gizmo::{GizmoManager, GizmoVisibility},
};

mod docs {
    use crate::option_panels::LabelAndHoverText;

    pub const REFERENCE_FRAME_VISIBLE: LabelAndHoverText = LabelAndHoverText {
        label: "Reference frame axes",
        hover_text: "\
            When enabled, a red, green and blue line segment representing the x- y- \
            and z-axis (respectively) of the local reference frame will be shown \
            atop applicable entities. The lines are of unit length in the local \
            reference frame.",
    };
}

#[derive(Clone, Copy, Debug, Default)]
pub struct GizmoOptionPanel;

impl GizmoOptionPanel {
    pub fn run(&mut self, ctx: &Context, config: &UserInterfaceConfig, engine: &Engine) {
        let mut gizmo_manager = engine.gizmo_manager().write().unwrap();

        option_panel(ctx, config, "gizmo_option_panel", |ui| {
            option_group(ui, "gizmo_options", |ui| {
                gizmo_options(ui, &mut gizmo_manager);
            });
        });
    }
}

fn gizmo_options(ui: &mut Ui, gizmo_manager: &mut GizmoManager) {
    let mut reference_frames_visible = gizmo_manager
        .config()
        .reference_frame_visibility
        .is_visible_for_all();

    if option_checkbox(
        ui,
        &mut reference_frames_visible,
        docs::REFERENCE_FRAME_VISIBLE,
    )
    .changed()
    {
        gizmo_manager.set_visibility_for_reference_frame_gizmo(if reference_frames_visible {
            GizmoVisibility::VisibleForAll
        } else {
            GizmoVisibility::Hidden
        });
    }
}
