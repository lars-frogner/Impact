//! Tasks for synchronizing render passes.

use crate::{
    application::{Application, tasks::AppTaskScheduler},
    define_task,
    gpu::rendering::{resource::tasks::SyncRenderResources, tasks::RenderingTag},
};
use anyhow::Result;

define_task!(
    /// This [`Task`](crate::scheduling::Task) ensures that all render commands
    /// required for rendering the entities are up to date with the current
    /// render resources.
    [pub] SyncRenderCommands,
    depends_on = [SyncRenderResources],
    execute_on = [RenderingTag],
    |app: &Application| {
        with_debug_logging!("Synchronizing render commands"; {
            let renderer = app.renderer().read().unwrap();
            let mut shader_manager = renderer.shader_manager().write().unwrap();
            let render_resource_manager = renderer.render_resource_manager().read().unwrap();
            let mut render_command_manager = renderer.render_command_manager().write().unwrap();
            let scene = app.scene().read().unwrap();
            let material_library = scene.material_library().read().unwrap();

            render_command_manager.sync_with_render_resources(
                renderer.graphics_device(),
                &mut shader_manager,
                &material_library,
                render_resource_manager.synchronized(),
            )
        })
    }
);

/// Registers tasks for synchronizing render commands in the given task
/// scheduler.
pub fn register_render_command_tasks(task_scheduler: &mut AppTaskScheduler) -> Result<()> {
    task_scheduler.register_task(SyncRenderCommands)
}
