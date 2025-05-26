//! Management of tasks in the engine.

use crate::{
    engine::Engine, gpu, physics, runtime::EventLoopController, scene, scheduling::TaskScheduler,
    thread::ThreadPoolTaskErrors, voxel,
};
use anyhow::Result;
use std::{num::NonZeroUsize, sync::Arc};

pub type EngineTaskScheduler = TaskScheduler<Engine>;

impl Engine {
    /// Creates a new task scheduler with the given number of workers and
    /// registers all tasks in it.
    ///
    /// # Errors
    /// Returns an error the registration of any of the tasks failed.
    pub fn create_task_scheduler(
        self,
        n_workers: NonZeroUsize,
    ) -> Result<(Arc<Self>, EngineTaskScheduler)> {
        let world = Arc::new(self);
        let mut task_scheduler = EngineTaskScheduler::new(n_workers, Arc::clone(&world));

        register_all_tasks(&mut task_scheduler)?;

        Ok((world, task_scheduler))
    }

    /// Identifies errors that need special handling in the given set of task
    /// errors and handles them.
    pub fn handle_task_errors(
        &self,
        task_errors: &mut ThreadPoolTaskErrors,
        event_loop_controller: &EventLoopController<'_>,
    ) {
        self.simulator
            .read()
            .unwrap()
            .handle_task_errors(task_errors, event_loop_controller);

        self.scene
            .read()
            .unwrap()
            .handle_task_errors(task_errors, event_loop_controller);

        self.renderer
            .read()
            .unwrap()
            .handle_task_errors(task_errors, event_loop_controller);
    }
}

/// Registers all tasks in the given task scheduler.
pub fn register_all_tasks(task_scheduler: &mut EngineTaskScheduler) -> Result<()> {
    scene::tasks::register_scene_tasks(task_scheduler)?;
    gpu::rendering::tasks::register_rendering_tasks(task_scheduler)?;
    physics::tasks::register_physics_tasks(task_scheduler)?;
    voxel::tasks::register_voxel_tasks(task_scheduler)?;
    task_scheduler.complete_task_registration()
}
