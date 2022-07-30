//! Main loop driving simulation and rendering.

use crate::{
    window::{ControlFlow, HandlingResult, InputHandler, WindowEvent},
    world::World,
};
use std::{
    num::NonZeroU32,
    thread,
    time::{Duration, Instant},
};

/// A loop driving simulation and rendering of a [`World`].
#[derive(Debug)]
pub struct GameLoop {
    world: World,
    input_handler: InputHandler,
    frame_rate_tracker: FrameDurationTracker,
    previous_iter_end_time: Instant,
    config: GameLoopConfig,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GameLoopConfig {
    max_fps: Option<NonZeroU32>,
}

#[derive(Clone, Debug)]
struct GenericFrameDurationTracker<const N_FRAMES: usize> {
    last_frame_durations: [Duration; N_FRAMES],
    idx_of_oldest: usize,
}

type FrameDurationTracker = GenericFrameDurationTracker<5>;

impl GameLoop {
    pub fn new(world: World, input_handler: InputHandler, config: GameLoopConfig) -> Self {
        let frame_rate_tracker = FrameDurationTracker::default();
        let previous_iter_end_time = Instant::now();
        Self {
            world,
            input_handler,
            frame_rate_tracker,
            previous_iter_end_time,
            config,
        }
    }

    pub fn handle_input_event(
        &mut self,
        control_flow: &mut ControlFlow<'_>,
        event: &WindowEvent<'_>,
    ) -> HandlingResult {
        self.input_handler
            .handle_event(&mut self.world, control_flow, event)
    }

    pub fn resize_rendering_surface(&mut self, new_size: (u32, u32)) {
        self.world.resize_rendering_surface(new_size);
    }

    pub fn perform_iteration(&mut self, control_flow: &mut ControlFlow<'_>) {
        self.world.sync_render_data();

        // <- Do physics here at the same time as rendering

        self.world.render(control_flow);

        let iter_end_time = self.wait_for_target_frame_duration();
        self.frame_rate_tracker
            .add_frame_duration(iter_end_time - self.previous_iter_end_time);
        self.previous_iter_end_time = iter_end_time;
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
        Self { max_fps: None }
    }
}
