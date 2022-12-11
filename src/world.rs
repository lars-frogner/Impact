//! Container for all data in the world.

use crate::{
    control::{MotionController, MotionDirection, MotionState},
    rendering::RenderingSystem,
    scene::Scene,
    scheduling::TaskScheduler,
    thread::ThreadPoolTaskErrors,
    window::{self, ControlFlow},
};
use anyhow::Result;
use impact_ecs::world::World as ECSWorld;
use std::{
    num::NonZeroUsize,
    sync::{Arc, Mutex, RwLock},
};

/// Container for all data required for simulating and
/// rendering the world.
#[derive(Debug)]
pub struct World {
    ecs_world: RwLock<ECSWorld>,
    scene: RwLock<Scene>,
    renderer: RwLock<RenderingSystem>,
    motion_controller: Mutex<Box<dyn MotionController<f32>>>,
}

pub type WorldTaskScheduler = TaskScheduler<World>;

impl World {
    /// Creates a new world data container.
    pub fn new(
        scene: Scene,
        renderer: RenderingSystem,
        controller: impl 'static + MotionController<f32>,
    ) -> Self {
        Self {
            ecs_world: RwLock::new(ECSWorld::new()),
            scene: RwLock::new(scene),
            renderer: RwLock::new(renderer),
            motion_controller: Mutex::new(Box::new(controller)),
        }
    }

    /// Returns a reference to the ECS [`World`](impact_ecs::world::World), guarded
    /// by a [`RwLock`].
    pub fn ecs_world(&self) -> &RwLock<ECSWorld> {
        &self.ecs_world
    }

    /// Returns a reference to the [`Scene`], guarded
    /// by a [`RwLock`].
    pub fn scene(&self) -> &RwLock<Scene> {
        &self.scene
    }

    /// Returns a reference to the [`RenderingSystem`], guarded
    /// by a [`RwLock`].
    pub fn renderer(&self) -> &RwLock<RenderingSystem> {
        &self.renderer
    }

    /// Sets a new size for the rendering surface and updates
    /// the aspect ratio of all cameras.
    pub fn resize_rendering_surface(&self, new_size: (u32, u32)) {
        self.renderer.write().unwrap().resize_surface(new_size);

        self.scene()
            .read()
            .unwrap()
            .camera_repository()
            .write()
            .unwrap()
            .set_aspect_ratios(window::calculate_aspect_ratio(new_size.0, new_size.1));
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
    }

    /// Creates a new task scheduler with the given number of
    /// workers and registers all tasks in it.
    ///
    /// # Errors
    /// Returns an error the registration of any of the tasks
    /// failed.
    pub fn create_task_scheduler(
        self,
        n_workers: NonZeroUsize,
    ) -> Result<(Arc<Self>, WorldTaskScheduler)> {
        let world = Arc::new(self);
        let mut task_scheduler = WorldTaskScheduler::new(n_workers, Arc::clone(&world));

        Self::register_all_tasks(&mut task_scheduler)?;

        Ok((world, task_scheduler))
    }

    /// Identifies errors that need special handling in the given
    /// set of task errors and handles them.
    pub fn handle_task_errors(
        &self,
        task_errors: &mut ThreadPoolTaskErrors,
        control_flow: &mut ControlFlow<'_>,
    ) {
        self.scene
            .read()
            .unwrap()
            .handle_task_errors(task_errors, control_flow);

        self.renderer
            .read()
            .unwrap()
            .handle_task_errors(task_errors, control_flow);
    }

    /// Registers all tasks in the given task scheduler.
    fn register_all_tasks(task_scheduler: &mut WorldTaskScheduler) -> Result<()> {
        Scene::register_tasks(task_scheduler)?;
        RenderingSystem::register_tasks(task_scheduler)?;
        task_scheduler.complete_task_registration()
    }
}
