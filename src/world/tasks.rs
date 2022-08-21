//! Tasks for coordination between systems in the world.

use crate::{
    define_task,
    rendering::RenderingTag,
    thread::ThreadPoolTaskErrors,
    window::ControlFlow,
    world::{World, WorldTaskScheduler},
};
use anyhow::Result;

define_task!(
    /// This [`Task`](crate::scheduling::Task) uses the
    /// [`SceneGraph`](crate::scene::SceneGraph) to update the
    /// model-to-camera space transforms of the model instances
    /// that are visible with the active camera.
    [pub] SyncVisibleModelInstances,
    depends_on = [],
    execute_on = [RenderingTag],
    |world: &World| {
        with_debug_logging!("Synchronizing visible model instances"; world.sync_visible_model_instances())
    }
);

impl World {
    /// Registers all tasks needed for coordinate between systems
    /// in the world in the given task scheduler.
    pub fn register_world_tasks(task_scheduler: &mut WorldTaskScheduler) -> Result<()> {
        task_scheduler.register_task(SyncVisibleModelInstances)
    }

    /// Identifies world-related errors that need special
    /// handling in the given set of task errors and handles them.
    pub fn handle_world_task_errors(
        &self,
        _task_errors: &mut ThreadPoolTaskErrors,
        _control_flow: &mut ControlFlow<'_>,
    ) {
    }
}
