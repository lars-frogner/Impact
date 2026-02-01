//! Development user interface for the Impact engine.

#![allow(clippy::unused_self)]

mod command;
pub mod option_panels;
pub mod overlay;
mod time_overlay;
mod timing_panels;
mod toolbar;

pub use command::{UICommand, UICommandQueue};

use anyhow::Result;
use impact::{
    command::{
        AdminCommand, controller::ControlAdminCommand,
        instrumentation::InstrumentationAdminCommand, physics::PhysicsAdminCommand,
        uils::ToActiveState,
    },
    egui::{Context, FullOutput, RawInput, Ui},
    engine::Engine,
    ui,
};
use option_panels::{
    gizmo::GizmoOptionPanel, physics::PhysicsOptionPanel, rendering::RenderingOptionPanel,
    ui::UIOptionPanel,
};
use serde::{Deserialize, Serialize};
use std::path::Path;
use time_overlay::TimeOverlay;
use timing_panels::{render_pass::RenderPassTimingPanel, task::TaskTimingPanel};
use toolbar::Toolbar;

/// The development user interface for the Impact engine.
#[derive(Clone, Debug, Default)]
pub struct UserInterface {
    toolbar: Toolbar,
    ui_option_panel: UIOptionPanel,
    rendering_option_panel: RenderingOptionPanel,
    physics_option_panel: PhysicsOptionPanel,
    gizmo_option_panel: GizmoOptionPanel,
    task_timing_panel: TaskTimingPanel,
    render_pass_timing_panel: RenderPassTimingPanel,
    time_overlay: TimeOverlay,
    config: UserInterfaceConfig,
    screenshot_requested: bool,
    single_step_requested: bool,
}

/// Configuration parameters for the develompment user interface.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct UserInterfaceConfig {
    pub interactive: bool,
    pub alpha: f32,
    pub show_ui_options: bool,
    pub show_rendering_options: bool,
    pub show_physics_options: bool,
    pub show_gizmo_options: bool,
    pub show_task_timings: bool,
    pub show_render_pass_timings: bool,
    pub show_time_overlay: bool,
    pub disable_cursor_capture: bool,
    pub hide_ui_during_screenshots: bool,
}

pub trait CustomElements {
    fn run_toolbar_buttons(&mut self, _ui: &mut Ui) {}

    fn run_option_panels(&mut self, _ctx: &Context, _alpha: f32) {}

    fn run_overlays(&mut self, _ctx: &Context) {}
}

#[derive(Clone, Copy, Debug)]
pub struct NoCustomElements;

impl UserInterface {
    pub fn new(config: UserInterfaceConfig) -> Self {
        Self {
            config,
            ..Default::default()
        }
    }

    pub fn setup(&self, engine: &Engine) {
        engine.enqueue_admin_command(AdminCommand::Control(ControlAdminCommand::SetControls(
            ToActiveState::from_enabled(!self.config.interactive),
        )));
        engine.enqueue_admin_command(AdminCommand::Instrumentation(
            InstrumentationAdminCommand::SetTaskTimings(ToActiveState::from_enabled(
                self.config.show_task_timings,
            )),
        ));
        engine.enqueue_admin_command(AdminCommand::Instrumentation(
            InstrumentationAdminCommand::SetRenderPassTimings(ToActiveState::from_enabled(
                self.config.show_render_pass_timings,
            )),
        ));
    }

    pub fn run(
        &mut self,
        ctx: &Context,
        input: RawInput,
        engine: &Engine,
        command_queue: &UICommandQueue,
    ) -> FullOutput {
        self.run_with_custom_elements(ctx, input, engine, command_queue, &mut NoCustomElements)
    }

    pub fn run_with_custom_elements(
        &mut self,
        ctx: &Context,
        input: RawInput,
        engine: &Engine,
        command_queue: &UICommandQueue,
        custom_elements: &mut impl CustomElements,
    ) -> FullOutput {
        let mut output = ctx.run(input, |ctx| {
            // Return without adding any output if we requested a screenshot in
            // the previous frame and should hide the UI
            if self.screenshot_requested && self.config.hide_ui_during_screenshots {
                self.screenshot_requested = false;
                return;
            }

            let single_step_was_requested = self.single_step_requested;

            if self.config.interactive {
                self.toolbar
                    .run(ctx, &mut self.config, engine, custom_elements);

                if self.config.show_ui_options {
                    self.ui_option_panel.run(ctx, &mut self.config);
                }
                if self.config.show_rendering_options {
                    self.rendering_option_panel.run(
                        ctx,
                        &mut self.config,
                        engine,
                        &mut self.screenshot_requested,
                    );
                }
                if self.config.show_physics_options {
                    self.physics_option_panel.run(
                        ctx,
                        &self.config,
                        engine,
                        &mut self.single_step_requested,
                    );
                }
                if self.config.show_gizmo_options {
                    self.gizmo_option_panel.run(ctx, &self.config, engine);
                }
                if self.config.show_task_timings {
                    self.task_timing_panel.run(ctx, &self.config, engine);
                }
                if self.config.show_render_pass_timings {
                    self.render_pass_timing_panel.run(ctx, &self.config, engine);
                }
                custom_elements.run_option_panels(ctx, self.config.alpha);
            }

            if self.config.show_time_overlay {
                self.time_overlay.run(ctx, engine);
            }

            custom_elements.run_overlays(ctx);

            // Re-disable the simulation if we requested a single step in the
            // previous frame
            if single_step_was_requested {
                self.single_step_requested = false;
                engine.enqueue_admin_command(AdminCommand::Physics(
                    PhysicsAdminCommand::SetSimulation(ToActiveState::from_enabled(false)),
                ));
            }
        });

        // The cursor icon will be reset each run, so it won't stay hidden
        // unless we make it
        if !self.config.interactive && !self.config.disable_cursor_capture {
            ui::egui::ensure_cursor_hidden(&mut output);
        }

        self.execute_commands(&mut output, engine, command_queue);

        output
    }
}

impl UserInterfaceConfig {
    /// Parses the configuration from the RON file at the given path.
    pub fn from_ron_file(file_path: impl AsRef<Path>) -> Result<Self> {
        let file_path = file_path.as_ref();
        impact_io::parse_ron_file(file_path)
    }
}

impl Default for UserInterfaceConfig {
    fn default() -> Self {
        Self {
            interactive: true,
            alpha: 0.85,
            show_ui_options: false,
            show_rendering_options: false,
            show_physics_options: false,
            show_gizmo_options: false,
            show_task_timings: false,
            show_render_pass_timings: false,
            show_time_overlay: true,
            disable_cursor_capture: false,
            hide_ui_during_screenshots: true,
        }
    }
}

impl CustomElements for NoCustomElements {}
