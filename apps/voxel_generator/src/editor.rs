use impact::{
    egui::{Color32, Context, Frame, Id, SidePanel, Ui, Window, ecolor::linear_u8_from_linear_f32},
    engine::Engine,
};
use impact_dev_ui::{CustomPanels, UserInterfaceConfig as DevUserInterfaceConfig};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug)]
pub struct Editor {
    config: EditorConfig,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct EditorConfig {
    pub show_editor: bool,
}

impl Editor {
    pub fn new(config: EditorConfig) -> Self {
        Self { config }
    }
}

impl CustomPanels for Editor {
    fn run_toolbar_buttons(&mut self, ui: &mut Ui) {
        ui.toggle_value(&mut self.config.show_editor, "Voxel editor");
    }

    fn run_panels(&mut self, ctx: &Context, config: &DevUserInterfaceConfig, engine: &Engine) {
        if !self.config.show_editor {
            return;
        }
        editor_panel(ctx, config, "Editor panel", |ui| {});

        Window::new("Generator graph")
            .default_size((800.0, 600.0))
            .vscroll(false)
            .hscroll(false)
            .show(ctx, |ui| {});
    }
}

impl Default for EditorConfig {
    fn default() -> Self {
        Self { show_editor: true }
    }
}

fn editor_panel(
    ctx: &Context,
    config: &DevUserInterfaceConfig,
    name: impl Into<Id>,
    add_contents: impl FnOnce(&mut Ui),
) {
    let frame = Frame::side_top_panel(&ctx.style());
    let fill = Color32::from_black_alpha(linear_u8_from_linear_f32(config.alpha).max(1));

    SidePanel::left(name)
        .frame(Frame { fill, ..frame })
        .resizable(true)
        .show(ctx, add_contents);
}
