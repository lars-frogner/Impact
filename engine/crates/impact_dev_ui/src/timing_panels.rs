pub mod render_pass;
pub mod task;

use super::UserInterfaceConfig;
use impact::egui::{Color32, Context, Frame, Id, SidePanel, Ui, ecolor::linear_u8_from_linear_f32};

fn timing_panel(
    ctx: &Context,
    config: &UserInterfaceConfig,
    name: impl Into<Id>,
    default_width: f32,
    add_contents: impl FnOnce(&mut Ui),
) {
    let frame = Frame::side_top_panel(&ctx.style());
    let fill = Color32::from_black_alpha(linear_u8_from_linear_f32(config.alpha).max(1));

    SidePanel::left(name)
        .frame(Frame { fill, ..frame })
        .resizable(false)
        .default_width(default_width)
        .show(ctx, add_contents);
}
