//! Top-level management of tasks.

use crate::{engine::Engine, tasks, ui::UserInterface};
use anyhow::Result;
use impact_scheduling::TaskScheduler;
use std::{num::NonZeroUsize, sync::Arc};

pub type RuntimeTaskScheduler = TaskScheduler<RuntimeContext>;

/// Shared context providing access to engine and UI resources for tasks.
///
/// This context is passed to all tasks running in the runtime scheduler,
/// giving them access to the engine and user interface systems they need
/// to perform their work. The context is thread-safe and can be shared
/// across multiple worker threads.
#[derive(Clone, Debug)]
pub struct RuntimeContext {
    engine: Arc<Engine>,
    user_interface: Arc<dyn UserInterface>,
}

impl RuntimeContext {
    pub(super) fn new(engine: Arc<Engine>, user_interface: Arc<dyn UserInterface>) -> Self {
        Self {
            engine,
            user_interface,
        }
    }

    pub fn engine(&self) -> &Engine {
        self.engine.as_ref()
    }

    pub fn user_interface(&self) -> &dyn UserInterface {
        self.user_interface.as_ref()
    }
}

/// Creates a new task scheduler with the given number of workers and
/// registers all tasks in it.
///
/// # Errors
/// Returns an error the registration of any of the tasks failed.
pub fn create_task_scheduler(
    ctx: RuntimeContext,
    n_workers: NonZeroUsize,
) -> Result<RuntimeTaskScheduler> {
    let mut task_scheduler = RuntimeTaskScheduler::new(n_workers, ctx);
    tasks::register_all_tasks(&mut task_scheduler)?;
    task_scheduler.complete_task_registration()?;
    Ok(task_scheduler)
}
