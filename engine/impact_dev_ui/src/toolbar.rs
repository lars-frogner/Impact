use crate::UserInterfaceConfig;
use impact::egui::{Context, DragValue, Frame, TopBottomPanel};

#[derive(Clone, Copy, Debug, Default)]
pub struct Toolbar;

impl Toolbar {
    pub fn run(&mut self, config: &mut UserInterfaceConfig, ctx: &Context) {
        TopBottomPanel::top("toolbar_panel")
            .frame(Frame {
                ..Frame::side_top_panel(&ctx.style())
            })
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.add(
                        DragValue::new(&mut config.alpha)
                            .speed(0.01)
                            .range(0.0..=1.0)
                            .prefix("α "),
                    );
                    ui.toggle_value(&mut config.show_rendering_options, "Rendering options");
                    ui.toggle_value(&mut config.show_physics_options, "Physics options");
                    ui.toggle_value(&mut config.show_gizmo_options, "Gizmos");
                    ui.toggle_value(&mut config.show_task_timings, "Task timings");
                    ui.toggle_value(&mut config.show_render_pass_timings, "Render pass timings");
                    ui.toggle_value(&mut config.show_time_overlay, "Time overlay");
                });
            });
    }
}
