//! Container for all data in the world.

use crate::{
    control::{MotionController, MotionDirection, MotionState},
    geometry::GeometricalData,
    rendering::RenderingSystem,
    scheduling::TaskScheduler,
    thread::ThreadPoolTaskErrors,
    window::ControlFlow,
};
use anyhow::Result;
use std::{
    num::NonZeroUsize,
    sync::{Arc, Mutex, RwLock},
};

/// Container for all data required for simulating and
/// rendering the world.
#[derive(Debug)]
pub struct World {
    geometrical_data: RwLock<GeometricalData>,
    renderer: RwLock<RenderingSystem>,
    motion_controller: Mutex<Box<dyn MotionController<f32>>>,
}

pub type WorldTaskScheduler = TaskScheduler<World>;

impl World {
    /// Creates a new world data container.
    pub fn new(
        geometrical_data: GeometricalData,
        renderer: RenderingSystem,
        controller: impl 'static + MotionController<f32>,
    ) -> Self {
        Self {
            geometrical_data: RwLock::new(geometrical_data),
            renderer: RwLock::new(renderer),
            motion_controller: Mutex::new(Box::new(controller)),
        }
    }

    /// Returns a reference to the [`GeometricalData`], guarded
    /// by a [`RwLock`].
    pub fn geometrical_data(&self) -> &RwLock<GeometricalData> {
        &self.geometrical_data
    }

    /// Returns a reference to the [`RenderingSystem`], guarded
    /// by a [`RwLock`].
    pub fn renderer(&self) -> &RwLock<RenderingSystem> {
        &self.renderer
    }

    /// Updates the motion controller with the given motion.
    pub fn update_motion_controller(&self, state: MotionState, direction: MotionDirection) {
        log::debug!(
            "Updating motion controller to state {:?} and direction {:?}",
            state,
            direction
        );

        let mut motion_controller = self.motion_controller.lock().unwrap();

        motion_controller.update_motion(state, direction);

        if let Some(translation) = motion_controller.next_translation() {
            drop(motion_controller); // Don't hold lock longer than neccessary

            self.geometrical_data
                .write()
                .unwrap()
                .transform_cameras(&translation.into());
        }
    }

    pub fn create_task_scheduler(
        self,
        n_workers: NonZeroUsize,
    ) -> Result<(Arc<Self>, WorldTaskScheduler)> {
        let world = Arc::new(self);
        let mut task_scheduler = WorldTaskScheduler::new(n_workers, Arc::clone(&world));

        Self::register_all_tasks(&mut task_scheduler)?;

        Ok((world, task_scheduler))
    }

    pub fn handle_task_errors(
        &self,
        task_errors: &mut ThreadPoolTaskErrors,
        control_flow: &mut ControlFlow<'_>,
    ) {
        self.renderer
            .read()
            .unwrap()
            .handle_task_errors(task_errors, control_flow);
    }

    fn register_all_tasks(task_scheduler: &mut WorldTaskScheduler) -> Result<()> {
        RenderingSystem::register_tasks(task_scheduler)?;
        task_scheduler.complete_task_registration()
    }
}
