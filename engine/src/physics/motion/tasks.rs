//! Tasks related to motion.

use crate::{
    define_task,
    engine::{Engine, tasks::EngineTaskScheduler},
    physics::{
        motion,
        tasks::{AdvanceSimulation, PhysicsTag},
    },
};
use anyhow::Result;

define_task!(
    /// This [`Task`](crate::scheduling::Task) logs the kinetic energy of each
    /// applicable rigid body.
    [pub] LogKineticEnergy,
    depends_on = [AdvanceSimulation],
    execute_on = [PhysicsTag],
    |engine: &Engine| {
        instrument_engine_task!("Logging kinetic energy", engine, {
            let ecs_world = engine.ecs_world().read().unwrap();
            motion::systems::log_kinetic_energies(&ecs_world);
            Ok(())
        })
    }
);

define_task!(
    /// This [`Task`](crate::scheduling::Task) logs the linear and angular
    /// momentum of each applicable rigid body.
    [pub] LogMomentum,
    depends_on = [AdvanceSimulation],
    execute_on = [PhysicsTag],
    |engine: &Engine| {
        instrument_engine_task!("Logging momentum", engine, {
            let ecs_world = engine.ecs_world().read().unwrap();
            motion::systems::log_momenta(&ecs_world);
            Ok(())
        })
    }
);

/// Registers all tasks related to motion in the given task scheduler.
pub fn register_motion_tasks(task_scheduler: &mut EngineTaskScheduler) -> Result<()> {
    task_scheduler.register_task(LogKineticEnergy)?;
    task_scheduler.register_task(LogMomentum)
}
