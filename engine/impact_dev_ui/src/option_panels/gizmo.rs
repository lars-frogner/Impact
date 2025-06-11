use super::{LabelAndHoverText, option_checkbox, option_group, option_panel};
use crate::UserInterfaceConfig;
use impact::{
    egui::{Context, Ui},
    engine::Engine,
    gizmo::{GizmoManager, GizmoType, GizmoVisibility},
};

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
    for gizmo in GizmoType::all() {
        let mut visible = gizmo_manager
            .config()
            .visibility(gizmo)
            .is_visible_for_all();

        if option_checkbox(
            ui,
            &mut visible,
            LabelAndHoverText {
                label: gizmo.label(),
                hover_text: gizmo.description(),
            },
        )
        .changed()
        {
            gizmo_manager.set_visibility_for_gizmo(
                gizmo,
                if visible {
                    GizmoVisibility::VisibleForAll
                } else {
                    GizmoVisibility::Hidden
                },
            );
        }
    }
}
