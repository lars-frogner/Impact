//! Tasks for physics.

use crate::{
    physics::{PhysicsSimulator, motion},
    runtime::tasks::{RuntimeContext, RuntimeTaskScheduler},
    scene::tasks::{SyncLightsInStorage, SyncSceneObjectTransformsAndFlags},
};
use anyhow::Result;
use impact_scheduling::{define_execution_tag, define_task};
use impact_thread::ThreadPoolTaskErrors;

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
                    engine.ecs_world(),
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

impl PhysicsSimulator {
    /// Identifies physics-related errors that need special handling in the
    /// given set of task errors and handles them.
    pub fn handle_task_errors(&self, _task_errors: &mut ThreadPoolTaskErrors) {}
}

/// Registers all tasks needed for physics in the given task scheduler.
pub fn register_physics_tasks(task_scheduler: &mut RuntimeTaskScheduler) -> Result<()> {
    task_scheduler.register_task(UpdateControlledEntities)?;
    task_scheduler.register_task(AdvanceSimulation)?;
    motion::tasks::register_motion_tasks(task_scheduler)
}
