//! The app's public interface for the engine.

use crate::{
    App,
    interface::{
        access_app_mut,
        scripting::{self, ScriptLib},
        with_dropped_write_guard,
    },
    user_interface::UI_COMMANDS,
};
use anyhow::{Context, Result};
use dynamic_lib::DynamicLibrary;
use impact::{
    application::ApplicationInterface,
    egui,
    engine::Engine,
    input::{
        key::KeyboardEvent,
        mouse::{MouseButtonEvent, MouseDragEvent, MouseScrollEvent},
    },
};
use std::sync::Arc;

#[derive(Debug)]
pub struct AppInterfaceForEngine;

impl ApplicationInterface for AppInterfaceForEngine {
    fn on_engine_initialized(&self, engine: Arc<Engine>) -> Result<()> {
        log::debug!("Loading script library");
        ScriptLib::load().context("Failed to load script library")?;

        let mut app = access_app_mut();

        app.activate_script_reloader()?;

        app.set_initialized_engine(engine);
        log::debug!("Engine initialized");

        log::debug!("Setting up UI");
        app.setup_ui();

        log::debug!("Setting up scene");
        _ = with_dropped_write_guard(app, scripting::setup_scene)?;

        Ok(())
    }

    fn on_new_frame(&self, _frame_number: u64) -> Result<()> {
        let mut app = access_app_mut();

        if app.should_reset_scene_after_script_reload() || app.app_options.scene_reset_requested {
            app.app_options.scene_reset_requested = false;

            log::debug!("Resetting scene");
            app.reset_world()?;
            _ = with_dropped_write_guard(app, scripting::setup_scene)?;
        }

        Ok(())
    }

    fn handle_keyboard_event(&self, event: KeyboardEvent) -> Result<()> {
        log::trace!("Handling keyboard event {event:?}");
        scripting::handle_keyboard_event(event)
    }

    fn handle_mouse_button_event(&self, event: MouseButtonEvent) -> Result<()> {
        log::trace!("Handling mouse button event {event:?}");
        scripting::handle_mouse_button_event(event)
    }

    fn handle_mouse_drag_event(&self, event: MouseDragEvent) -> Result<()> {
        log::trace!("Handling mouse drag event {event:?}");
        scripting::handle_mouse_drag_event(event)
    }

    fn handle_mouse_scroll_event(&self, event: MouseScrollEvent) -> Result<()> {
        log::trace!("Handling mouse scroll event {event:?}");
        scripting::handle_mouse_scroll_event(event)
    }

    fn run_egui_ui(&self, ctx: &egui::Context, input: egui::RawInput) -> egui::FullOutput {
        access_app_mut().run_ui(ctx, input)
    }
}

impl App {
    fn set_initialized_engine(&mut self, engine: Arc<Engine>) {
        self.engine = Some(engine);
    }

    fn reset_world(&self) -> Result<()> {
        self.engine().reset_world()?;
        UI_COMMANDS.clear();
        Ok(())
    }
}
