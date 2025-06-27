//! Main loop driving simulation and rendering.

use crate::{
    define_execution_tag_set,
    engine::Engine,
    gpu::rendering::tasks::RenderingTag,
    instrumentation::{self, FrameDurationTracker},
    physics::tasks::PhysicsTag,
    runtime::tasks::RuntimeTaskScheduler,
    ui::tasks::UserInterfaceTag,
};
use impact_thread::ThreadPoolResult;
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

define_execution_tag_set!(ALL_SYSTEMS, [PhysicsTag, RenderingTag, UserInterfaceTag]);

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
        task_scheduler: &RuntimeTaskScheduler,
    ) -> ThreadPoolResult {
        let execution_result = impact_log::with_timing_info_logging!("Game loop iteration"; {
            task_scheduler.execute_and_wait(&ALL_SYSTEMS)
        });

        if let Err(mut task_errors) = execution_result {
            engine.handle_task_errors(&mut task_errors);

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

        engine.gather_metrics_after_completed_frame(smooth_frame_duration);

        impact_log::info!(
            "Completed game loop iteration after {:.1} ms (~{} FPS)",
            iter_duration.as_secs_f64() * 1e3,
            instrumentation::frame_duration_to_fps(smooth_frame_duration)
        );

        impact_log::info!(
            "Elapsed time: {:.1} s",
            self.start_time.elapsed().as_secs_f64()
        );

        Ok(())
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

impl GameLoopConfig {
    fn min_frame_duration(&self) -> Option<Duration> {
        self.max_fps
            .map(|fps| instrumentation::fps_to_frame_duration(fps.get()))
    }
}
