//! Tasks for rendering.

use crate::{
    gpu::rendering::{RenderingSystem, render_command::tasks::SyncRenderCommands},
    runtime::tasks::{RuntimeContext, RuntimeTaskScheduler},
};
use anyhow::Result;
use impact_scheduling::{Task, define_execution_tag, define_task};
use impact_thread::ThreadPoolTaskErrors;

use super::{render_command, resource};

define_execution_tag!(
    /// Execution tag for [`Task`](crate::scheduling::Task)s
    /// related to rendering.
    [pub] RenderingTag
);

define_task!(
    /// This [`Task`](crate::scheduling::Task) executes the
    /// [`RenderingSystem::render_to_surface`] method.
    [pub] Render,
    depends_on = [SyncRenderCommands],
    execute_on = [RenderingTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();
        instrument_engine_task!("Rendering", engine, {
            let scene = engine.scene().read().unwrap();
            engine.renderer().write().unwrap().render_to_surface(
                &scene,
                ctx.user_interface(),
            )?;
            engine.save_requested_screenshots()
        })
    }
);

impl RenderingSystem {
    /// Identifies rendering-related errors that need special handling in the
    /// given set of task errors and handles them.
    pub fn handle_task_errors(&self, task_errors: &mut ThreadPoolTaskErrors) {
        if let Some(render_error) = task_errors.get_error_of(Render.id()) {
            if let Some(wgpu::SurfaceError::Lost) = render_error.downcast_ref() {
                self.handle_surface_lost();
                task_errors.clear_error_of(Render.id());
            }
        }
    }
}

/// Registers all tasks needed for rendering in the given task scheduler.
pub fn register_rendering_tasks(task_scheduler: &mut RuntimeTaskScheduler) -> Result<()> {
    resource::tasks::register_render_resource_tasks(task_scheduler)?;
    render_command::tasks::register_render_command_tasks(task_scheduler)?;
    task_scheduler.register_task(Render)
}
