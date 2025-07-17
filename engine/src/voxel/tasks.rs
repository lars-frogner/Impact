//! Tasks related to voxels.

use crate::{
    gpu::rendering::tasks::RenderingTag,
    physics::tasks::{AdvanceSimulation, PhysicsTag},
    runtime::tasks::{RuntimeContext, RuntimeTaskScheduler},
    scene::tasks::UpdateSceneGroupToWorldTransforms,
};
use anyhow::Result;
use impact_scheduling::define_task;
use impact_voxel::interaction::{self, absorption, ecs::ECSVoxelObjectInteractionContext};

define_task!(
    /// This [`Task`](crate::scheduling::Task) applies each voxel absorber
    /// to the affected voxel objects.
    [pub] ApplyVoxelAbsorption,
    depends_on = [
        AdvanceSimulation,
        UpdateSceneGroupToWorldTransforms,
        SyncVoxelObjectMeshes
    ],
    execute_on = [PhysicsTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();
        instrument_engine_task!("Applying voxel absorbers", engine, {
            let mut entity_stager = engine.entity_stager().lock().unwrap();
            let ecs_world = engine.ecs_world().read().unwrap();
            let simulator = engine.simulator().read().unwrap();
            let mut rigid_body_manager = simulator.rigid_body_manager().write().unwrap();
            let scene = engine.scene().read().unwrap();
            let mut voxel_manager = scene.voxel_manager().write().unwrap();
            let scene_graph = scene.scene_graph().read().unwrap();

            let mut interaction_context = ECSVoxelObjectInteractionContext {
                entity_stager: &mut entity_stager,
                ecs_world: &ecs_world,
                scene_graph: &scene_graph,
            };

            absorption::apply_absorption(
                &mut interaction_context,
                &mut voxel_manager,
                &mut rigid_body_manager,
                simulator.time_step_duration(),
            );

            Ok(())
        })
    }
);

define_task!(
    /// This [`Task`](crate::scheduling::Task) updates the
    /// [`ModelTransform`](impact_geometry::ModelTransform) component of each
    /// voxel object to match its center of mass.
    [pub] SyncVoxelObjectModelTransforms,
    depends_on = [ApplyVoxelAbsorption],
    execute_on = [PhysicsTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();
        instrument_engine_task!("Synchronizing voxel object model transforms", engine, {
            let mut ecs_world = engine.ecs_world().write().unwrap();
            let scene = engine.scene().read().unwrap();
            let voxel_manager = scene.voxel_manager().read().unwrap();

            interaction::ecs::sync_voxel_object_model_transforms(
                &mut ecs_world,
                &voxel_manager.object_manager,
            );

            Ok(())
        })
    }
);

define_task!(
    /// This [`Task`](crate::scheduling::Task) recomputes invalidated mesh data
    /// for all meshed voxel objects.
    [pub] SyncVoxelObjectMeshes,
    depends_on = [],
    execute_on = [RenderingTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();
        instrument_engine_task!("Synchronizing voxel object meshes", engine, {
            let scene = engine.scene().read().unwrap();
            let mut voxel_manager = scene.voxel_manager().write().unwrap();

            let mut desynchronized = false;

            for voxel_object in voxel_manager.object_manager.voxel_objects_mut().values_mut() {
                voxel_object.sync_mesh_with_object(&mut desynchronized);
            }

            if desynchronized {
                engine
                    .renderer()
                    .read()
                    .unwrap()
                    .declare_render_resources_desynchronized();
            }
            Ok(())
        })
    }
);

/// Registers all tasks related to voxels in the given task scheduler.
pub fn register_voxel_tasks(task_scheduler: &mut RuntimeTaskScheduler) -> Result<()> {
    task_scheduler.register_task(ApplyVoxelAbsorption)?;
    task_scheduler.register_task(SyncVoxelObjectMeshes)?;
    task_scheduler.register_task(SyncVoxelObjectModelTransforms)
}
