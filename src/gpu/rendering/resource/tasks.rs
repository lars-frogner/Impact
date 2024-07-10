//! Tasks for synchronizing GPU buffers.

use super::DesynchronizedRenderResources;
use crate::{
    define_task,
    gpu::rendering::tasks::RenderingTag,
    scene::tasks::{
        BoundOmnidirectionalLightsAndBufferShadowCastingModelInstances,
        BoundUnidirectionalLightsAndBufferShadowCastingModelInstances, BufferVisibleModelInstances,
        SyncLightsInStorage, SyncSceneCameraViewTransform,
    },
    world::{tasks::WorldTaskScheduler, World},
};
use anyhow::Result;

define_task!(
    /// This [`Task`](crate::scheduling::Task) performs any required
    /// updates for keeping the [`World`]s render resources in sync with
    /// the source data.
    ///
    /// GPU buffers whose source data no longer exists will
    /// be removed, and missing render resources for new source
    /// data will be created.
    [pub] SyncRenderResources,
    depends_on = [
        SyncCameraGPUBuffer,
        SyncMeshGPUBuffers,
        SyncLightGPUBuffers,
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

define_task!(
    SyncCameraGPUBuffer,
    depends_on = [SyncSceneCameraViewTransform],
    execute_on = [RenderingTag],
    |world: &World| {
        with_debug_logging!("Synchronizing camera GPU buffer"; {
            let renderer = world.renderer().read().unwrap();
            let render_resource_manager = renderer.render_resource_manager().read().unwrap();
            if render_resource_manager.is_desynchronized() {
                if let Some(scene_camera) = world.scene().read().unwrap()
                                                 .scene_camera().read().unwrap().as_ref() {
                    DesynchronizedRenderResources::sync_camera_buffer_with_scene_camera(
                        renderer.graphics_device(),
                        render_resource_manager
                            .desynchronized()
                            .camera_buffer_manager
                            .lock()
                            .unwrap()
                            .as_mut(),
                        scene_camera,
                    );
                }
            }
            Ok(())
        })
    }
);

define_task!(
    SyncMeshGPUBuffers,
    depends_on = [],
    execute_on = [RenderingTag],
    |world: &World| {
        with_debug_logging!("Synchronizing mesh GPU buffers"; {
            let renderer = world.renderer().read().unwrap();
            let render_resource_manager = renderer.render_resource_manager().read().unwrap();
            if render_resource_manager.is_desynchronized() {
                DesynchronizedRenderResources::sync_mesh_buffers_with_meshes(
                    renderer.graphics_device(),
                    render_resource_manager
                        .desynchronized()
                        .mesh_buffer_managers
                        .lock()
                        .unwrap()
                        .as_mut(),
                    world
                        .scene().read().unwrap()
                        .mesh_repository().read().unwrap()
                        .meshes(),
                );
            }
            Ok(())
        })
    }
);

define_task!(
    SyncLightGPUBuffers,
    depends_on = [
        SyncLightsInStorage,
        BoundOmnidirectionalLightsAndBufferShadowCastingModelInstances,
        BoundUnidirectionalLightsAndBufferShadowCastingModelInstances
    ],
    execute_on = [RenderingTag],
    |world: &World| {
        with_debug_logging!("Synchronizing light GPU buffers"; {
            let renderer = world.renderer().read().unwrap();
            let render_resource_manager = renderer.render_resource_manager().read().unwrap();
            if render_resource_manager.is_desynchronized() {
                let scene = world.scene().read().unwrap();
                let light_storage = scene.light_storage().read().unwrap();
                DesynchronizedRenderResources::sync_light_buffers_with_light_storage(
                    renderer.graphics_device(),
                    render_resource_manager
                        .desynchronized()
                        .light_buffer_manager
                        .lock()
                        .unwrap()
                        .as_mut(),
                        &light_storage,
                        renderer.config(),
                );
            }
            Ok(())
        })
    }
);

define_task!(
    SyncInstanceFeatureBuffers,
    depends_on = [
        BufferVisibleModelInstances,
        BoundOmnidirectionalLightsAndBufferShadowCastingModelInstances,
        BoundUnidirectionalLightsAndBufferShadowCastingModelInstances
    ],
    execute_on = [RenderingTag],
    |world: &World| {
        with_debug_logging!("Synchronizing model instance feature GPU buffers"; {
            let renderer = world.renderer().read().unwrap();
            let render_resource_manager = renderer.render_resource_manager().read().unwrap();
            if render_resource_manager.is_desynchronized() {
                DesynchronizedRenderResources::sync_instance_feature_buffers_with_manager(
                    renderer.graphics_device(),
                    render_resource_manager
                        .desynchronized()
                        .instance_feature_buffer_managers
                        .lock()
                        .unwrap()
                        .as_mut(),
                    &mut world
                        .scene().read().unwrap()
                        .instance_feature_manager().write().unwrap(),
                );
            }
            Ok(())
        })
    }
);

/// Registers tasks for synchronizing render resources in the given task
/// scheduler.
pub fn register_render_resource_tasks(task_scheduler: &mut WorldTaskScheduler) -> Result<()> {
    task_scheduler.register_task(SyncCameraGPUBuffer)?;
    task_scheduler.register_task(SyncMeshGPUBuffers)?;
    task_scheduler.register_task(SyncLightGPUBuffers)?;
    task_scheduler.register_task(SyncInstanceFeatureBuffers)?;
    task_scheduler.register_task(SyncRenderResources)
}
