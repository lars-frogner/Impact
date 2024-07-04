//! Tasks for synchronizing render passes.

use super::RenderCommandManager;
use crate::{
    define_task,
    gpu::rendering::{RenderingTag, SyncRenderResources},
    world::{World, WorldTaskScheduler},
};
use anyhow::Result;

define_task!(
    /// This [`Task`](crate::scheduling::Task) ensures that all render commands
    /// required for rendering the entities present in the render resources
    /// exist.
    ///
    /// Render commands whose entities are no longer present in the render
    /// resources will be removed, and missing render commands for new entities
    /// will be created.
    [pub] SyncRenderCommands,
    depends_on = [SyncRenderResources],
    execute_on = [RenderingTag],
    |world: &World| {
        with_debug_logging!("Synchronizing render commands"; {
            let renderer = world.renderer().read().unwrap();
            let render_resource_manager = renderer.render_resource_manager().read().unwrap();
            let mut render_command_manager = renderer.render_command_manager().write().unwrap();
            let gpu_computation_library = renderer.gpu_computation_library().read().unwrap();
            let scene = world.scene().read().unwrap();
            let material_library = scene.material_library().read().unwrap();
            let mut shader_manager = scene.shader_manager().write().unwrap();
            let postprocessor = scene.postprocessor().read().unwrap();

            render_command_manager.sync_with_render_resources(
                renderer.config(),
                renderer.graphics_device(),
                renderer.rendering_surface(),
                &material_library,
                render_resource_manager.synchronized(),
                renderer.render_attachment_texture_manager(),
                &gpu_computation_library,
                &mut shader_manager,
                &postprocessor,
            )
        })
    }
);

impl RenderCommandManager {
    /// Registers tasks for synchronizing render commands in the given task
    /// scheduler.
    pub fn register_tasks(task_scheduler: &mut WorldTaskScheduler) -> Result<()> {
        task_scheduler.register_task(SyncRenderCommands)
    }
}
