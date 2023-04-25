//! Tasks for rendering.

use crate::{
    rendering::{RenderPassManager, RenderResourceManager, RenderingSystem, SyncRenderPasses},
    scheduling::Task,
    thread::ThreadPoolTaskErrors,
    window::ControlFlow,
    world::{World, WorldTaskScheduler},
    {define_execution_tag, define_task},
};
use anyhow::Result;

define_execution_tag!(
    /// Execution tag for [`Task`](crate::scheduling::Task)s
    /// related to rendering.
    [pub] RenderingTag
);

define_task!(
    /// This [`Task`](crate::scheduling::Task) executes the
    /// [`RenderingSystem::render`](crate::rendering::RenderingSystem::render)
    /// method.
    [pub] Render,
    depends_on = [SyncRenderPasses],
    execute_on = [RenderingTag],
    |world: &World| {
        with_debug_logging!("Rendering"; {
            world.capture_screenshots()?;
            world.renderer().read().unwrap().render()
        })
    }
);

impl RenderingSystem {
    /// Registers all tasks needed for rendering in the given
    /// task scheduler.
    pub fn register_tasks(task_scheduler: &mut WorldTaskScheduler) -> Result<()> {
        RenderResourceManager::register_tasks(task_scheduler)?;
        RenderPassManager::register_tasks(task_scheduler)?;
        task_scheduler.register_task(Render)
    }

    /// Identifies rendering-related errors that need special
    /// handling in the given set of task errors and handles them.
    pub fn handle_task_errors(
        &self,
        task_errors: &mut ThreadPoolTaskErrors,
        control_flow: &mut ControlFlow<'_>,
    ) {
        if let Err(render_error) = task_errors.take_result_of(Render.id()) {
            self.handle_render_error(render_error, control_flow);
        }
        if task_errors.n_errors() > 0 {
            log::error!("Aborting due to fatal errors");
            control_flow.exit();
        }
    }
}
