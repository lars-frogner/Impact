//! Tasks for physics.

use crate::{
    define_execution_tag, define_task,
    physics::PhysicsSimulator,
    scene::tasks::{SyncLightsInStorage, SyncSceneObjectTransforms},
    thread::ThreadPoolTaskErrors,
    window::EventLoopController,
    world::{tasks::WorldTaskScheduler, World},
};
use anyhow::Result;

use super::motion;

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
        SyncSceneObjectTransforms,
        SyncLightsInStorage
    ],
    execute_on = [PhysicsTag],
    |world: &World| {
        with_debug_logging!("Updating controlled entities"; {
            world.update_controlled_entities();
            Ok(())
        })
    }
);

define_task!(
    /// This [`Task`](crate::scheduling::Task) advances the physics simulation
    /// by one time step.
    [pub] AdvanceSimulation,
    depends_on = [
        SyncSceneObjectTransforms,
        SyncLightsInStorage,
        UpdateControlledEntities
    ],
    execute_on = [PhysicsTag],
    |world: &World| {
        with_debug_logging!("Advancing simulation"; {
            world.simulator().write().unwrap().advance_simulation(world.ecs_world());
            Ok(())
        })
    }
);

impl PhysicsSimulator {
    /// Identifies physics-related errors that need special handling in the
    /// given set of task errors and handles them.
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

/// Registers all tasks needed for physics in the given task scheduler.
pub fn register_physics_tasks(task_scheduler: &mut WorldTaskScheduler) -> Result<()> {
    task_scheduler.register_task(UpdateControlledEntities)?;
    task_scheduler.register_task(AdvanceSimulation)?;
    motion::tasks::register_motion_tasks(task_scheduler)
}
