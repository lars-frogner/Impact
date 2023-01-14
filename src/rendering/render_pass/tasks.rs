//! Tasks for synchronizing render passes.

use super::RenderPassManager;
use crate::{
    define_task,
    rendering::{RenderingTag, SyncRenderResources},
    world::{World, WorldTaskScheduler},
};
use anyhow::Result;

define_task!(
    /// This [`Task`](crate::scheduling::Task) ensures that all render
    /// passes required for rendering the entities present in the render
    /// resources exist.
    ///
    /// Render passes whose entities are no longer present in the render
    /// resources will be removed, and missing render passes for new
    /// entities will be created.
    [pub] SyncRenderPasses,
    depends_on = [SyncRenderResources],
    execute_on = [RenderingTag],
    |world: &World| {
        with_debug_logging!("Synchronizing render passes"; {
            match world.scene().read().unwrap().get_active_camera_id() {
                Some(camera_id) => {
                    let renderer = world.renderer().read().unwrap();
                    let render_resource_manager = renderer.render_resource_manager().read().unwrap();
                    let mut render_pass_manager = renderer.render_pass_manager().write().unwrap();
                    let scene = world.scene().read().unwrap();
                    let mut shader_builder = scene.shader_manager().write().unwrap();

                    render_pass_manager.sync_with_render_resources(
                        renderer.core_system(),
                        render_resource_manager.synchronized(),
                        &mut shader_builder,
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
