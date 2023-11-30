//! Tasks for physics.

use crate::{
    define_execution_tag,
    physics::{AdvanceOrientations, AdvancePositions, PhysicsSimulator, SyncRigidBodyMotion},
    thread::ThreadPoolTaskErrors,
    window::EventLoopController,
    world::WorldTaskScheduler,
};
use anyhow::Result;

define_execution_tag!(
    /// Execution tag for [`Task`](crate::scheduling::Task)s
    /// related to physics.
    [pub] PhysicsTag
);

impl PhysicsSimulator {
    /// Registers all tasks needed for physics in the given
    /// task scheduler.
    pub fn register_tasks(task_scheduler: &mut WorldTaskScheduler) -> Result<()> {
        task_scheduler.register_task(AdvancePositions)?;
        task_scheduler.register_task(AdvanceOrientations)?;
        task_scheduler.register_task(SyncRigidBodyMotion)
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
