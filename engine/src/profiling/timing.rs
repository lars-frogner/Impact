//! Measuring execution time.

use impact_math::ConstStringHash64;
use std::{
    collections::HashMap,
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

#[derive(Debug)]
pub struct TaskTimer {
    enabled: AtomicBool,
    task_execution_times:
        Mutex<HashMap<TimedTaskID, Duration, nohash_hasher::BuildNoHashHasher<TimedTaskID>>>,
}

pub type TimedTaskID = ConstStringHash64;

impl TaskTimer {
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled: AtomicBool::new(enabled),
            task_execution_times: Mutex::new(HashMap::default()),
        }
    }

    pub fn enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
    }

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
            .insert(task_id, elapsed);

        result
    }

    pub fn clear(&self) {
        self.task_execution_times.lock().unwrap().clear();
    }
}
