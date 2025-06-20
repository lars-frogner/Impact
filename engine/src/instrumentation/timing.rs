//! Measuring execution time.

use impact_math::ConstStringHash64;
use std::{
    mem,
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

/// A timer for various tasks that stores the time measurements.
#[derive(Debug)]
pub struct TaskTimer {
    enabled: AtomicBool,
    task_execution_times: Mutex<TaskExecutionTimes>,
}

pub type TaskExecutionTimes = Vec<(TimedTaskID, Duration)>;

/// An ID for a task that can be timed.
pub type TimedTaskID = ConstStringHash64;

impl TaskTimer {
    /// Creates new timer that is initially enabled or disabled.
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled: AtomicBool::new(enabled),
            task_execution_times: Mutex::new(Vec::new()),
        }
    }

    pub fn enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
    }

    /// Executes the given closure and returns the result. If the timer is
    /// enabled, the time it took for the closure to execute will be stored
    /// under the given task ID.
    pub fn time<R>(&self, task_id: TimedTaskID, f: impl FnOnce() -> R) -> R {
        if !self.enabled() {
            return f();
        }
        let start = Instant::now();

        let result = f();

        let elapsed = start.elapsed();

        self.task_execution_times
            .lock()
            .unwrap()
            .push((task_id, elapsed));

        result
    }

    /// If the timer is enabled, moves all timing measurements done by
    /// [`Self::time`] since this function was last called into the given
    /// [`TaskExecutionTimes`]. Any existing values will be overwritten.
    pub fn report_task_execution_times(&self, times: &mut TaskExecutionTimes) {
        if !self.enabled() {
            return;
        }
        times.clear();
        let mut task_execution_times = self.task_execution_times.lock().unwrap();
        mem::swap(&mut *task_execution_times, times);
    }
}
