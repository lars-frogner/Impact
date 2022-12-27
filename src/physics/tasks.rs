//! Tasks for physics.

use crate::{
    define_execution_tag,
    physics::{AdvancePositions, PhysicsSimulator},
    thread::ThreadPoolTaskErrors,
    window::ControlFlow,
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
        task_scheduler.register_task(AdvancePositions)
    }

    /// Identifies physics-related errors that need special
    /// handling in the given set of task errors and handles them.
    pub fn handle_task_errors(
        &self,
        _task_errors: &mut ThreadPoolTaskErrors,
        _control_flow: &mut ControlFlow<'_>,
    ) {
    }
}
