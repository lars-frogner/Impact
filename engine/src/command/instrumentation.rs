//! Commands for instrumentation.

use crate::{command::uils::ToActiveState, instrumentation::timing::TaskTimer};

#[derive(Clone, Debug)]
pub enum InstrumentationCommand {
    SetTaskTimings(ToActiveState),
}

pub fn set_task_timings(task_timer: &TaskTimer, to: ToActiveState) {
    let mut enabled = task_timer.enabled();
    if to.set(&mut enabled).changed {
        task_timer.set_enabled(enabled);
    }
}
