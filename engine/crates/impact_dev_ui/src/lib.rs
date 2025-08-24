//! Development user interface for the Impact engine.

#![allow(clippy::unused_self)]

mod command;
mod option_panels;
mod time_overlay;
mod timing_panels;
mod toolbar;

pub use command::{UICommand, UICommandQueue};

use anyhow::Result;
use impact::{
    egui::{Context, FullOutput, RawInput},
    engine::Engine,
    lock_order::OrderedRwLock,
    ui,
};
use option_panels::{
    gizmo::GizmoOptionPanel, physics::PhysicsOptionPanel, rendering::RenderingOptionPanel,
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
    rendering_option_panel: RenderingOptionPanel,
    physics_option_panel: PhysicsOptionPanel,
    gizmo_option_panel: GizmoOptionPanel,
    task_timing_panel: TaskTimingPanel,
    render_pass_timing_panel: RenderPassTimingPanel,
    time_overlay: TimeOverlay,
    config: UserInterfaceConfig,
}

/// Configuration parameters for the develompment user interface.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct UserInterfaceConfig {
    pub interactive: bool,
    pub alpha: f32,
    pub show_rendering_options: bool,
    pub show_physics_options: bool,
    pub show_gizmo_options: bool,
    pub show_task_timings: bool,
    pub show_render_pass_timings: bool,
    pub show_time_overlay: bool,
    pub disable_cursor_capture: bool,
}

impl UserInterface {
    pub fn new(config: UserInterfaceConfig) -> Self {
        Self {
            config,
            ..Default::default()
        }
    }

    pub fn setup(&self, engine: &Engine) {
        engine.set_controls_enabled(!self.config.interactive);

        engine
            .task_timer()
            .set_enabled(self.config.show_time_overlay);

        engine
            .renderer()
            .owrite()
            .set_render_pass_timings_enabled(self.config.show_render_pass_timings);
    }

    pub fn run(
        &mut self,
        ctx: &Context,
        input: RawInput,
        engine: &Engine,
        command_queue: &UICommandQueue,
    ) -> FullOutput {
        let mut output = ctx.run(input, |ctx| {
            if self.config.interactive {
                self.toolbar.run(ctx, &mut self.config, engine);

                if self.config.show_rendering_options {
                    self.rendering_option_panel.run(ctx, &self.config, engine);
                }
                if self.config.show_physics_options {
                    self.physics_option_panel.run(ctx, &self.config, engine);
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
            }

            if self.config.show_time_overlay {
                self.time_overlay.run(ctx, engine);
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
            show_rendering_options: false,
            show_physics_options: false,
            show_gizmo_options: false,
            show_task_timings: false,
            show_render_pass_timings: false,
            show_time_overlay: true,
            disable_cursor_capture: false,
        }
    }
}
