//! The game's public interface for the engine.

use crate::{
    Game,
    entities::setup,
    interface::{
        access_game, access_game_mut,
        scripting::{self, ScriptLib},
        with_dropped_write_guard,
    },
};
use anyhow::{Context, Result};
use dynamic_lib::DynamicLibrary;
use impact::{
    application::ApplicationInterface,
    egui,
    engine::Engine,
    impact_ecs::archetype::ArchetypeComponentStorage,
    input::{
        key::KeyboardEvent,
        mouse::{MouseButtonEvent, MouseDragEvent, MouseScrollEvent},
    },
};
use std::sync::Arc;

#[derive(Debug)]
pub struct GameInterfaceForEngine;

impl ApplicationInterface for GameInterfaceForEngine {
    fn on_engine_initialized(&self, engine: Arc<Engine>) -> Result<()> {
        log::debug!("Loading script library");
        ScriptLib::load().context("Failed to load script library")?;

        let mut game = access_game_mut();

        game.activate_script_reloader()?;

        game.set_initialized_engine(engine);
        log::debug!("Engine initialized");

        log::debug!("Setting up UI");
        game.setup_ui();

        log::debug!("Setting up scene");
        let ctx = game.create_setup_context();
        game = with_dropped_write_guard(game, || scripting::setup_scene(ctx))?;

        game.execute_game_commands();

        Ok(())
    }

    fn on_new_frame(&self, _frame_number: u64) -> Result<()> {
        let mut game = access_game_mut();

        if game.should_reset_scene_after_script_reload() || game.game_options.scene_reset_requested
        {
            game.game_options.scene_reset_requested = false;

            log::debug!("Resetting scene");

            game.reset_world()?;

            let ctx = game.create_setup_context();
            game = with_dropped_write_guard(game, || scripting::setup_scene(ctx))?;
        }

        let ctx = game.create_update_context();
        game = with_dropped_write_guard(game, || scripting::update_world(ctx))?;

        game.execute_game_commands();

        Ok(())
    }

    fn on_new_entities(&self, components: &mut ArchetypeComponentStorage) -> Result<()> {
        setup::perform_setup_for_new_entities(&access_game(), components)
    }

    fn handle_keyboard_event(&self, event: KeyboardEvent) -> Result<()> {
        log::trace!("Handling keyboard event {event:?}");
        let ctx = access_game().create_input_context();
        scripting::handle_keyboard_event(ctx, event)
    }

    fn handle_mouse_button_event(&self, event: MouseButtonEvent) -> Result<()> {
        log::trace!("Handling mouse button event {event:?}");
        let ctx = access_game().create_input_context();
        scripting::handle_mouse_button_event(ctx, event)
    }

    fn handle_mouse_drag_event(&self, event: MouseDragEvent) -> Result<()> {
        log::trace!("Handling mouse drag event {event:?}");
        let ctx = access_game().create_input_context();
        scripting::handle_mouse_drag_event(ctx, event)
    }

    fn handle_mouse_scroll_event(&self, event: MouseScrollEvent) -> Result<()> {
        log::trace!("Handling mouse scroll event {event:?}");
        let ctx = access_game().create_input_context();
        scripting::handle_mouse_scroll_event(ctx, event)
    }

    fn run_egui_ui(&self, ctx: &egui::Context, input: egui::RawInput) -> egui::FullOutput {
        access_game_mut().run_ui(ctx, input)
    }
}

impl Game {
    fn set_initialized_engine(&mut self, engine: Arc<Engine>) {
        self.engine = Some(engine);
    }
}
