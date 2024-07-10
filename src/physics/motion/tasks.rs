//! Tasks related to motion.

use crate::{
    application::{tasks::AppTaskScheduler, Application},
    define_task,
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
    |app: &Application| {
        with_debug_logging!("Logging kinetic energy"; {
            let ecs_world = app.ecs_world().read().unwrap();
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
    |app: &Application| {
        with_debug_logging!("Logging momentum"; {
            let ecs_world = app.ecs_world().read().unwrap();
            motion::systems::log_momenta(&ecs_world);
            Ok(())
        })
    }
);

/// Registers all tasks related to motion in the given task scheduler.
pub fn register_motion_tasks(task_scheduler: &mut AppTaskScheduler) -> Result<()> {
    task_scheduler.register_task(LogKineticEnergy)?;
    task_scheduler.register_task(LogMomentum)
}
