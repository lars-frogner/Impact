//! Tasks for synchronizing GPU buffers.

use super::DesynchronizedRenderResources;
use crate::{
    define_task,
    engine::{Engine, tasks::EngineTaskScheduler},
    gpu::rendering::tasks::RenderingTag,
    scene::tasks::{
        BoundOmnidirectionalLightsAndBufferShadowCastingModelInstances,
        BoundUnidirectionalLightsAndBufferShadowCastingModelInstances, BufferVisibleModelInstances,
        SyncLightsInStorage, SyncSceneCameraViewTransform,
    },
    voxel::tasks::SyncVoxelObjectMeshes,
};
use anyhow::Result;

define_task!(
    /// This [`Task`](crate::scheduling::Task) performs any required
    /// updates for keeping the [`World`]s render resources in sync with
    /// the source data.
    ///
    /// GPU resources whose source data no longer exists will
    /// be removed, and missing render resources for new source
    /// data will be created.
    [pub] SyncRenderResources,
    depends_on = [
        SyncMinorResources,
        SyncMeshGPUBuffers,
        SyncVoxelObjectGPUBuffers,
        SyncLightGPUBuffers,
        SyncInstanceFeatureBuffers
    ],
    execute_on = [RenderingTag],
    |engine: &Engine| {
        instrument_engine_task!("Completing synchronization of render resources", engine, {
            let renderer = engine.renderer().read().unwrap();
            let mut render_resource_manager = renderer.render_resource_manager().write().unwrap();
            render_resource_manager.declare_synchronized();
            Ok(())
        })
    }
);

define_task!(
    SyncMinorResources,
    depends_on = [SyncSceneCameraViewTransform],
    execute_on = [RenderingTag],
    |engine: &Engine| {
        instrument_engine_task!("Synchronizing camera and skybox GPU resources", engine, {
            let renderer = engine.renderer().read().unwrap();
            let scene = engine.scene().read().unwrap();
            let render_resource_manager = renderer.render_resource_manager().read().unwrap();
            if render_resource_manager.is_desynchronized() {
                DesynchronizedRenderResources::sync_camera_buffer_with_scene_camera(
                    renderer.graphics_device(),
                    render_resource_manager
                        .desynchronized()
                        .camera_buffer_manager
                        .lock()
                        .unwrap()
                        .as_mut(),
                    scene.scene_camera().read().unwrap().as_ref(),
                );
                DesynchronizedRenderResources::sync_skybox_resources_with_scene_skybox(
                    renderer.graphics_device(),
                    &engine.assets().read().unwrap(),
                    render_resource_manager
                        .desynchronized()
                        .skybox_resource_manager
                        .lock()
                        .unwrap()
                        .as_mut(),
                    scene.skybox().read().unwrap().as_ref(),
                )?;
            }
            Ok(())
        })
    }
);

define_task!(
    SyncMeshGPUBuffers,
    depends_on = [],
    execute_on = [RenderingTag],
    |engine: &Engine| {
        instrument_engine_task!("Synchronizing mesh GPU buffers", engine, {
            let renderer = engine.renderer().read().unwrap();
            let render_resource_manager = renderer.render_resource_manager().read().unwrap();
            if render_resource_manager.is_desynchronized() {
                DesynchronizedRenderResources::sync_triangle_mesh_buffers_with_triangle_meshes(
                    renderer.graphics_device(),
                    render_resource_manager
                        .desynchronized()
                        .triangle_mesh_buffer_managers
                        .lock()
                        .unwrap()
                        .as_mut(),
                    engine
                        .scene()
                        .read()
                        .unwrap()
                        .mesh_repository()
                        .read()
                        .unwrap()
                        .triangle_meshes(),
                );
                DesynchronizedRenderResources::sync_line_segment_mesh_buffers_with_line_segment_meshes(
                    renderer.graphics_device(),
                    render_resource_manager
                        .desynchronized()
                        .line_segment_mesh_buffer_managers
                        .lock()
                        .unwrap()
                        .as_mut(),
                    engine
                        .scene()
                        .read()
                        .unwrap()
                        .mesh_repository()
                        .read()
                        .unwrap()
                        .line_segment_meshes(),
                );
            }
            Ok(())
        })
    }
);

define_task!(
    SyncVoxelObjectGPUBuffers,
    depends_on = [SyncVoxelObjectMeshes],
    execute_on = [RenderingTag],
    |engine: &Engine| {
        instrument_engine_task!("Synchronizing voxel object GPU buffers", engine, {
            let renderer = engine.renderer().read().unwrap();
            let render_resource_manager = renderer.render_resource_manager().read().unwrap();
            if render_resource_manager.is_desynchronized() {
                DesynchronizedRenderResources::sync_voxel_resources_with_voxel_manager(
                    renderer.graphics_device(),
                    engine.assets(),
                    render_resource_manager
                        .desynchronized()
                        .voxel_resource_managers
                        .lock()
                        .unwrap()
                        .as_mut(),
                    &mut engine
                        .scene()
                        .read()
                        .unwrap()
                        .voxel_manager()
                        .write()
                        .unwrap(),
                )?;
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
    |engine: &Engine| {
        instrument_engine_task!("Synchronizing light GPU buffers", engine, {
            let renderer = engine.renderer().read().unwrap();
            let render_resource_manager = renderer.render_resource_manager().read().unwrap();
            if render_resource_manager.is_desynchronized() {
                let scene = engine.scene().read().unwrap();
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
                    renderer.shadow_mapping_config(),
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
    |engine: &Engine| {
        instrument_engine_task!(
            "Synchronizing model instance feature GPU buffers",
            engine,
            {
                let renderer = engine.renderer().read().unwrap();
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
                        &mut engine
                            .scene()
                            .read()
                            .unwrap()
                            .instance_feature_manager()
                            .write()
                            .unwrap(),
                    );
                }
                Ok(())
            }
        )
    }
);

/// Registers tasks for synchronizing render resources in the given task
/// scheduler.
pub fn register_render_resource_tasks(task_scheduler: &mut EngineTaskScheduler) -> Result<()> {
    task_scheduler.register_task(SyncMinorResources)?;
    task_scheduler.register_task(SyncMeshGPUBuffers)?;
    task_scheduler.register_task(SyncVoxelObjectGPUBuffers)?;
    task_scheduler.register_task(SyncLightGPUBuffers)?;
    task_scheduler.register_task(SyncInstanceFeatureBuffers)?;
    task_scheduler.register_task(SyncRenderResources)
}
