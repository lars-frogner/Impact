//! Tasks related to voxels.

use crate::{
    application::{tasks::AppTaskScheduler, Application},
    define_task,
    gpu::rendering::{render_command::tasks::SyncRenderCommands, tasks::RenderingTag},
    physics::tasks::{AdvanceSimulation, PhysicsTag},
    scene::RenderResourcesDesynchronized,
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
    depends_on = [],
    execute_on = [RenderingTag],
    |app: &Application| {
        with_debug_logging!("Synchronizing voxel object meshes"; {
            let scene = app.scene().read().unwrap();
            let mut voxel_manager = scene.voxel_manager().write().unwrap();

            let mut desynchronized = RenderResourcesDesynchronized::No;

            for voxel_object in voxel_manager.object_manager.voxel_objects_mut().values_mut() {
                voxel_object.sync_mesh_with_object(&mut desynchronized);
            }

            if desynchronized.is_yes() {
                app
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
    |app: &Application| {
        with_debug_logging!("Handling staged voxel objects"; {
            voxel::entity::handle_staged_voxel_objects(app)
        })
    }
);

define_task!(
    /// This [`Task`](crate::scheduling::Task) handles entity removal for voxel
    /// objects that have been emptied.
    [pub] HandleEmptiedVoxelObjects,
    depends_on = [SyncRenderCommands, ApplySphereVoxelAbsorption],
    execute_on = [PhysicsTag, RenderingTag],
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
    task_scheduler.register_task(HandleStagedVoxelObjects)
}
