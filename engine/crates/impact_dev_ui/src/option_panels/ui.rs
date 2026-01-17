use super::{LabelAndHoverText, option_checkbox, option_drag_value, option_group, option_panel};
use crate::UserInterfaceConfig;
use impact::egui::{Context, DragValue, Ui};

#[derive(Clone, Copy, Debug, Default)]
pub struct UIOptionPanel;

impl UIOptionPanel {
    pub fn run(&mut self, ctx: &Context, config: &mut UserInterfaceConfig) {
        option_panel(ctx, "ui_option_panel", config.alpha, |ui| {
            option_group(ui, "ui_options", |ui| {
                ui_options(ui, config);
            });
        });
    }
}

fn ui_options(ui: &mut Ui, config: &mut UserInterfaceConfig) {
    option_checkbox(
        ui,
        &mut config.show_time_overlay,
        LabelAndHoverText {
            label: "Time overlay",
            hover_text: "Whether to show elapsed time and FPS in upper right corner.",
        },
    );

    option_drag_value(
        ui,
        LabelAndHoverText {
            label: "Opacity",
            hover_text: "The opacity of UI elements.",
        },
        DragValue::new(&mut config.alpha)
            .speed(0.01)
            .range(0.0..=1.0),
    );
}
