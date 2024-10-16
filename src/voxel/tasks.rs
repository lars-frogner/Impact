//! Tasks related to voxels.

use crate::{
    application::{tasks::AppTaskScheduler, Application},
    define_task,
    gpu::rendering::{render_command::tasks::SyncRenderCommands, tasks::RenderingTag},
    physics::tasks::{AdvanceSimulation, PhysicsTag},
    voxel,
};
use anyhow::Result;

define_task!(
    /// This [`Task`](crate::scheduling::Task) applies each voxel-absorbing
    /// sphere to the affected voxel objects.
    [pub] ApplySphereVoxelAbsorption,
    depends_on = [AdvanceSimulation],
    execute_on = [PhysicsTag],
    |app: &Application| {
        with_debug_logging!("Applying voxel-absorbing spheres"; {
            let simulator = app.simulator().read().unwrap();
            let scene = app.scene().read().unwrap();
            let mut voxel_manager = scene.voxel_manager().write().unwrap();
            let ecs_world = app.ecs_world().read().unwrap();
            voxel::systems::apply_sphere_absorption(&simulator, &mut voxel_manager, &ecs_world);
            Ok(())
        })
    }
);

define_task!(
    /// This [`Task`](crate::scheduling::Task) recomputes invalidated mesh data
    /// for all meshed voxel objects.
    [pub] SyncVoxelObjectMeshes,
    depends_on = [ApplySphereVoxelAbsorption],
    execute_on = [RenderingTag],
    |app: &Application| {
        with_debug_logging!("Synchronizing voxel object meshes"; {
            let scene = app.scene().read().unwrap();
            let mut voxel_manager = scene.voxel_manager().write().unwrap();
            for voxel_object in voxel_manager.voxel_objects_mut().values_mut() {
                voxel_object.sync_mesh_with_object();
            }
            Ok(())
        })
    }
);

define_task!(
    /// This [`Task`](crate::scheduling::Task) handles meshing and entity
    /// creation for voxel objects that have been disconnected from a larger
    /// object.
    [pub] HandleDisconnectedVoxelObjects,
    // This may add entities, which is best done after rendering is initated
    // so as to not affect the render resources for the current frame
    depends_on = [SyncRenderCommands],
    execute_on = [RenderingTag],
    |app: &Application| {
        with_debug_logging!("Handling disconnected voxel objects"; {
            voxel::entity::handle_disconnected_voxel_objects(app)
        })
    }
);

define_task!(
    /// This [`Task`](crate::scheduling::Task) handles entity removal for voxel
    /// objects that have been emptied.
    [pub] HandleEmptiedVoxelObjects,
    // We need to handle disconnected objects first, since they need the parent
    // entity to still exist when they are created
    depends_on = [SyncRenderCommands, HandleDisconnectedVoxelObjects],
    execute_on = [RenderingTag],
    |app: &Application| {
        with_debug_logging!("Handling emptied voxel objects"; {
            voxel::entity::handle_emptied_voxel_objects(app)
        })
    }
);

/// Registers all tasks related to voxels in the given task scheduler.
pub fn register_voxel_tasks(task_scheduler: &mut AppTaskScheduler) -> Result<()> {
    task_scheduler.register_task(ApplySphereVoxelAbsorption)?;
    task_scheduler.register_task(SyncVoxelObjectMeshes)?;
    task_scheduler.register_task(HandleEmptiedVoxelObjects)?;
    task_scheduler.register_task(HandleDisconnectedVoxelObjects)
}
