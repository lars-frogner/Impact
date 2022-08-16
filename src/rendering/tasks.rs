use crate::{
    rendering::{RenderBufferManager, RenderPassManager, RenderingSystem, SyncRenderPasses},
    scheduling::Task,
    thread::ThreadPoolTaskErrors,
    window::ControlFlow,
    world::{World, WorldTaskScheduler},
    {define_execution_tag, define_task},
};
use anyhow::Result;

define_execution_tag!([pub] RenderingTag);

define_task!(
    [pub] Render,
    depends_on = [SyncRenderPasses],
    execute_on = [RenderingTag],
    |world: &World| {
        with_debug_logging!("Rendering"; world.renderer().read().unwrap().render())
    }
);

impl RenderingSystem {
    pub fn register_tasks(task_scheduler: &mut WorldTaskScheduler) -> Result<()> {
        RenderBufferManager::register_tasks(task_scheduler)?;
        RenderPassManager::register_tasks(task_scheduler)?;
        task_scheduler.register_task(Render)
    }

    pub fn handle_task_errors(
        &self,
        task_errors: &mut ThreadPoolTaskErrors,
        control_flow: &mut ControlFlow<'_>,
    ) {
        if let Err(render_error) = task_errors.take_result_of(Render.id()) {
            self.handle_render_error(render_error, control_flow);
        }
    }
}
