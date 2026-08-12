use crate::{CustomElements, UserInterfaceConfig};
use impact::{
    command::{AdminCommand, instrumentation::InstrumentationAdminCommand, utils::ToActiveState},
    egui::{Frame, Panel, Ui},
    engine::Engine,
};

#[derive(Clone, Copy, Debug, Default)]
pub struct Toolbar;

impl Toolbar {
    pub fn run(
        &mut self,
        ui: &mut Ui,
        config: &mut UserInterfaceConfig,
        engine: &Engine,
        custom_panels: &mut impl CustomElements,
    ) {
        Panel::top("toolbar_panel")
            .frame(Frame {
                ..Frame::side_top_panel(ui.style())
            })
            .show_inside(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.toggle_value(&mut config.show_ui_options, "UI options");

                    ui.toggle_value(&mut config.show_rendering_options, "Rendering options");

                    ui.toggle_value(&mut config.show_physics_options, "Physics options");

                    ui.toggle_value(&mut config.show_gizmo_options, "Gizmos");

                    if ui
                        .toggle_value(&mut config.show_task_timings, "Task timings")
                        .changed()
                    {
                        engine.enqueue_admin_command(AdminCommand::Instrumentation(
                            InstrumentationAdminCommand::SetTaskTimings(
                                ToActiveState::from_enabled(config.show_task_timings),
                            ),
                        ));
                    }

                    if ui
                        .toggle_value(&mut config.show_render_pass_timings, "Render pass timings")
                        .changed()
                    {
                        engine.enqueue_admin_command(AdminCommand::Instrumentation(
                            InstrumentationAdminCommand::SetRenderPassTimings(
                                ToActiveState::from_enabled(config.show_render_pass_timings),
                            ),
                        ));
                    }

                    custom_panels.run_toolbar_buttons(ui);
                });
            });
    }
}
