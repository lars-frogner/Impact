use crate::{CustomPanels, UserInterfaceConfig};
use impact::{
    command::{AdminCommand, instrumentation::InstrumentationCommand, uils::ToActiveState},
    egui::{Context, DragValue, Frame, TopBottomPanel},
    engine::Engine,
};

#[derive(Clone, Copy, Debug, Default)]
pub struct Toolbar;

impl Toolbar {
    pub fn run(
        &mut self,
        ctx: &Context,
        config: &mut UserInterfaceConfig,
        engine: &Engine,
        custom_panels: &mut impl CustomPanels,
    ) {
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

                    if ui
                        .toggle_value(&mut config.show_task_timings, "Task timings")
                        .changed()
                    {
                        engine.enqueue_admin_command(AdminCommand::Instrumentation(
                            InstrumentationCommand::SetTaskTimings(ToActiveState::from_enabled(
                                config.show_task_timings,
                            )),
                        ));
                    }

                    if ui
                        .toggle_value(&mut config.show_render_pass_timings, "Render pass timings")
                        .changed()
                    {
                        engine.enqueue_admin_command(AdminCommand::Instrumentation(
                            InstrumentationCommand::SetRenderPassTimings(
                                ToActiveState::from_enabled(config.show_render_pass_timings),
                            ),
                        ));
                    }

                    ui.toggle_value(&mut config.show_time_overlay, "Time overlay");

                    custom_panels.run_toolbar_buttons(ui);
                });
            });
    }
}
