//! Utilities for performance profiling.

pub mod benchmarks;
pub mod criterion;
pub mod profile;

use std::{
    hint::black_box,
    time::{Duration, Instant},
};

pub trait Profiler {
    fn profile<T>(self, f: &mut impl FnMut() -> T);
}

#[derive(Clone, Debug)]
pub struct BasicProfiler {
    duration: Duration,
    delayer: Delayer,
}

#[derive(Clone, Debug)]
pub struct Delayer {
    program_start: Instant,
    delay: Duration,
}

impl BasicProfiler {
    pub fn new(duration: Duration, delayer: Delayer) -> Self {
        Self { duration, delayer }
    }
}

impl Profiler for BasicProfiler {
    fn profile<T>(self, f: &mut impl FnMut() -> T) {
        self.delayer.wait();
        let start = Instant::now();
        loop {
            black_box(f());

            if start.elapsed() > self.duration {
                break;
            }
        }
    }
}

impl Delayer {
    pub fn new(program_start: Instant, delay_seconds: f64) -> Self {
        Self {
            program_start,
            delay: Duration::from_secs_f64(delay_seconds),
        }
    }

    fn wait(self) {
        let remaining = self.delay.saturating_sub(self.program_start.elapsed());
        if remaining > Duration::ZERO {
            std::thread::sleep(remaining);
        }
    }
}
