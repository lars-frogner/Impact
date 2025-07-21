//! Instrumentation for engine systems.

pub mod timing;

use serde::{Deserialize, Serialize};
use std::time::Duration;
use timing::TaskExecutionTimes;

/// Configuration for engine instrumentation features.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InstrumentationConfig {
    /// Whether to enable timing measurements for tasks.
    pub task_timing_enabled: bool,
}

/// Metrics gathered during engine execution.
#[derive(Clone, Debug, Default)]
pub struct EngineMetrics {
    /// The current smoothed frame duration calculated from recent frame timings.
    pub current_smooth_frame_duration: Duration,
    /// The execution times of all [`Task`](impact_scheduling::Task)s executed
    /// during the last frame the engine's task timer was enabled.
    pub last_task_execution_times: TaskExecutionTimes,
}

/// A generic tracker that maintains a rolling window of frame durations.
///
/// Uses a circular buffer to track the last N frame durations and compute
/// smoothed averages to reduce frame time jitter.
#[derive(Clone, Debug)]
pub struct GenericFrameDurationTracker<const N_FRAMES: usize> {
    last_frame_durations: [Duration; N_FRAMES],
    idx_of_oldest: usize,
}

/// A frame duration tracker that keeps track of the last 10 frames.
pub type FrameDurationTracker = GenericFrameDurationTracker<10>;

impl Default for InstrumentationConfig {
    fn default() -> Self {
        Self {
            task_timing_enabled: false,
        }
    }
}

impl EngineMetrics {
    /// The current smoothed FPS calculated from recent frame timings.
    pub fn current_smooth_fps(&self) -> u32 {
        frame_duration_to_fps(self.current_smooth_frame_duration)
    }
}

impl<const N_FRAMES: usize> GenericFrameDurationTracker<N_FRAMES> {
    /// Creates a new tracker initialized with the given frame duration.
    pub fn new(initial_frame_duration: Duration) -> Self {
        let last_frame_durations = [initial_frame_duration; N_FRAMES];
        Self {
            last_frame_durations,
            idx_of_oldest: 0,
        }
    }

    /// Computes the average frame duration from the tracked frames.
    pub fn compute_smooth_frame_duration(&self) -> Duration {
        let total_duration: Duration = self.last_frame_durations.iter().sum();
        total_duration.div_f64(N_FRAMES as f64)
    }

    /// Adds a new frame duration to the tracker, replacing the oldest entry.
    pub fn add_frame_duration(&mut self, frame_duration: Duration) {
        self.last_frame_durations[self.idx_of_oldest] = frame_duration;
        self.idx_of_oldest = (self.idx_of_oldest + 1) % N_FRAMES;
    }
}

impl<const N_FRAMES: usize> Default for GenericFrameDurationTracker<N_FRAMES> {
    fn default() -> Self {
        Self::new(fps_to_frame_duration(30))
    }
}

pub fn frame_duration_to_fps(duration: Duration) -> u32 {
    (1.0 / duration.as_secs_f64()).round() as u32
}

pub fn fps_to_frame_duration(fps: u32) -> Duration {
    Duration::from_secs_f64(1.0 / f64::from(fps))
}
