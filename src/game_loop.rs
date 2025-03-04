//! Main loop driving simulation and rendering.

use crate::{
    application::{Application, tasks::AppTaskScheduler},
    define_execution_tag_set,
    gpu::rendering::tasks::RenderingTag,
    physics::tasks::PhysicsTag,
    thread::ThreadPoolResult,
    window::{EventLoopController, HandlingResult, InputHandler, Window, WindowEvent},
};
use anyhow::Result;
use std::{
    num::{NonZeroU32, NonZeroUsize},
    sync::Arc,
    thread,
    time::{Duration, Instant},
};
use winit::event::DeviceEvent;

/// A loop driving simulation and rendering in an [`Application`].
#[derive(Debug)]
pub struct GameLoop {
    app: Arc<Application>,
    task_scheduler: AppTaskScheduler,
    input_handler: InputHandler,
    frame_rate_tracker: FrameDurationTracker,
    start_time: Instant,
    previous_iter_end_time: Instant,
    config: GameLoopConfig,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GameLoopConfig {
    n_worker_threads: NonZeroUsize,
    max_fps: Option<NonZeroU32>,
}

define_execution_tag_set!(PHYSICS_AND_RENDERING_TAGS, [PhysicsTag, RenderingTag]);
define_execution_tag_set!(RENDERING_TAGS, [RenderingTag]);

#[derive(Clone, Debug)]
struct GenericFrameDurationTracker<const N_FRAMES: usize> {
    last_frame_durations: [Duration; N_FRAMES],
    idx_of_oldest: usize,
}

type FrameDurationTracker = GenericFrameDurationTracker<10>;

impl GameLoop {
    pub fn new(
        app: Application,
        input_handler: InputHandler,
        config: GameLoopConfig,
    ) -> Result<Self> {
        let (world, task_scheduler) = app.create_task_scheduler(config.n_worker_threads)?;

        world.perform_setup_for_game_loop();

        let frame_rate_tracker = FrameDurationTracker::default();
        let start_time = Instant::now();
        let previous_iter_end_time = start_time;

        Ok(Self {
            app: world,
            task_scheduler,
            input_handler,
            frame_rate_tracker,
            start_time,
            previous_iter_end_time,
            config,
        })
    }

    pub fn app(&self) -> &Application {
        self.app.as_ref()
    }

    pub fn window(&self) -> &Window {
        self.app().window()
    }

    pub fn handle_window_event(
        &self,
        event_loop_controller: &EventLoopController<'_>,
        event: &WindowEvent,
    ) -> Result<HandlingResult> {
        self.input_handler
            .handle_window_event(&self.app, event_loop_controller, event)
    }

    pub fn handle_device_event(
        &self,
        event_loop_controller: &EventLoopController<'_>,
        event: &DeviceEvent,
    ) -> Result<HandlingResult> {
        self.input_handler
            .handle_device_event(&self.app, event_loop_controller, event)
    }

    pub fn resize_rendering_surface(&self, new_width: NonZeroU32, new_height: NonZeroU32) {
        self.app.resize_rendering_surface(new_width, new_height);
    }

    pub fn perform_iteration(
        &mut self,
        event_loop_controller: &EventLoopController<'_>,
    ) -> ThreadPoolResult {
        if self.app.is_paused() {
            let iter_end_time = self.wait_for_target_frame_duration();
            self.previous_iter_end_time = iter_end_time;
            return Ok(());
        }

        let execution_result = with_timing_info_logging!("Game loop iteration"; {
            self.task_scheduler
                .execute_and_wait(&PHYSICS_AND_RENDERING_TAGS)
        });

        if let Err(mut task_errors) = execution_result {
            self.app
                .handle_task_errors(&mut task_errors, event_loop_controller);

            // Pass any unhandled errors to caller
            if task_errors.n_errors() > 0 {
                return Err(task_errors);
            }
        }

        self.app.renderer().write().unwrap().present();

        let iter_end_time = self.wait_for_target_frame_duration();

        let iter_duration = iter_end_time - self.previous_iter_end_time;
        self.frame_rate_tracker.add_frame_duration(iter_duration);
        self.previous_iter_end_time = iter_end_time;

        let smooth_frame_duration = self.frame_rate_tracker.compute_smooth_frame_duration();

        self.app
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

impl Default for GameLoopConfig {
    fn default() -> Self {
        Self {
            n_worker_threads: NonZeroUsize::new(1).unwrap(),
            max_fps: None,
        }
    }
}
