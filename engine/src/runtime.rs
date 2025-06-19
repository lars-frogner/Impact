//! The top-level orchestrator of engine components.

pub mod window;

use crate::{
    engine::{Engine, tasks::EngineTaskScheduler},
    game_loop::{GameLoop, GameLoopConfig},
    thread::ThreadPoolResult,
    ui::UserInterface,
};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{
    num::{NonZeroU32, NonZeroUsize},
    sync::Arc,
};

/// Top-level orchestrator of engine components.
#[derive(Debug)]
pub struct Runtime {
    engine: Arc<Engine>,
    task_scheduler: EngineTaskScheduler,
    game_loop: GameLoop,
    user_interface: UserInterface,
}

/// Configuration parameters for the engine runtime.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeConfig {
    n_worker_threads: NonZeroUsize,
    game_loop: GameLoopConfig,
}

impl Runtime {
    pub fn new(
        engine: Engine,
        user_interface: UserInterface,
        config: RuntimeConfig,
    ) -> Result<Self> {
        let (engine, task_scheduler) = engine.create_task_scheduler(config.n_worker_threads)?;

        let game_loop = GameLoop::new(config.game_loop);

        Ok(Self {
            engine,
            task_scheduler,
            game_loop,
            user_interface,
        })
    }

    pub fn engine(&self) -> &Engine {
        self.engine.as_ref()
    }

    pub fn arc_engine(&self) -> Arc<Engine> {
        Arc::clone(&self.engine)
    }

    fn run_ui_processing(&mut self) {
        // This could be moved into GameLoop::perform_iteration and the tesselation
        // could be done in parallel with other tasks. The actual running must be
        // done before beginning to execute other tasks since user interactions
        // can affect the engine state.
        let raw_ui_output = self.user_interface.run(&self.game_loop, &self.engine);
        let ui_output = self.user_interface.process_raw_output(raw_ui_output);
        *self.engine.ui_output().write().unwrap() = Some(ui_output);
    }

    fn perform_game_loop_iteration(&mut self) -> ThreadPoolResult {
        self.game_loop
            .perform_iteration(&self.engine, &self.task_scheduler)
    }

    fn resize_rendering_surface(&self, new_width: NonZeroU32, new_height: NonZeroU32) {
        self.engine.resize_rendering_surface(new_width, new_height);
    }

    fn update_pixels_per_point(&self, pixels_per_point: f64) {
        self.engine.update_pixels_per_point(pixels_per_point);
    }

    fn shutdown_requested(&self) -> bool {
        self.engine.shutdown_requested()
    }
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            n_worker_threads: NonZeroUsize::new(1).unwrap(),
            game_loop: GameLoopConfig::default(),
        }
    }
}
