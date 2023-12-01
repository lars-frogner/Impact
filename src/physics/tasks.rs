//! Tasks for physics.

use crate::{
    define_execution_tag, define_task,
    physics::{AdvanceOrientations, AdvancePositions, PhysicsSimulator},
    thread::ThreadPoolTaskErrors,
    window::EventLoopController,
    world::{World, WorldTaskScheduler},
};
use anyhow::Result;

define_execution_tag!(
    /// Execution tag for [`Task`](crate::scheduling::Task)s
    /// related to physics.
    [pub] PhysicsTag
);

define_task!(
    /// This [`Task`](crate::scheduling::Task) advances the physics simulation
    /// by one time step.
    [pub] AdvanceSimulation,
    depends_on = [],
    execute_on = [PhysicsTag],
    |world: &World| {
        with_debug_logging!("Advancing simulation"; {
            world.simulator().read().unwrap().advance_simulation(world.ecs_world());
            Ok(())
        })
    }
);

impl PhysicsSimulator {
    /// Registers all tasks needed for physics in the given
    /// task scheduler.
    pub fn register_tasks(task_scheduler: &mut WorldTaskScheduler) -> Result<()> {
        task_scheduler.register_task(AdvanceSimulation)?;
        task_scheduler.register_task(AdvancePositions)?;
        task_scheduler.register_task(AdvanceOrientations)
    }

    /// Identifies physics-related errors that need special
    /// handling in the given set of task errors and handles them.
    pub fn handle_task_errors(
        &self,
        task_errors: &ThreadPoolTaskErrors,
        event_loop_controller: &EventLoopController<'_>,
    ) {
        if task_errors.n_errors() > 0 {
            log::error!("Aborting due to fatal errors");
            event_loop_controller.exit();
        }
    }
}
