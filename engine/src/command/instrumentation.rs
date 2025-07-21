//! Commands for instrumentation.

use crate::{command::uils::ToActiveState, instrumentation::timing::TaskTimer};
use roc_integration::roc;

#[roc(parents = "Command")]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InstrumentationCommand {
    SetTaskTimings(ToActiveState),
}

pub fn set_task_timings(task_timer: &TaskTimer, to: ToActiveState) {
    let mut enabled = task_timer.enabled();
    if to.set(&mut enabled).changed {
        task_timer.set_enabled(enabled);
    }
}
