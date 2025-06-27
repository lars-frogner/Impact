//! Tasks for all engine subsystems.

use crate::{
    engine::Engine, gizmo, gpu, physics, runtime::tasks::RuntimeTaskScheduler, scene, voxel,
};
use anyhow::Result;
use impact_thread::ThreadPoolTaskErrors;

impl Engine {
    /// Identifies errors that need special handling in the given set of task
    /// errors and handles them.
    pub fn handle_task_errors(&self, task_errors: &mut ThreadPoolTaskErrors) {
        self.simulator
            .read()
            .unwrap()
            .handle_task_errors(task_errors);

        self.scene.read().unwrap().handle_task_errors(task_errors);

        self.renderer
            .read()
            .unwrap()
            .handle_task_errors(task_errors);
    }
}

/// Registers all tasks for engine subsystems in the given task scheduler.
pub fn register_engine_tasks(task_scheduler: &mut RuntimeTaskScheduler) -> Result<()> {
    scene::tasks::register_scene_tasks(task_scheduler)?;
    gpu::rendering::tasks::register_rendering_tasks(task_scheduler)?;
    physics::tasks::register_physics_tasks(task_scheduler)?;
    voxel::tasks::register_voxel_tasks(task_scheduler)?;
    gizmo::tasks::register_gizmo_tasks(task_scheduler)
}
