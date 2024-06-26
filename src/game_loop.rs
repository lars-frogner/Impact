//! Main loop driving simulation and rendering.

use crate::{
    define_execution_tag_set,
    physics::PhysicsTag,
    rendering::RenderingTag,
    thread::ThreadPoolResult,
    window::{EventLoopController, HandlingResult, InputHandler, Window, WindowEvent},
    world::{World, WorldTaskScheduler},
};
use anyhow::Result;
use std::{
    num::{NonZeroU32, NonZeroUsize},
    sync::Arc,
    thread,
    time::{Duration, Instant},
};
use winit::event::DeviceEvent;

/// A loop driving simulation and rendering of a [`World`].
#[derive(Debug)]
pub struct GameLoop {
    world: Arc<World>,
    task_scheduler: WorldTaskScheduler,
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

type FrameDurationTracker = GenericFrameDurationTracker<5>;

impl GameLoop {
    pub fn new(world: World, input_handler: InputHandler, config: GameLoopConfig) -> Result<Self> {
        let (world, task_scheduler) = world.create_task_scheduler(config.n_worker_threads)?;

        world.perform_setup_for_game_loop();

        let frame_rate_tracker = FrameDurationTracker::default();
        let start_time = Instant::now();
        let previous_iter_end_time = start_time;

        Ok(Self {
            world,
            task_scheduler,
            input_handler,
            frame_rate_tracker,
            start_time,
            previous_iter_end_time,
            config,
        })
    }

    pub fn world(&self) -> &World {
        self.world.as_ref()
    }

    pub fn window(&self) -> &Window {
        self.world().window()
    }

    pub fn handle_window_event(
        &self,
        event_loop_controller: &EventLoopController<'_>,
        event: &WindowEvent,
    ) -> Result<HandlingResult> {
        self.input_handler
            .handle_window_event(&self.world, event_loop_controller, event)
    }

    pub fn handle_device_event(
        &self,
        event_loop_controller: &EventLoopController<'_>,
        event: &DeviceEvent,
    ) -> Result<HandlingResult> {
        self.input_handler
            .handle_device_event(&self.world, event_loop_controller, event)
    }

    pub fn resize_rendering_surface(&self, new_size: (u32, u32)) {
        self.world.resize_rendering_surface(new_size);
    }

    pub fn perform_iteration(
        &mut self,
        event_loop_controller: &EventLoopController<'_>,
    ) -> ThreadPoolResult {
        if self.world.is_paused() {
            let iter_end_time = self.wait_for_target_frame_duration();
            self.previous_iter_end_time = iter_end_time;
            return Ok(());
        }

        let execution_result = with_timing_info_logging!("Game loop iteration"; {
            self.task_scheduler
                .execute_and_wait(&PHYSICS_AND_RENDERING_TAGS)
        });

        if let Err(mut task_errors) = execution_result {
            self.world
                .handle_task_errors(&mut task_errors, event_loop_controller);

            // Pass any unhandled errors to caller
            if task_errors.n_errors() > 0 {
                return Err(task_errors);
            }
        }

        let iter_end_time = self.wait_for_target_frame_duration();
        let iter_duration = iter_end_time - self.previous_iter_end_time;
        self.frame_rate_tracker.add_frame_duration(iter_duration);
        self.previous_iter_end_time = iter_end_time;

        let smooth_frame_duration = self.frame_rate_tracker.compute_smooth_frame_duration();

        self.world
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

            let remaining_duration = target_end_time - iter_end_time;
            if remaining_duration > Duration::ZERO {
                thread::sleep(remaining_duration);
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
            max_fps: Some(NonZeroU32::new(30).unwrap()),
        }
    }
}
