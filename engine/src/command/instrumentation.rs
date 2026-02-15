//! Commands for instrumentation.

use crate::{command::uils::ToActiveState, rendering::RenderingSystem};
use impact_profiling::TaskTimer;

#[derive(Clone, Debug)]
pub enum InstrumentationAdminCommand {
    SetTaskTimings(ToActiveState),
    SetRenderPassTimings(ToActiveState),
}

pub fn set_task_timings(task_timer: &TaskTimer, to: ToActiveState) {
    let mut enabled = task_timer.enabled();
    if to.set(&mut enabled).changed {
        task_timer.set_enabled(enabled);
    }
}

pub fn set_render_pass_timings(renderer: &mut RenderingSystem, to: ToActiveState) {
    renderer.set_render_pass_timings_enabled(to.enabled());
}
