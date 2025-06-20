//! Tasks related to voxels.

use crate::{
    define_task,
    gpu::rendering::{render_command::tasks::SyncRenderCommands, tasks::RenderingTag},
    physics::tasks::{AdvanceSimulation, PhysicsTag},
    runtime::tasks::{RuntimeContext, RuntimeTaskScheduler},
    scene::{RenderResourcesDesynchronized, tasks::UpdateSceneGroupToWorldTransforms},
    voxel,
};
use anyhow::Result;

define_task!(
    /// This [`Task`](crate::scheduling::Task) applies each voxel absorber
    /// to the affected voxel objects.
    [pub] ApplySphereVoxelAbsorption,
    depends_on = [AdvanceSimulation, UpdateSceneGroupToWorldTransforms],
    execute_on = [PhysicsTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();
        instrument_engine_task!("Applying voxel absorbers", engine, {
            let simulator = engine.simulator().read().unwrap();
            let scene = engine.scene().read().unwrap();
            let mut voxel_manager = scene.voxel_manager().write().unwrap();
            let scene_graph = scene.scene_graph().read().unwrap();
            let ecs_world = engine.ecs_world().read().unwrap();
            voxel::systems::apply_absorption(&simulator, &mut voxel_manager, &scene_graph, &ecs_world);
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

            let mut desynchronized = RenderResourcesDesynchronized::No;

            for voxel_object in voxel_manager.object_manager.voxel_objects_mut().values_mut() {
                voxel_object.sync_mesh_with_object(&mut desynchronized);
            }

            if desynchronized.is_yes() {
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

define_task!(
    /// This [`Task`](crate::scheduling::Task) handles meshing and entity
    /// creation for voxel objects that have been staged for realization.
    [pub] HandleStagedVoxelObjects,
    depends_on = [SyncRenderCommands, ApplySphereVoxelAbsorption],
    execute_on = [PhysicsTag, RenderingTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();
        instrument_engine_task!("Handling staged voxel objects", engine, {
            voxel::entity::handle_staged_voxel_objects(engine)
        })
    }
);

define_task!(
    /// This [`Task`](crate::scheduling::Task) handles entity removal for voxel
    /// objects that have been emptied.
    [pub] HandleEmptiedVoxelObjects,
    depends_on = [SyncRenderCommands, ApplySphereVoxelAbsorption],
    execute_on = [PhysicsTag, RenderingTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();
        instrument_engine_task!("Handling emptied voxel objects", engine, {
            voxel::entity::handle_emptied_voxel_objects(engine)
        })
    }
);

/// Registers all tasks related to voxels in the given task scheduler.
pub fn register_voxel_tasks(task_scheduler: &mut RuntimeTaskScheduler) -> Result<()> {
    task_scheduler.register_task(ApplySphereVoxelAbsorption)?;
    task_scheduler.register_task(SyncVoxelObjectMeshes)?;
    task_scheduler.register_task(HandleEmptiedVoxelObjects)?;
    task_scheduler.register_task(HandleStagedVoxelObjects)
}
