//! Tasks for synchronizing render buffers.

use super::{DesynchronizedRenderResources, RenderResourceManager};
use crate::{
    define_task,
    rendering::RenderingTag,
    scene::BufferVisibleModelInstances,
    world::{World, WorldTaskScheduler},
};
use anyhow::Result;

define_task!(
    /// This [`Task`](crate::scheduling::Task) performs any required
    /// updates for keeping the [`World`]s render resources in sync with
    /// the source data.
    ///
    /// Render buffers whose source data no longer exists will
    /// be removed, and missing render resources for new source
    /// data will be created.
    [pub] SyncRenderResources,
    depends_on = [
        SyncPerspectiveCameraBuffers,
        SyncColorMeshBuffers,
        SyncTextureMeshBuffers,
        SyncMaterialRenderResources,
        SyncInstanceFeatureBuffers
    ],
    execute_on = [RenderingTag],
    |world: &World| {
        with_debug_logging!("Completing synchronization of render resources"; {
            let renderer = world.renderer().read().unwrap();
            let mut render_resource_manager = renderer.render_resource_manager().write().unwrap();
            render_resource_manager.declare_synchronized();
            Ok(())
        })
    }
);

impl RenderResourceManager {
    /// Registers tasks for synchronizing render resources
    /// in the given task scheduler.
    pub fn register_tasks(task_scheduler: &mut WorldTaskScheduler) -> Result<()> {
        task_scheduler.register_task(SyncPerspectiveCameraBuffers)?;
        task_scheduler.register_task(SyncColorMeshBuffers)?;
        task_scheduler.register_task(SyncTextureMeshBuffers)?;
        task_scheduler.register_task(SyncMaterialRenderResources)?;
        task_scheduler.register_task(SyncInstanceFeatureBuffers)?;
        task_scheduler.register_task(SyncRenderResources)
    }
}

define_task!(
    SyncPerspectiveCameraBuffers,
    depends_on = [],
    execute_on = [RenderingTag],
    |world: &World| {
        with_debug_logging!("Synchronizing perspective camera render buffers"; {
            let renderer = world.renderer().read().unwrap();
            let render_resource_manager = renderer.render_resource_manager().read().unwrap();
            if render_resource_manager.is_desynchronized() {
                DesynchronizedRenderResources::sync_camera_buffers_with_cameras(
                    renderer.core_system(),
                    render_resource_manager
                        .desynchronized()
                        .perspective_camera_buffer_managers
                        .lock()
                        .unwrap()
                        .as_mut(),
                    world
                        .scene().read().unwrap()
                        .camera_repository().read().unwrap()
                        .perspective_cameras(),
                );
            }
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
            let render_resource_manager = renderer.render_resource_manager().read().unwrap();
            if render_resource_manager.is_desynchronized() {
                DesynchronizedRenderResources::sync_mesh_buffers_with_meshes(
                    renderer.core_system(),
                    render_resource_manager
                        .desynchronized()
                        .color_mesh_buffer_managers
                        .lock()
                        .unwrap()
                        .as_mut(),
                    world
                        .scene().read().unwrap()
                        .mesh_repository().read().unwrap()
                        .color_meshes(),
                );
            }
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
            let render_resource_manager = renderer.render_resource_manager().read().unwrap();
            if render_resource_manager.is_desynchronized() {
                DesynchronizedRenderResources::sync_mesh_buffers_with_meshes(
                    renderer.core_system(),
                    render_resource_manager
                        .desynchronized()
                        .texture_mesh_buffer_managers
                        .lock()
                        .unwrap()
                        .as_mut(),
                    world
                        .scene().read().unwrap()
                        .mesh_repository().read().unwrap()
                        .texture_meshes(),
                );
            }
            Ok(())
        })
    }
);

define_task!(
    SyncMaterialRenderResources,
    depends_on = [],
    execute_on = [RenderingTag],
    |world: &World| {
        with_debug_logging!("Synchronizing material render resources"; {
            let renderer = world.renderer().read().unwrap();
            let render_resource_manager = renderer.render_resource_manager().read().unwrap();
            if render_resource_manager.is_desynchronized() {
                let scene = world.scene().read().unwrap();
                let shader_library = scene.shader_library().read().unwrap();
                let material_library = scene.material_library().read().unwrap();
                DesynchronizedRenderResources::sync_material_resources_with_material_specifications(
                    renderer.core_system(),
                    renderer.assets(),
                    &shader_library,
                    render_resource_manager
                        .desynchronized()
                        .material_resource_managers
                        .lock()
                        .unwrap()
                        .as_mut(),
                        material_library.material_specifications(),
                )
            } else {
                Ok(())
            }
        })
    }
);

define_task!(
    SyncInstanceFeatureBuffers,
    depends_on = [BufferVisibleModelInstances],
    execute_on = [RenderingTag],
    |world: &World| {
        with_debug_logging!("Synchronizing model instance feature render buffers"; {
            let renderer = world.renderer().read().unwrap();
            let render_resource_manager = renderer.render_resource_manager().read().unwrap();
            if render_resource_manager.is_desynchronized() {
                DesynchronizedRenderResources::sync_instance_feature_buffers_with_manager(
                    renderer.core_system(),
                    render_resource_manager
                        .desynchronized()
                        .instance_feature_buffer_managers
                        .lock()
                        .unwrap()
                        .as_mut(),
                    &world
                        .scene().read().unwrap()
                        .instance_feature_manager().read().unwrap(),
                );
            }
            Ok(())
        })
    }
);
