//! Basic execution time measurement.

use parking_lot::Mutex;
use std::{
    collections::HashMap,
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

/// A basic timer for various tasks.
#[derive(Debug)]
pub struct TaskTimer {
    enabled: AtomicBool,
    timed_task_manager: Mutex<TimedTaskManager>,
}

/// Label and duration of a timed task.
#[derive(Clone, Copy, Debug)]
pub struct TimedTask {
    pub label: &'static str,
    pub duration: Duration,
}

#[derive(Debug)]
struct TimedTaskManager {
    recorded_tasks: Vec<TimedTask>,
    total_durations_by_label: HashMap<&'static str, Duration>,
}

impl TaskTimer {
    /// Creates new timer that is initially enabled or disabled.
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled: AtomicBool::new(enabled),
            timed_task_manager: Mutex::new(TimedTaskManager::new()),
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
    /// under the given task label. If there is already a measurement with
    /// the same label, the times will be aggregated.
    pub fn time<R>(&self, label: &'static str, f: impl FnOnce() -> R) -> R {
        if !self.enabled() {
            return f();
        }
        let start = Instant::now();

        let result = f();

        let elapsed = start.elapsed();

        self.timed_task_manager.lock().add_recorded_task(TimedTask {
            label,
            duration: elapsed,
        });

        result
    }

    /// If the timer is enabled, moves all timing measurements done by
    /// [`Self::time`] since this function was last called into the given
    /// `Vec`. Any existing values will be overwritten.
    pub fn report_task_execution_times(&self, times: &mut Vec<TimedTask>) {
        if !self.enabled() {
            return;
        }
        self.timed_task_manager.lock().report_tasks(times);
    }
}

impl TimedTaskManager {
    fn new() -> Self {
        Self {
            recorded_tasks: Vec::new(),
            total_durations_by_label: HashMap::new(),
        }
    }

    fn add_recorded_task(&mut self, task: TimedTask) {
        self.recorded_tasks.push(task);
    }

    fn report_tasks(&mut self, times: &mut Vec<TimedTask>) {
        times.clear();

        // Aggregate the durations for all tasks with the same label, then write
        // the aggregated durations into the output vector in the same order as
        // they were registered, leaving the list of recorded tasks cleared for
        // new measurements

        self.total_durations_by_label.clear();
        for task in &self.recorded_tasks {
            self.total_durations_by_label
                .entry(task.label)
                .and_modify(|duration| *duration += task.duration)
                .or_insert(task.duration);
        }

        for task in self.recorded_tasks.drain(..) {
            if let Some(duration) = self.total_durations_by_label.remove(task.label) {
                times.push(TimedTask {
                    label: task.label,
                    duration,
                });
            }
        }
    }
}
