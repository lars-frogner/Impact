//! Container for all data in the world.

use crate::{
    control::{self, MotionController, MotionDirection, MotionState, OrientationController},
    geometry::TextureProjection,
    physics::PhysicsSimulator,
    rendering::{fre, RenderingSystem, ScreenCapturer},
    scene::{io, MeshComp, Scene},
    scheduling::TaskScheduler,
    thread::ThreadPoolTaskErrors,
    ui::UserInterface,
    window::{EventLoopController, Window},
};
use anyhow::Result;
use impact_ecs::{
    archetype::{ArchetypeComponentStorage, ArchetypeComponents},
    component::{ComponentArray, SingleInstance},
    world::{Entity, World as ECSWorld},
};
use std::{
    fmt::Debug,
    num::NonZeroUsize,
    path::Path,
    sync::{Arc, Mutex, RwLock},
};

/// Container for all data required for simulating and
/// rendering the world.
#[derive(Debug)]
pub struct World {
    window: Arc<Window>,
    user_interface: RwLock<UserInterface>,
    ecs_world: RwLock<ECSWorld>,
    scene: RwLock<Scene>,
    renderer: RwLock<RenderingSystem>,
    simulator: RwLock<PhysicsSimulator>,
    motion_controller: Option<Mutex<Box<dyn MotionController>>>,
    orientation_controller: Option<Mutex<Box<dyn OrientationController>>>,
    screen_capturer: ScreenCapturer,
}

pub type WorldTaskScheduler = TaskScheduler<World>;

impl World {
    /// Creates a new world data container.
    pub fn new(
        window: Window,
        renderer: RenderingSystem,
        simulator: PhysicsSimulator,
        motion_controller: Option<Box<dyn MotionController>>,
        orientation_controller: Option<Box<dyn OrientationController>>,
    ) -> Self {
        let window = Arc::new(window);
        Self {
            window: Arc::clone(&window),
            user_interface: RwLock::new(UserInterface::new(window)),
            ecs_world: RwLock::new(ECSWorld::new()),
            scene: RwLock::new(Scene::new()),
            renderer: RwLock::new(renderer),
            simulator: RwLock::new(simulator),
            motion_controller: motion_controller.map(Mutex::new),
            orientation_controller: orientation_controller.map(Mutex::new),
            screen_capturer: ScreenCapturer::new(2048),
        }
    }

    /// Returns a reference to the [`Window`].
    pub fn window(&self) -> &Window {
        self.window.as_ref()
    }

