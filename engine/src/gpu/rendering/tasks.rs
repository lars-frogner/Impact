//! Tasks for rendering.

use crate::{
    application::{Application, tasks::AppTaskScheduler},
    gpu::rendering::{RenderingSystem, render_command::tasks::SyncRenderCommands},
    scheduling::Task,
    thread::ThreadPoolTaskErrors,
    window::EventLoopController,
    {define_execution_tag, define_task},
};
use anyhow::Result;

use super::{render_command, resource};

define_execution_tag!(
    /// Execution tag for [`Task`](crate::scheduling::Task)s
    /// related to rendering.
    [pub] RenderingTag
);

define_task!(
    /// This [`Task`](crate::scheduling::Task) executes the
    /// [`RenderingSystem::render`] method.
    [pub] Render,
    depends_on = [SyncRenderCommands],
    execute_on = [RenderingTag],
    |app: &Application| {
        with_debug_logging!("Rendering"; {
            let scene = app.scene().read().unwrap();
            app.renderer().write().unwrap().render_to_surface(&scene)?;
            app.capture_screenshots()
        })
    }
);

impl RenderingSystem {
    /// Identifies rendering-related errors that need special handling in the
    /// given set of task errors and handles them.
    pub fn handle_task_errors(
        &self,
        task_errors: &mut ThreadPoolTaskErrors,
        event_loop_controller: &EventLoopController<'_>,
    ) {
        if let Err(render_error) = task_errors.take_result_of(Render.id()) {
            self.handle_render_error(render_error, event_loop_controller);
        }
        if task_errors.n_errors() > 0 {
            log::error!("Aborting due to fatal errors");
            event_loop_controller.exit();
        }
    }
}

/// Registers all tasks needed for rendering in the given task scheduler.
pub fn register_rendering_tasks(task_scheduler: &mut AppTaskScheduler) -> Result<()> {
    resource::tasks::register_render_resource_tasks(task_scheduler)?;
    render_command::tasks::register_render_command_tasks(task_scheduler)?;
    task_scheduler.register_task(Render)
}
