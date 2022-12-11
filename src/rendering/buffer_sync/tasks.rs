//! Tasks for synchronizing render buffers.

use super::{DesynchronizedRenderBuffers, RenderBufferManager};
use crate::{
    define_task,
    rendering::RenderingTag,
    scene::SyncVisibleModelInstances,
    world::{World, WorldTaskScheduler},
};
use anyhow::Result;

define_task!(
    /// This [`Task`](crate::scheduling::Task) performs any required
    /// updates for keeping the [`World`]s render buffers in sync with
    /// the source geometry.
    ///
    /// Render buffers whose source geometry no longer exists will
    /// be removed, and missing render buffers for new geometry
    /// will be created.
    [pub] SyncRenderBuffers,
    depends_on = [
        SyncPerspectiveCameraBuffers,
        SyncColorMeshBuffers,
        SyncTextureMeshBuffers,
        SyncModelInstanceBuffers
    ],
    execute_on = [RenderingTag],
    |world: &World| {
        with_debug_logging!("Completing synchronization of render buffers"; {
            let renderer = world.renderer().read().unwrap();
            let mut render_buffer_manager = renderer.render_buffer_manager().write().unwrap();
            render_buffer_manager.declare_synchronized();
            Ok(())
        })
    }
);

impl RenderBufferManager {
    /// Registers tasks for synchronizing render buffers
    /// in the given task scheduler.
    pub fn register_tasks(task_scheduler: &mut WorldTaskScheduler) -> Result<()> {
        task_scheduler.register_task(SyncPerspectiveCameraBuffers)?;
        task_scheduler.register_task(SyncColorMeshBuffers)?;
        task_scheduler.register_task(SyncTextureMeshBuffers)?;
        task_scheduler.register_task(SyncModelInstanceBuffers)?;
        task_scheduler.register_task(SyncRenderBuffers)
    }
}

define_task!(
    SyncPerspectiveCameraBuffers,
    depends_on = [],
    execute_on = [RenderingTag],
    |world: &World| {
        with_debug_logging!("Synchronizing perspective camera render buffers"; {
            let renderer = world.renderer().read().unwrap();
            let render_buffer_manager = renderer.render_buffer_manager().read().unwrap();
            DesynchronizedRenderBuffers::sync_camera_buffers_with_geometry(
                renderer.core_system(),
                render_buffer_manager
                    .desynchronized()
                    .perspective_camera_buffers
                    .lock()
                    .unwrap()
                    .as_mut(),
                world
                    .scene().read().unwrap()
                    .camera_repository().read().unwrap()
                    .perspective_cameras(),
            );
            Ok(())
        })
    }
);

define_task!(
    SyncColorMeshBuffers,
    depends_on = [],
    execute_on = [RenderingTag],
    |world: &World| {
        with_debug_logging!("Synchronizing color mesh render buffers"; {
            let renderer = world.renderer().read().unwrap();
            let render_buffer_manager = renderer.render_buffer_manager().read().unwrap();
            DesynchronizedRenderBuffers::sync_mesh_buffers_with_geometry(
                renderer.core_system(),
                render_buffer_manager
                    .desynchronized()
                    .color_mesh_buffers
                    .lock()
                    .unwrap()
                    .as_mut(),
                world
                    .scene().read().unwrap()
                    .mesh_repository().read().unwrap()
                    .color_meshes(),
            );
            Ok(())
        })
    }
);

define_task!(
    SyncTextureMeshBuffers,
    depends_on = [],
    execute_on = [RenderingTag],
    |world: &World| {
        with_debug_logging!("Synchronizing texture mesh render buffers"; {
            let renderer = world.renderer().read().unwrap();
            let render_buffer_manager = renderer.render_buffer_manager().read().unwrap();
            DesynchronizedRenderBuffers::sync_mesh_buffers_with_geometry(
                renderer.core_system(),
                render_buffer_manager
                    .desynchronized()
                    .texture_mesh_buffers
                    .lock()
                    .unwrap()
                    .as_mut(),
                world
                    .scene().read().unwrap()
                    .mesh_repository().read().unwrap()
                    .texture_meshes(),
            );
            Ok(())
        })
    }
);

define_task!(
    SyncModelInstanceBuffers,
    depends_on = [SyncVisibleModelInstances],
    execute_on = [RenderingTag],
    |world: &World| {
        with_debug_logging!("Synchronizing model instance render buffers"; {
            let renderer = world.renderer().read().unwrap();
            let render_buffer_manager = renderer.render_buffer_manager().read().unwrap();
            DesynchronizedRenderBuffers::sync_model_instance_buffers_with_geometry(
                renderer.core_system(),
                render_buffer_manager
                    .desynchronized()
                    .model_instance_buffers
                    .lock()
                    .unwrap()
                    .as_mut(),
                &world
                    .scene().read().unwrap()
                    .model_instance_pool().read().unwrap()
                    .model_instance_buffers,
            );
            Ok(())
        })
    }
);
