//! Main loop driving simulation and rendering.

use crate::{
    define_execution_tag_set, engine::Engine, gpu::rendering::tasks::RenderingTag,
    physics::tasks::PhysicsTag, runtime::EventLoopController, scheduling::TaskScheduler,
    thread::ThreadPoolResult,
};
use serde::{Deserialize, Serialize};
use std::{
    num::NonZeroU32,
    thread,
    time::{Duration, Instant},
};

/// A loop driving simulation and rendering in an [`Engine`].
#[derive(Debug)]
pub struct GameLoop {
    frame_rate_tracker: FrameDurationTracker,
    start_time: Instant,
    previous_iter_end_time: Instant,
    config: GameLoopConfig,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameLoopConfig {
    max_fps: Option<NonZeroU32>,
}

define_execution_tag_set!(PHYSICS_AND_RENDERING_TAGS, [PhysicsTag, RenderingTag]);

#[derive(Clone, Debug)]
struct GenericFrameDurationTracker<const N_FRAMES: usize> {
    last_frame_durations: [Duration; N_FRAMES],
    idx_of_oldest: usize,
}

type FrameDurationTracker = GenericFrameDurationTracker<10>;

impl GameLoop {
    pub fn new(config: GameLoopConfig) -> Self {
        let frame_rate_tracker = FrameDurationTracker::default();
        let start_time = Instant::now();
        let previous_iter_end_time = start_time;
        Self {
            frame_rate_tracker,
            start_time,
            previous_iter_end_time,
            config,
        }
    }

    pub fn perform_iteration(
        &mut self,
        engine: &Engine,
        task_scheduler: &TaskScheduler<Engine>,
        event_loop_controller: &EventLoopController<'_>,
    ) -> ThreadPoolResult {
        engine.task_timer().clear();

        let execution_result = with_timing_info_logging!("Game loop iteration"; {
            task_scheduler.execute_and_wait(&PHYSICS_AND_RENDERING_TAGS)
        });

        if let Err(mut task_errors) = execution_result {
            engine.handle_task_errors(&mut task_errors, event_loop_controller);

            // Pass any unhandled errors to caller
            if task_errors.n_errors() > 0 {
                return Err(task_errors);
            }
        }

        engine.renderer().write().unwrap().present();

        let iter_end_time = self.wait_for_target_frame_duration();

        let iter_duration = iter_end_time - self.previous_iter_end_time;
        self.frame_rate_tracker.add_frame_duration(iter_duration);
        self.previous_iter_end_time = iter_end_time;

        let smooth_frame_duration = self.frame_rate_tracker.compute_smooth_frame_duration();

        engine
            .simulator()
            .write()
            .unwrap()
            .update_time_step_duration(&smooth_frame_duration);

        log::info!(
            "Completed game loop iteration after {:.1} ms (~{} FPS)",
            iter_duration.as_secs_f64() * 1e3,
            frame_duration_to_fps(smooth_frame_duration)
        );

        log::info!(
            "Elapsed time: {:.1} s",
            self.start_time.elapsed().as_secs_f64()
        );

        Ok(())
    }

    pub fn smooth_frame_duration(&self) -> Duration {
        self.frame_rate_tracker.compute_smooth_frame_duration()
    }

    pub fn smooth_fps(&self) -> u32 {
        frame_duration_to_fps(self.smooth_frame_duration())
    }

    pub fn elapsed_time(&self) -> Duration {
        self.start_time.elapsed()
    }

    fn wait_for_target_frame_duration(&self) -> Instant {
        let mut iter_end_time = Instant::now();
        if let Some(min_frame_duration) = self.config.min_frame_duration() {
            let target_end_time = self.previous_iter_end_time + min_frame_duration;

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

impl<const N_FRAMES: usize> GenericFrameDurationTracker<N_FRAMES> {
    fn new(initial_frame_duration: Duration) -> Self {
        let last_frame_durations = [initial_frame_duration; N_FRAMES];
        Self {
            last_frame_durations,
            idx_of_oldest: 0,
        }
    }

    fn compute_smooth_frame_duration(&self) -> Duration {
        let total_duration: Duration = self.last_frame_durations.iter().sum();
        total_duration.div_f64(N_FRAMES as f64)
    }

    fn add_frame_duration(&mut self, frame_duration: Duration) {
        self.last_frame_durations[self.idx_of_oldest] = frame_duration;
        self.idx_of_oldest = (self.idx_of_oldest + 1) % N_FRAMES;
    }
}

impl<const N_FRAMES: usize> Default for GenericFrameDurationTracker<N_FRAMES> {
    fn default() -> Self {
        Self::new(fps_to_frame_duration(30))
    }
}

fn frame_duration_to_fps(duration: Duration) -> u32 {
    (1.0 / duration.as_secs_f64()).round() as u32
}

fn fps_to_frame_duration(fps: u32) -> Duration {
    Duration::from_secs_f64(1.0 / f64::from(fps))
}

impl GameLoopConfig {
    fn min_frame_duration(&self) -> Option<Duration> {
        self.max_fps.map(|fps| fps_to_frame_duration(fps.get()))
    }
}
