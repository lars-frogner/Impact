//! The top-level orchestrator of engine components.

pub mod headless;
pub mod tasks;

#[cfg(feature = "window")]
pub mod window;

use crate::{
    engine::Engine,
    runtime::tasks::RuntimeTaskScheduler,
    ui::{NoUserInterface, UserInterface},
};
use anyhow::Result;
use impact_profiling::instrumentation;
use impact_thread::pool::DynamicThreadPool;
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
}

/// Configuration parameters for the engine runtime.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct RuntimeConfig {
    /// Number of threads for parallel task execution.
    ///
    /// Note: Using more than one thread may break determinism.
    n_task_threads: NonZeroUsize,
    /// Communication queue capacity for task threads.
    task_queue_capacity: NonZeroUsize,
    /// Number of threads for parallelization within certain tasks.
    ///
    /// Note: Using more than one thread may break determinism.
    n_intra_task_threads: NonZeroUsize,
    /// Communication queue capacity for intra-task threads.
    intra_task_queue_capacity: NonZeroUsize,
}

impl<UI> Runtime<UI>
where
    UI: UserInterface + 'static,
{
    pub fn new(mut engine: Engine, user_interface: UI, config: RuntimeConfig) -> Result<Self> {
        instrumentation::initialize();
        instrumentation::set_thread_name("Main");

        if config.n_intra_task_threads.get() > 1 {
            let thread_pool = DynamicThreadPool::new_dynamic(
                config.n_intra_task_threads,
                config.intra_task_queue_capacity,
            );
            engine.set_intra_task_thread_pool(Some(thread_pool));
        }

        let engine = Arc::new(engine);
        let user_interface = Arc::new(user_interface);

        let ctx = RuntimeContext::new(engine.clone(), user_interface.clone());

        let task_scheduler =
            tasks::create_task_scheduler(ctx, config.n_task_threads, config.task_queue_capacity)?;

        Ok(Self {
            engine,
            user_interface,
            task_scheduler,
        })
    }
}

impl Runtime<NoUserInterface> {
    pub fn new_without_ui(engine: Engine, config: RuntimeConfig) -> Result<Self> {
        Self::new(engine, NoUserInterface, config)
    }
}

impl<UI> Runtime<UI> {
    pub fn engine(&self) -> &Engine {
        self.engine.as_ref()
    }

    pub fn arc_engine(&self) -> Arc<Engine> {
        Arc::clone(&self.engine)
    }

    pub fn user_interface(&self) -> &UI {
        self.user_interface.as_ref()
    }

    pub fn perform_game_loop_iteration(&self) -> Result<()> {
        self.engine
            .perform_game_loop_iteration(&self.task_scheduler)
    }

    pub fn resize_rendering_surface(&self, new_width: NonZeroU32, new_height: NonZeroU32) {
        self.engine.resize_rendering_surface(new_width, new_height);
    }

    pub fn update_pixels_per_point(&self, pixels_per_point: f64) {
        self.engine.update_pixels_per_point(pixels_per_point);
    }

    pub fn shutdown_requested(&self) -> bool {
        self.engine.shutdown_requested()
    }
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            n_task_threads: NonZeroUsize::new(1).unwrap(),
            task_queue_capacity: NonZeroUsize::new(1024).unwrap(),
            n_intra_task_threads: NonZeroUsize::new(1).unwrap(),
            intra_task_queue_capacity: NonZeroUsize::new(1024).unwrap(),
        }
    }
}