    /// Returns a reference to the [`UserInterface`], guarded
    /// by a [`RwLock`].
    pub fn user_interface(&self) -> &RwLock<UserInterface> {
        &self.user_interface
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

    /// Returns a reference to the [`PhysicsSimulator`], guarded
    /// by a [`RwLock`].
    pub fn simulator(&self) -> &RwLock<PhysicsSimulator> {
        &self.simulator
    }

    /// Returns a reference to the [`ScreenCapturer`].
    pub fn screen_capturer(&self) -> &ScreenCapturer {
        &self.screen_capturer
    }

    /// Captures any screenshots or related textures requested through the
    /// [`ScreenCapturer`].
    pub fn capture_screenshots(&self) -> Result<()> {
        self.screen_capturer
            .save_screenshot_if_requested(self.renderer())?;

        self.screen_capturer
            .save_render_attachment_quantity_if_requested(self.renderer())?;

        self.screen_capturer
            .save_omnidirectional_light_shadow_map_if_requested(self.renderer())?;

        self.screen_capturer
            .save_unidirectional_light_shadow_map_if_requested(self.renderer())
    }

    /// Reads the Wavefront OBJ file at the given path and any associated MTL
    /// material files and returns the set of components representing the mesh
    /// and material of each model in the file. The meshes are added to the mesh
    /// repository, and any textures referenced in the MTL files are loaded as
    /// rendering assets. Each [`ArchetypeComponentStorage`] in the returned
    /// list contains the components describing a single model, and their order
    /// in the list is the same as their order in the OBJ file.
    ///
    /// # Errors
    /// Returns an error if any of the involved OBJ, MTL or texture files can
    /// not be found or loaded.
    pub fn load_models_from_obj_file<P>(
        &self,
        obj_file_path: P,
    ) -> Result<Vec<SingleInstance<ArchetypeComponentStorage>>>
    where
        P: AsRef<Path> + Debug,
    {
        io::load_models_from_obj_file(
            &self.renderer,
            self.scene.read().unwrap().mesh_repository(),
            obj_file_path,
        )
    }

    /// Reads the Wavefront OBJ file at the given path and adds the contained mesh
    /// to the mesh repository if it does not already exist. If there are multiple
    /// meshes in the file, they are merged into a single mesh.
    ///
    /// # Returns
    /// The [`MeshComp`] representing the mesh.
    ///
    /// # Errors
    /// Returns an error if the file can not be found or loaded as a mesh.
    pub fn load_mesh_from_obj_file<P>(&self, obj_file_path: P) -> Result<MeshComp>
    where
        P: AsRef<Path> + Debug,
    {
        io::load_mesh_from_obj_file(self.scene.read().unwrap().mesh_repository(), obj_file_path)
    }

    /// Reads the Wavefront OBJ file at the given path and adds the contained mesh
    /// to the mesh repository if it does not already exist, after generating
    /// texture coordinates for the mesh using the given projection. If there are
    /// multiple meshes in the file, they are merged into a single mesh.
    ///
    /// # Returns
    /// The [`MeshComp`] representing the mesh.
    ///
    /// # Errors
    /// Returns an error if the file can not be found or loaded as a mesh.
    pub fn load_mesh_from_obj_file_with_projection<P>(
        &self,
        obj_file_path: P,
        projection: &impl TextureProjection<fre>,
    ) -> Result<MeshComp>
    where
        P: AsRef<Path> + Debug,
    {
        io::load_mesh_from_obj_file_with_projection(
            self.scene.read().unwrap().mesh_repository(),
            obj_file_path,
            projection,
        )
    }

    /// Reads the PLY (Polygon File Format, also called Stanford Triangle
    /// Format) file at the given path and adds the contained mesh to the mesh
    /// repository if it does not already exist.
    ///
    /// # Returns
    /// The [`MeshComp`] representing the mesh.
    ///
    /// # Errors
    /// Returns an error if the file can not be found or loaded as a mesh.
    pub fn load_mesh_from_ply_file<P>(&self, ply_file_path: P) -> Result<MeshComp>
    where
        P: AsRef<Path> + Debug,
    {
        io::load_mesh_from_ply_file(self.scene.read().unwrap().mesh_repository(), ply_file_path)
    }

    /// Reads the PLY (Polygon File Format, also called Stanford Triangle Format)
    /// file at the given path and adds the contained mesh to the mesh repository if
    /// it does not already exist, after generating texture coordinates for the mesh
    /// using the given projection.
    ///
    /// # Returns
    /// The [`MeshComp`] representing the mesh.
    ///
    /// # Errors
    /// Returns an error if the file can not be found or loaded as a mesh.
    pub fn load_mesh_from_ply_file_with_projection<P>(
        &self,
        ply_file_path: P,
        projection: &impl TextureProjection<fre>,
    ) -> Result<MeshComp>
    where
        P: AsRef<Path> + Debug,
    {
        io::load_mesh_from_ply_file_with_projection(
            self.scene.read().unwrap().mesh_repository(),
            ply_file_path,
            projection,
        )
    }

    pub fn create_entity<A, E>(
        &self,
        components: impl TryInto<SingleInstance<ArchetypeComponents<A>>, Error = E>,
    ) -> Result<Entity>
    where
        A: ComponentArray,
        E: Into<anyhow::Error>,
    {
        Ok(self
            .create_entities(components.try_into().map_err(E::into)?.into_inner())?
            .pop()
            .unwrap())
    }

    pub fn create_entities<A, E>(
        &self,
        components: impl TryInto<ArchetypeComponents<A>, Error = E>,
    ) -> Result<Vec<Entity>>
    where
        A: ComponentArray,
        E: Into<anyhow::Error>,
    {
        let mut components = components.try_into().map_err(E::into)?.into_storage();

        let render_resources_desynchronized = self.scene().read().unwrap().handle_entity_created(
            self.window(),
            &self.ecs_world,
            &mut components,
        )?;

        if render_resources_desynchronized.is_yes() {
            self.renderer()
                .read()
                .unwrap()
                .declare_render_resources_desynchronized();
        }

        self.ecs_world.write().unwrap().create_entities(components)
    }

    pub fn remove_entity(&self, entity: &Entity) -> Result<()> {
        let mut ecs_world = self.ecs_world.write().unwrap();

        let entry = ecs_world.entity(entity);

        let render_resources_desynchronized =
            self.scene().read().unwrap().handle_entity_removed(&entry);

        drop(entry);

        if render_resources_desynchronized.is_yes() {
            self.renderer()
                .read()
                .unwrap()
                .declare_render_resources_desynchronized();
        }

        ecs_world.remove_entity(entity)
    }

    /// Sets a new size for the rendering surface and updates
    /// the aspect ratio of all cameras.
    pub fn resize_rendering_surface(&self, new_size: (u32, u32)) {
        self.renderer().write().unwrap().resize_surface(new_size);

        let render_resources_desynchronized =
            self.scene().read().unwrap().handle_window_resized(new_size);

        if render_resources_desynchronized.is_yes() {
            self.renderer()
                .read()
                .unwrap()
                .declare_render_resources_desynchronized();
        }
    }

    pub fn toggle_interaction_mode(&self) {
        let mut user_interface = self.user_interface().write().unwrap();
        if user_interface.control_mode_active() {
            self.stop_motion_controller();
            user_interface.activate_cursor_mode();
        } else {
            user_interface.activate_control_mode();
        }
    }

    pub fn control_mode_active(&self) -> bool {
        self.user_interface().read().unwrap().control_mode_active()
    }

    /// Updates the motion controller with the given motion.
    pub fn update_motion_controller(&self, state: MotionState, direction: MotionDirection) {
        if let Some(motion_controller) = &self.motion_controller {
            log::debug!(
                "Updating motion controller to state {:?} and direction {:?}",
                state,
                direction
            );

            let mut motion_controller = motion_controller.lock().unwrap();

            let result = motion_controller.update_motion(state, direction);

            if result.motion_changed() {
                control::set_velocities_of_controlled_entities(
                    &self.ecs_world().read().unwrap(),
                    motion_controller.as_ref(),
                );
            }
        }
    }

    fn stop_motion_controller(&self) {
        if let Some(motion_controller) = &self.motion_controller {
            let mut motion_controller = motion_controller.lock().unwrap();

            let result = motion_controller.stop();

            if result.motion_changed() {
                control::set_velocities_of_controlled_entities(
                    &self.ecs_world().read().unwrap(),
                    motion_controller.as_ref(),
                );
            }
        }
    }

    /// Updates the orientation controller with the given mouse
    /// displacement.
    pub fn update_orientation_controller(&self, mouse_displacement: (f64, f64)) {
        if let Some(orientation_controller) = &self.orientation_controller {
            log::info!(
                "Updating orientation controller by mouse delta ({}, {})",
                mouse_displacement.0,
                mouse_displacement.1
            );

            let mut orientation_controller = orientation_controller.lock().unwrap();

            orientation_controller.update_orientation_change(self.window(), mouse_displacement);

            let ecs_world = self.ecs_world().read().unwrap();
            control::update_orientations_of_controlled_entities(
                &ecs_world,
                orientation_controller.as_ref(),
            );
            if let Some(motion_controller) = &self.motion_controller {
                control::set_velocities_of_controlled_entities(
                    &ecs_world,
                    motion_controller.lock().unwrap().as_ref(),
                );
            }
        }
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
        event_loop_controller: &EventLoopController<'_>,
    ) {
        self.simulator
            .read()
            .unwrap()
            .handle_task_errors(task_errors, event_loop_controller);

        self.scene
            .read()
            .unwrap()
            .handle_task_errors(task_errors, event_loop_controller);

        self.renderer
            .read()
            .unwrap()
            .handle_task_errors(task_errors, event_loop_controller);
    }

    /// Registers all tasks in the given task scheduler.
    fn register_all_tasks(task_scheduler: &mut WorldTaskScheduler) -> Result<()> {
        Scene::register_tasks(task_scheduler)?;
        RenderingSystem::register_tasks(task_scheduler)?;
        PhysicsSimulator::register_tasks(task_scheduler)?;
        task_scheduler.complete_task_registration()
    }
}
