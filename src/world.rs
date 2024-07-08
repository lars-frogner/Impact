//! Container for all data in the world.

use crate::{
    assets::Assets,
    camera,
    components::{ComponentCategory, ComponentRegistry},
    control::{self, MotionController, MotionDirection, MotionState, OrientationController},
    gpu::{
        rendering::{fre, RenderingSystem, ScreenCapturer},
        GraphicsDevice,
    },
    io, light, material,
    mesh::{self, components::MeshComp, texture_projection::TextureProjection},
    physics::{self, PhysicsSimulator, SteppingScheme},
    scene::{self, RenderResourcesDesynchronized, Scene},
    scheduling::TaskScheduler,
    thread::ThreadPoolTaskErrors,
    ui::UserInterface,
    voxel,
    window::{EventLoopController, Window},
};
use anyhow::Result;
use impact_ecs::{
    archetype::{ArchetypeComponentStorage, ArchetypeComponents},
    component::{ComponentArray, ComponentID, SingleInstance},
    world::{Entity, World as ECSWorld},
};
use std::{
    fmt::Debug,
    num::{NonZeroU32, NonZeroUsize},
    path::Path,
    sync::{Arc, Mutex, RwLock},
};

/// Container for all data required for simulating and
/// rendering the world.
#[derive(Debug)]
pub struct World {
    window: Arc<Window>,
    graphics_device: Arc<GraphicsDevice>,
    user_interface: RwLock<UserInterface>,
    component_registry: RwLock<ComponentRegistry>,
    ecs_world: RwLock<ECSWorld>,
    renderer: RwLock<RenderingSystem>,
    assets: RwLock<Assets>,
    scene: RwLock<Scene>,
    simulator: RwLock<PhysicsSimulator>,
    motion_controller: Option<Mutex<Box<dyn MotionController>>>,
    orientation_controller: Option<Mutex<Box<dyn OrientationController>>>,
    screen_capturer: ScreenCapturer,
}

pub type WorldTaskScheduler = TaskScheduler<World>;

impl World {
    /// Creates a new world data container.
    pub fn new(
        window: Arc<Window>,
        graphics_device: Arc<GraphicsDevice>,
        renderer: RenderingSystem,
        assets: Assets,
        scene: Scene,
        simulator: PhysicsSimulator,
        motion_controller: Option<Box<dyn MotionController>>,
        orientation_controller: Option<Box<dyn OrientationController>>,
    ) -> Self {
        let mut component_registry = ComponentRegistry::new();
        if let Err(err) = Self::register_all_components(&mut component_registry) {
            panic!("Failed to register components: {}", err);
        }

        Self {
            window: Arc::clone(&window),
            graphics_device,
            user_interface: RwLock::new(UserInterface::new(window)),
            component_registry: RwLock::new(component_registry),
            ecs_world: RwLock::new(ECSWorld::new()),
            renderer: RwLock::new(renderer),
            assets: RwLock::new(assets),
            scene: RwLock::new(scene),
            simulator: RwLock::new(simulator),
            motion_controller: motion_controller.map(Mutex::new),
            orientation_controller: orientation_controller.map(Mutex::new),
            screen_capturer: ScreenCapturer::new(NonZeroU32::new(2048).unwrap()),
        }
    }

    /// Returns a reference to the [`Window`].
    pub fn window(&self) -> &Window {
        self.window.as_ref()
    }

    /// Returns a reference to the [`GraphicsDevice`].
    pub fn graphics_device(&self) -> &GraphicsDevice {
        &self.graphics_device
    }

    /// Returns a reference to the [`UserInterface`], guarded by a [`RwLock`].
    pub fn user_interface(&self) -> &RwLock<UserInterface> {
        &self.user_interface
    }

    /// Returns a reference to the ECS [`ComponentRegistry`], guarded by a
    /// [`RwLock`].
    pub fn component_registry(&self) -> &RwLock<ComponentRegistry> {
        &self.component_registry
    }

    /// Returns a reference to the ECS [`World`](impact_ecs::world::World),
    /// guarded by a [`RwLock`].
    pub fn ecs_world(&self) -> &RwLock<ECSWorld> {
        &self.ecs_world
    }

    /// Returns a reference to the [`RenderingSystem`], guarded by a [`RwLock`].
    pub fn renderer(&self) -> &RwLock<RenderingSystem> {
        &self.renderer
    }

    /// Returns a reference to the [`Assets`], guarded by a [`RwLock`].
    pub fn assets(&self) -> &RwLock<Assets> {
        &self.assets
    }

    /// Returns a reference to the [`Scene`], guarded by a [`RwLock`].
    pub fn scene(&self) -> &RwLock<Scene> {
        &self.scene
    }

    /// Returns a reference to the [`PhysicsSimulator`], guarded by a
    /// [`RwLock`].
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
            .save_screenshot_if_requested(self.renderer(), self.scene())?;

