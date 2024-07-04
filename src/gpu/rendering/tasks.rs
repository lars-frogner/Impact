//! Tasks for rendering.

use crate::{
    gpu::rendering::{
        RenderCommandManager, RenderResourceManager, RenderingSystem, SyncRenderCommands,
    },
    scheduling::Task,
    thread::ThreadPoolTaskErrors,
    window::EventLoopController,
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
    /// [`RenderingSystem::render`] method.
    [pub] Render,
    depends_on = [SyncRenderCommands],
    execute_on = [RenderingTag],
    |world: &World| {
        with_debug_logging!("Rendering"; {
            world.capture_screenshots()?;
            let scene = world.scene().read().unwrap();
            let material_library = scene.material_library().read().unwrap();
            world.renderer().read().unwrap().render(&material_library)
        })
    }
);

impl RenderingSystem {
    /// Registers all tasks needed for rendering in the given
    /// task scheduler.
    pub fn register_tasks(task_scheduler: &mut WorldTaskScheduler) -> Result<()> {
        RenderResourceManager::register_tasks(task_scheduler)?;
        RenderCommandManager::register_tasks(task_scheduler)?;
        task_scheduler.register_task(Render)
    }

    /// Identifies rendering-related errors that need special
    /// handling in the given set of task errors and handles them.
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
