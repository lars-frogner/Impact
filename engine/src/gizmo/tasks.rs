//! Tasks for gizmo management.

use crate::{
    define_task,
    engine::{Engine, tasks::EngineTaskScheduler},
    gizmo::{self, GizmoSet, GizmoType},
    gpu::rendering::tasks::RenderingTag,
    scene::tasks::{BufferVisibleModelInstances, ClearModelInstanceBuffers, SyncLightsInStorage},
};
use anyhow::Result;

define_task!(
    /// This [`Task`](crate::scheduling::Task) updates the appropriate gizmo
    /// visibility flags for all applicable entities based on which gizmos have
    /// been newly configured to be globally visible or hidden.
    [pub] UpdateVisibilityFlagsForGizmos,
    depends_on = [],
    execute_on = [RenderingTag],
    |engine: &Engine| {
        instrument_engine_task!("Updating visibility flags for gizmos", engine, {
            let mut gizmo_manager = engine.gizmo_manager().write().unwrap();
            if !gizmo_manager.global_visibility_changed_for_any_of_gizmos(GizmoSet::all()) {
                return Ok(());
            }

            let ecs_world = engine.ecs_world().read().unwrap();

            for gizmo in GizmoType::all() {
                if gizmo_manager.global_visibility_changed_for_any_of_gizmos(gizmo.as_set()) {
                    gizmo::systems::update_visibility_flags_for_gizmo(
                        &ecs_world,
                        &gizmo_manager,
                        gizmo,
                    );
                }
            }
            gizmo_manager.declare_visibilities_synchronized();
            Ok(())
        })
    }
);

define_task!(
    /// This [`Task`](crate::scheduling::Task) finds entities for which gizmos
    /// should be displayed and writes their model-view transforms to the
    /// dedicated buffers for the gizmos.
    [pub] BufferTransformsForGizmos,
    depends_on = [
        UpdateVisibilityFlagsForGizmos,
        ClearModelInstanceBuffers,
        BufferVisibleModelInstances,
        SyncLightsInStorage
    ],
    execute_on = [RenderingTag],
    |engine: &Engine| {
        instrument_engine_task!("Buffering transforms for gizmos", engine, {
            let ecs_world = engine.ecs_world().read().unwrap();
            let current_frame_count = engine.renderer().read().unwrap().current_frame_count();
            let scene = engine.scene().read().unwrap();
            let mut instance_feature_manager = scene.instance_feature_manager().write().unwrap();
            let scene_graph = scene.scene_graph().read().unwrap();
            let light_storage = scene.light_storage().read().unwrap();
            let scene_camera = scene.scene_camera().read().unwrap();

            gizmo::systems::buffer_transforms_for_gizmos(
                &ecs_world,
                &mut instance_feature_manager,
                &scene_graph,
                &light_storage,
                scene_camera.as_ref(),
                current_frame_count,
            );
            Ok(())
        })
    }
);

pub fn register_gizmo_tasks(task_scheduler: &mut EngineTaskScheduler) -> Result<()> {
    task_scheduler.register_task(UpdateVisibilityFlagsForGizmos)?;
    task_scheduler.register_task(BufferTransformsForGizmos)
}