        self.screen_capturer
            .save_render_attachment_quantity_if_requested(self.renderer(), self.scene())?;

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
        let mut assets = self.assets.write().unwrap();
        let scene = self.scene.read().unwrap();
        let mut mesh_repository = scene.mesh_repository().write().unwrap();
        io::obj::load_models_from_obj_file(
            self.graphics_device(),
            &mut assets,
            &mut mesh_repository,
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
        let scene = self.scene.read().unwrap();
        let mut mesh_repository = scene.mesh_repository().write().unwrap();
        io::obj::load_mesh_from_obj_file(&mut mesh_repository, obj_file_path)
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
        let scene = self.scene.read().unwrap();
        let mut mesh_repository = scene.mesh_repository().write().unwrap();
        io::obj::load_mesh_from_obj_file_with_projection(
            &mut mesh_repository,
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
        let scene = self.scene.read().unwrap();
        let mut mesh_repository = scene.mesh_repository().write().unwrap();
        io::ply::load_mesh_from_ply_file(&mut mesh_repository, ply_file_path)
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
        let scene = self.scene.read().unwrap();
        let mut mesh_repository = scene.mesh_repository().write().unwrap();
        io::ply::load_mesh_from_ply_file_with_projection(
            &mut mesh_repository,
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

        let mut render_resources_desynchronized = RenderResourcesDesynchronized::No;

        self.scene().read().unwrap().perform_setup_for_new_entity(
            self.graphics_device(),
            &self.assets().read().unwrap(),
            &mut components,
            &mut render_resources_desynchronized,
        )?;

        self.simulator()
            .read()
            .unwrap()
            .perform_setup_for_new_entity(
                self.scene().read().unwrap().mesh_repository(),
                &mut components,
            );

        self.scene().read().unwrap().add_new_entity_to_scene_graph(
            self.window(),
            &self.ecs_world,
            &mut components,
            &mut render_resources_desynchronized,
        )?;

        if render_resources_desynchronized.is_yes() {
            self.renderer()
                .read()
                .unwrap()
                .declare_render_resources_desynchronized();
        }

        let (setup_component_ids, setup_component_names, standard_component_names) =
            self.extract_component_metadata(&components);

        log::info!(
            "Creating {} entities:\nSetup components:\n    {}\nStandard components:\n    {}",
            components.component_count(),
            setup_component_names.join("\n    "),
            standard_component_names.join("\n    "),
        );

        // Remove all setup components
        components.remove_component_types_with_ids(setup_component_ids)?;

        self.ecs_world.write().unwrap().create_entities(components)
    }

    pub fn remove_entity(&self, entity: &Entity) -> Result<()> {
        let mut ecs_world = self.ecs_world.write().unwrap();

        let entry = ecs_world.entity(entity);

        self.simulator()
            .read()
            .unwrap()
            .perform_cleanup_for_removed_entity(&entry);

        let render_resources_desynchronized = self
            .scene()
            .read()
            .unwrap()
            .perform_cleanup_for_removed_entity(&entry);

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
    pub fn resize_rendering_surface(&self, new_width: NonZeroU32, new_height: NonZeroU32) {
        let mut renderer = self.renderer().write().unwrap();

        let (old_width, old_height) = renderer.rendering_surface().surface_dimensions();

        renderer.resize_rendering_surface(new_width, new_height);
        drop(renderer);

        let render_resources_desynchronized = self
            .scene()
            .read()
            .unwrap()
            .handle_window_resized(old_width, old_height, new_width, new_height);

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

    pub fn is_paused(&self) -> bool {
        self.user_interface().read().unwrap().is_paused()
    }

    /// Updates the motion controller with the given motion.
    pub fn update_motion_controller(&self, state: MotionState, direction: MotionDirection) {
        if let Some(motion_controller) = &self.motion_controller {
            log::debug!(
                "Updating motion controller to state {:?} and direction {:?}",
                state,
                direction
            );
            motion_controller
                .lock()
                .unwrap()
                .update_motion(state, direction);
        }
    }

    fn stop_motion_controller(&self) {
        if let Some(motion_controller) = &self.motion_controller {
            motion_controller.lock().unwrap().stop();
        }
    }

    /// Updates the orientation controller with the given mouse displacement.
    pub fn update_orientation_controller(&self, mouse_displacement: (f64, f64)) {
        if let Some(orientation_controller) = &self.orientation_controller {
            log::debug!(
                "Updating orientation controller by mouse delta ({}, {})",
                mouse_displacement.0,
                mouse_displacement.1
            );

            orientation_controller
                .lock()
                .unwrap()
                .update_orientation_change(self.window(), mouse_displacement);
        }
    }

    /// Updates the orientations and motion of all controlled entities.
    pub fn update_controlled_entities(&self) {
        let ecs_world = self.ecs_world().read().unwrap();
        let time_step_duration = self.simulator.read().unwrap().scaled_time_step_duration();

        if let Some(orientation_controller) = &self.orientation_controller {
            control::update_rotation_of_controlled_entities(
                &ecs_world,
                orientation_controller.lock().unwrap().as_mut(),
                time_step_duration,
            );
        }

        if let Some(motion_controller) = &self.motion_controller {
            control::update_motion_of_controlled_entities(
                &ecs_world,
                motion_controller.lock().unwrap().as_ref(),
                time_step_duration,
            );
        }
    }

    /// Increases the exposure by a small multiplicative factor.
    pub fn increase_exposure(&self) {
        self.renderer()
            .read()
            .unwrap()
            .postprocessor()
            .write()
            .unwrap()
            .increase_exposure();
    }

    /// Decreases the exposure by a small multiplicative factor.
    pub fn decrease_exposure(&self) {
        self.renderer()
            .read()
            .unwrap()
            .postprocessor()
            .write()
            .unwrap()
            .decrease_exposure();
    }

    /// Increases the simulation speed multiplier by the
    /// `simulation_speed_multiplier_increment_factor` specified in the
    /// simulation configuration and decrease the motion controller speed by the
    /// same factor to compensate.
    pub fn increment_simulation_speed_multiplier_and_compensate_controller_speed(&self) {
        let mut simulator = self.simulator.write().unwrap();
        simulator.increment_simulation_speed_multiplier();

        if let Some(motion_controller) = &self.motion_controller {
            let mut motion_controller = motion_controller.lock().unwrap();
            let new_movement_speed = motion_controller.movement_speed()
                / simulator.simulation_speed_multiplier_increment_factor();
            motion_controller.set_movement_speed(new_movement_speed);
        }
    }

    /// Decreases the simulation speed multiplier by the
    /// `simulation_speed_multiplier_increment_factor` specified in the
    /// simulation configuration and increase the motion controller speed by the
    /// same factor to compensate.
    pub fn decrement_simulation_speed_multiplier_and_compensate_controller_speed(&self) {
        let mut simulator = self.simulator.write().unwrap();
        simulator.decrement_simulation_speed_multiplier();

        if let Some(motion_controller) = &self.motion_controller {
            let mut motion_controller = motion_controller.lock().unwrap();
            let new_movement_speed = motion_controller.movement_speed()
                * simulator.simulation_speed_multiplier_increment_factor();
            motion_controller.set_movement_speed(new_movement_speed);
        }
    }

    /// Changes to the next stepping scheme for the physcis simulation.
    pub fn cycle_simulation_stepping_scheme(&self) {
        let mut simulator = self.simulator.write().unwrap();
        let new_stepping_scheme = match simulator.stepping_scheme() {
            SteppingScheme::EulerCromer => SteppingScheme::RK4,
            SteppingScheme::RK4 => SteppingScheme::EulerCromer,
        };
        simulator.set_stepping_scheme(new_stepping_scheme);
    }

    /// Performs any setup required before starting the game loop.
    pub fn perform_setup_for_game_loop(&self) {
        self.simulator
            .read()
            .unwrap()
            .perform_setup_for_game_loop(self.ecs_world());
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

    /// Registers all components in the given registry.
    fn register_all_components(registry: &mut ComponentRegistry) -> Result<()> {
        control::register_control_components(registry)?;
        physics::register_physics_components(registry)?;
        scene::components::register_scene_graph_components(registry)?;
        camera::components::register_camera_components(registry)?;
        light::components::register_light_components(registry)?;
        mesh::components::register_mesh_components(registry)?;
        mesh::texture_projection::components::register_texture_projection_components(registry)?;
        material::components::register_material_components(registry)?;
        voxel::components::register_voxel_components(registry)
    }

    /// Registers all tasks in the given task scheduler.
    fn register_all_tasks(task_scheduler: &mut WorldTaskScheduler) -> Result<()> {
        Scene::register_tasks(task_scheduler)?;
        RenderingSystem::register_tasks(task_scheduler)?;
        PhysicsSimulator::register_tasks(task_scheduler)?;
        task_scheduler.complete_task_registration()
    }

    fn extract_component_metadata(
        &self,
        components: &ArchetypeComponentStorage,
    ) -> (Vec<ComponentID>, Vec<&'static str>, Vec<&'static str>) {
        let mut setup_component_ids = Vec::with_capacity(components.n_component_types());
        let mut setup_component_names = Vec::with_capacity(components.n_component_types());
        let mut standard_component_names = Vec::with_capacity(components.n_component_types());

        let component_registry = self.component_registry.read().unwrap();

        for component_id in components.component_ids() {
            let entry = component_registry.component_with_id(component_id);
            match entry.category {
                ComponentCategory::Standard => {
                    standard_component_names.push(entry.name);
                }
                ComponentCategory::Setup => {
                    setup_component_ids.push(component_id);
                    setup_component_names.push(entry.name);
                }
            }
        }

        (
            setup_component_ids,
            setup_component_names,
            standard_component_names,
        )
    }
}
