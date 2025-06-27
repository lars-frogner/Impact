//! Tasks for the user interface.

use crate::runtime::tasks::{RuntimeContext, RuntimeTaskScheduler};
use anyhow::Result;
use impact_scheduling::{define_execution_tag, define_task};

define_execution_tag!(
    /// Execution tag for [`Task`](crate::scheduling::Task)s
    /// related to the user interface.
    [pub] UserInterfaceTag
);

define_task!(
    /// This [`Task`](crate::scheduling::Task) handles all UI logic and
    /// processes and stores the output.
    ///
    /// Since running the UI logic may change configuration parameters in the
    /// engine, this task must run before other task that may depend on those
    /// parameters.
    [pub] ProcessUserInterface,
    depends_on = [],
    execute_on = [UserInterfaceTag],
    |ctx: &RuntimeContext| {
        let engine = ctx.engine();
        instrument_engine_task!("Processing user interface", engine, {
            ctx.user_interface().process(engine)
        })
    }
);

/// Registers all tasks needed for the GUI in the given task scheduler.
pub fn register_user_interface_tasks(task_scheduler: &mut RuntimeTaskScheduler) -> Result<()> {
    task_scheduler.register_task(ProcessUserInterface)
}
