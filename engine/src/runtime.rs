//! The top-level orchestrator of engine components.

pub mod tasks;
pub mod window;

use crate::{
    engine::Engine,
    game_loop::{GameLoop, GameLoopConfig},
    runtime::tasks::RuntimeTaskScheduler,
    thread::ThreadPoolResult,
    ui::UserInterface,
};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{
    num::{NonZeroU32, NonZeroUsize},
    sync::Arc,
};
use tasks::RuntimeContext;

/// Top-level orchestrator of engine components.
#[derive(Debug)]
pub struct Runtime<UI> {
    engine: Arc<Engine>,
    user_interface: Arc<UI>,
    task_scheduler: RuntimeTaskScheduler,
    game_loop: GameLoop,
}

/// Configuration parameters for the engine runtime.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeConfig {
    n_worker_threads: NonZeroUsize,
    game_loop: GameLoopConfig,
}

impl<UI> Runtime<UI>
where
    UI: UserInterface + 'static,
{
    pub fn new(engine: Engine, user_interface: UI, config: RuntimeConfig) -> Result<Self> {
        let engine = Arc::new(engine);
        let user_interface = Arc::new(user_interface);

        let ctx = RuntimeContext::new(engine.clone(), user_interface.clone());

        let task_scheduler = tasks::create_task_scheduler(ctx, config.n_worker_threads)?;

        let game_loop = GameLoop::new(config.game_loop);

        Ok(Self {
            engine,
            user_interface,
            task_scheduler,
            game_loop,
        })
    }

    pub fn engine(&self) -> &Engine {
        self.engine.as_ref()
    }

    pub fn arc_engine(&self) -> Arc<Engine> {
        Arc::clone(&self.engine)
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
