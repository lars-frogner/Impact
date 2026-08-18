//! Commands for instrumentation.

use crate::{command::utils::ToActiveState, gpu::GraphicsSystem};
use impact_profiling::TaskTimer;

#[derive(Clone, Debug)]
pub enum InstrumentationAdminCommand {
    SetTaskTimings(ToActiveState),
    SetGPUPassTimings(ToActiveState),
}

pub fn set_task_timings(task_timer: &TaskTimer, to: ToActiveState) {
    let mut enabled = task_timer.enabled();
    if to.set(&mut enabled).changed {
        task_timer.set_enabled(enabled);
    }
}

pub fn set_gpu_pass_timings(graphics_system: &mut GraphicsSystem, to: ToActiveState) {
    graphics_system.set_gpu_pass_timings_enabled(to.enabled());
}
