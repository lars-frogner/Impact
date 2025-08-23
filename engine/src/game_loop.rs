//! Controller for the main loop driving simulation, rendering and UI.

use crate::instrumentation::{self, FrameDurationTracker};
use serde::{Deserialize, Serialize};
use std::{
    num::NonZeroU32,
    thread,
    time::{Duration, Instant},
};

/// A loop driving simulation and rendering in an
/// [`Engine`](crate::engine::Engine).
#[derive(Debug)]
pub struct GameLoopController {
    iteration: u64,
    frame_rate_tracker: FrameDurationTracker,
    start_time: Instant,
    config: GameLoopConfig,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct GameLoopConfig {
    max_fps: Option<NonZeroU32>,
    max_iterations: Option<u64>,
    state: GameLoopState,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum GameLoopState {
    #[default]
    Running,
    Paused,
    PauseAfterSingleIteration,
}

impl GameLoopController {
    pub fn new(config: GameLoopConfig) -> Self {
        let frame_rate_tracker = FrameDurationTracker::default();
        let start_time = Instant::now();
        Self {
            iteration: 0,
            frame_rate_tracker,
            start_time,
            config,
        }
    }

    pub fn state(&self) -> GameLoopState {
        self.config.state
    }

    pub fn should_perform_iteration(&self) -> bool {
        self.config.state != GameLoopState::Paused
    }

    pub fn iteration(&self) -> u64 {
        self.iteration
    }

    pub fn reached_max_iterations(&self) -> bool {
        self.config
            .max_iterations
            .is_some_and(|max_iterations| self.iteration >= max_iterations)
    }

    pub fn add_frame_duration(&mut self, frame_duration: Duration) {
        self.frame_rate_tracker.add_frame_duration(frame_duration);
    }

    pub fn compute_smooth_frame_duration(&self) -> Duration {
        self.frame_rate_tracker.compute_smooth_frame_duration()
    }

    pub fn increment_iteration(&mut self) {
        self.iteration += 1;
    }

    pub fn update_state_after_iteration(&mut self) {
        if self.config.state == GameLoopState::PauseAfterSingleIteration {
            self.config.state = GameLoopState::Paused;
        }
    }

    pub fn set_state(&mut self, state: GameLoopState) {
        self.config.state = state;
    }

    pub fn elapsed_time(&self) -> Duration {
        self.start_time.elapsed()
    }

    pub fn wait_for_target_frame_duration(&self, iter_start_time: Instant) -> Instant {
        let mut iter_end_time = Instant::now();
        if let Some(min_frame_duration) = self.config.min_frame_duration() {
            let target_end_time = iter_start_time + min_frame_duration;

            while iter_end_time < target_end_time {
                let remaining_duration = target_end_time - iter_end_time;

                if remaining_duration > Duration::from_millis(1) {
                    thread::sleep(remaining_duration - Duration::from_micros(500));
                } else {
                    // Busy-wait for the final microseconds
                    std::hint::spin_loop();
                }

                iter_end_time = Instant::now();
            }
        };
        iter_end_time
    }
}

impl GameLoopConfig {
    fn min_frame_duration(&self) -> Option<Duration> {
        self.max_fps
            .map(|fps| instrumentation::fps_to_frame_duration(fps.get()))
    }
}
