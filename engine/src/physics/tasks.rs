//! Tasks for physics.

use crate::{
    physics::systems::synchronize_rigid_body_components,
    runtime::tasks::{RuntimeContext, RuntimeTaskScheduler},
    scene::tasks::{SyncLightsInStorage, SyncSceneObjectTransformsAndFlags},
};
use anyhow::Result;
use impact_scheduling::{define_execution_tag, define_task};

define_execution_tag!(
    /// Execution tag for [`Task`](crate::scheduling::Task)s
    /// related to physics.
    [pub] PhysicsTag
);

define_task!(
    /// This [`Task`](crate::scheduling::Task) updates the orientations and
    /// motion of all controlled entities.
    [pub] UpdateControlledEntities,
    depends_on = [
        SyncSceneObjectTransformsAndFlags,
        SyncLightsInStorage
    ],
    execute_on = [PhysicsTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();
        instrument_engine_task!("Updating controlled entities", engine, {
            engine.update_controlled_entities();
            Ok(())
        })
    }
);

define_task!(
    /// This [`Task`](crate::scheduling::Task) advances the physics simulation
    /// by one time step.
    [pub] AdvanceSimulation,
    depends_on = [
        SyncSceneObjectTransformsAndFlags,
        SyncLightsInStorage,
        UpdateControlledEntities
    ],
    execute_on = [PhysicsTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();
        instrument_engine_task!("Advancing simulation", engine, {
            engine.simulator()
                .write()
                .unwrap()
                .advance_simulation(
                    &engine.scene()
                        .read()
                        .unwrap()
                        .voxel_manager()
                        .read()
                        .unwrap()
                        .object_manager
                );
            Ok(())
        })
    }
);

define_task!(
    /// This [`Task`](crate::scheduling::Task) updates the
    /// [`ReferenceFrame`](impact_geometry::ReferenceFrame) and
    /// [`Motion`](impact_physics::quantities::Motion) components of entities
    /// with the
    /// [`DynamicRigidBodyID`](impact_physics::rigid_body::DynamicRigidBodyID)
    /// or
    /// [`KinematicRigidBodyID`](impact_physics::rigid_body::KinematicRigidBodyID)
    /// component to match the current state of the rigid body.
    [pub] SyncRigidBodyComponents,
    depends_on = [AdvanceSimulation],
    execute_on = [PhysicsTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();
        instrument_engine_task!("Synchronizing rigid body components", engine, {
            let ecs_world = engine.ecs_world().read().unwrap();
            let simulator = engine.simulator().read().unwrap();
            let rigid_body_manager = simulator.rigid_body_manager().read().unwrap();
            synchronize_rigid_body_components(&ecs_world, &rigid_body_manager);
            Ok(())
        })
    }
);

/// Registers all tasks needed for physics in the given task scheduler.
pub fn register_physics_tasks(task_scheduler: &mut RuntimeTaskScheduler) -> Result<()> {
    task_scheduler.register_task(UpdateControlledEntities)?;
    task_scheduler.register_task(AdvanceSimulation)?;
    task_scheduler.register_task(SyncRigidBodyComponents)
}
