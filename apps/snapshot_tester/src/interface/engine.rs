//! The app's public interface for the engine.

use crate::{
    App,
    interface::{
        access_app, access_app_mut,
        scripting::{self, ScriptLib},
        with_dropped_read_guard,
    },
};
use anyhow::{Context, Result};
use dynamic_lib::DynamicLibrary;
use impact::{
    application::ApplicationInterface,
    command::{AdminCommand, SystemAdminCommand, capture::CaptureAdminCommand},
    engine::Engine,
};
use std::sync::Arc;

#[derive(Debug)]
pub struct AppInterfaceForEngine;

impl ApplicationInterface for AppInterfaceForEngine {
    fn on_engine_initialized(&self, engine: Arc<Engine>) -> Result<()> {
        log::debug!("Loading script library");
        ScriptLib::load().context("Failed to load script library")?;

        let mut app = access_app_mut();
        app.set_initialized_engine(engine);

        if app.test_scenes.is_empty() {
            log::info!("No scenes to test, exiting");
            app.engine()
                .enqueue_admin_command(AdminCommand::System(SystemAdminCommand::Shutdown));
            return Ok(());
        }

        Ok(())
    }

    fn on_new_frame(&self, frame_number: u64) -> Result<()> {
        let mut app = access_app();

        let frame = frame_number as usize;

        if frame == app.test_scenes.len() {
            // All scenes have been rendered and captured
            app.engine()
                .enqueue_admin_command(AdminCommand::System(SystemAdminCommand::Shutdown));
            return Ok(());
        }

        if frame > 0 {
            let rendered_scene = app.test_scenes[frame - 1];

            // Prepare for this frame's scene
            app.engine().reset_world()?;
            rendered_scene.restore_settings(app.engine());
        }

        let scene = app.test_scenes[frame];

        // Setup the scene for this frame
        scene.prepare_settings(app.engine());
        app = with_dropped_read_guard(app, || scripting::setup_scene(scene))?;

        // Request a capture for this frame
        app.engine()
            .enqueue_admin_command(AdminCommand::Capture(CaptureAdminCommand::SaveScreenshot));

        Ok(())
    }

    fn on_shutdown(&self) -> Result<()> {
        access_app().run_comparisons()
    }
}

impl App {
    fn set_initialized_engine(&mut self, engine: Arc<Engine>) {
        self.engine = Some(engine);
    }
}
