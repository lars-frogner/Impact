pub mod render_pass;
pub mod task;

use super::UserInterfaceConfig;
use impact::egui::{Color32, Frame, Id, Panel, Ui, ecolor::linear_u8_from_linear_f32};

fn timing_panel(
    ui: &mut Ui,
    config: &UserInterfaceConfig,
    name: impl Into<Id>,
    default_size: f32,
    add_contents: impl FnOnce(&mut Ui),
) {
    let frame = Frame::side_top_panel(ui.style());
    let fill = Color32::from_black_alpha(linear_u8_from_linear_f32(config.alpha).max(1));

    Panel::left(name)
        .frame(Frame { fill, ..frame })
        .resizable(false)
        .default_size(default_size)
        .show_inside(ui, add_contents);
}
