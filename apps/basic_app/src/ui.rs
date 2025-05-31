//! User interface.

#![allow(clippy::unused_self)]

mod option_panels;
mod time_overlay;
mod timing_panels;
mod toolbar;

use impact::{egui::Context, engine::Engine, game_loop::GameLoop};
use option_panels::{physics::PhysicsOptionPanel, rendering::RenderingOptionPanel};
use time_overlay::TimeOverlay;
use timing_panels::render_pass::RenderPassTimingPanel;
use toolbar::Toolbar;

#[derive(Clone, Debug, Default)]
pub struct UserInterface {
    toolbar: Toolbar,
    rendering_option_panel: RenderingOptionPanel,
    physics_option_panel: PhysicsOptionPanel,
    render_pass_timing_panel: RenderPassTimingPanel,
    time_overlay: TimeOverlay,
    config: UserInterfaceConfig,
}

/// Configuration parameters for the user interface.
#[derive(Clone, Debug)]
pub struct UserInterfaceConfig {
    pub alpha: f32,
    pub show_rendering_options: bool,
    pub show_physics_options: bool,
    pub show_render_pass_timings: bool,
    pub show_time_overlay: bool,
}

impl Default for UserInterfaceConfig {
    fn default() -> Self {
        Self {
            alpha: 0.85,
            show_rendering_options: true,
            show_physics_options: false,
            show_render_pass_timings: true,
            show_time_overlay: true,
        }
    }
}

impl UserInterface {
    pub fn run(&mut self, ctx: &Context, game_loop: &GameLoop, engine: &Engine) {
        if engine.ui_interactive() {
            self.toolbar.run(&mut self.config, ctx);

            if self.config.show_rendering_options {
                self.rendering_option_panel.run(ctx, &self.config, engine);
            }
            if self.config.show_physics_options {
                self.physics_option_panel.run(ctx, &self.config, engine);
            }
            if self.config.show_render_pass_timings {
                self.render_pass_timing_panel.run(ctx, &self.config, engine);
            }
        }

        if self.config.show_time_overlay {
            self.time_overlay.run(ctx, game_loop, engine);
        }
    }
}
