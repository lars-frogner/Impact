//! Tasks for synchronizing render passes.

use super::RenderPassManager;
use crate::{
    define_task,
    rendering::{RenderingTag, SyncRenderBuffers},
    world::{World, WorldTaskScheduler},
};
use anyhow::Result;

define_task!(
    /// This [`Task`](crate::scheduling::Task) ensures that all render
    /// passes required for rendering the entities present in the render
    /// buffers exist.
    ///
    /// Render passes whose entities are no longer present in the
    /// buffers will be removed, and missing render passes for
    /// new entities will be created.
    [pub] SyncRenderPasses,
    depends_on = [SyncRenderBuffers],
    execute_on = [RenderingTag],
    |world: &World| {
        with_debug_logging!("Synchronizing render passes"; {
            match world.get_active_camera_id() {
                Some(camera_id) => {
                    let renderer = world.renderer().read().unwrap();
                    let render_buffer_manager = renderer.render_buffer_manager().read().unwrap();
                    let mut render_pass_manager = renderer.render_pass_manager().write().unwrap();

                    render_pass_manager.sync_with_render_buffers(
                        renderer.core_system(),
                        renderer.assets(),
                        &world.model_library().read().unwrap(),
                        render_buffer_manager.synchronized(),
                        camera_id
                    )
                },
                None => Ok(())
            }
        })
    }
);

impl RenderPassManager {
    /// Registers tasks for synchronizing render passes
    /// in the given task scheduler.
    pub fn register_tasks(task_scheduler: &mut WorldTaskScheduler) -> Result<()> {
        task_scheduler.register_task(SyncRenderPasses)
    }
}
