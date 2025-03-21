//! Manager for all systems and data in the application.

pub mod components;
pub mod entity;
pub mod tasks;

use crate::{
    assets::{Assets, lookup_table},
    component::ComponentRegistry,
    control::{
        self, MotionController, OrientationController,
        motion::{MotionDirection, MotionState},
    },
    gpu::{
        self, GraphicsDevice,
        rendering::{RenderingConfig, RenderingSystem, ScreenCapturer},
    },
    io,
    material::{self, MaterialLibrary},
    mesh::{MeshRepository, components::MeshComp, texture_projection::TextureProjection},
    model::{self, InstanceFeatureManager},
    physics::PhysicsSimulator,
    scene::{Scene, SceneEntityFlags, components::SceneEntityFlagsComp},
    skybox::Skybox,
    ui::UserInterface,
    voxel::{self, VoxelManager, voxel_types::VoxelTypeRegistry},
    window::Window,
};
use anyhow::{Result, anyhow};
use impact_ecs::{
    archetype::ArchetypeComponentStorage,
    component::{Component, SingleInstance},
    world::{Entity, World as ECSWorld},
};
use std::{
    fmt::Debug,
    num::NonZeroU32,
    path::Path,
    sync::{Arc, Mutex, RwLock},
};

/// Manager for all systems and data in the application.
#[derive(Debug)]
pub struct Application {
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

impl Application {
    /// Creates a new world data container.
    pub fn new(
        window: Arc<Window>,
        rendering_config: RenderingConfig,
        simulator: PhysicsSimulator,
        motion_controller: Option<Box<dyn MotionController>>,
        orientation_controller: Option<Box<dyn OrientationController>>,
        voxel_type_registry: VoxelTypeRegistry,
    ) -> Result<Self> {
        let mut component_registry = ComponentRegistry::new();
        if let Err(err) = components::register_all_components(&mut component_registry) {
            panic!("Failed to register components: {}", err);
        }

        let (graphics_device, rendering_surface) = gpu::initialize_for_rendering(&window)?;

        let renderer = RenderingSystem::new(
            rendering_config,
            Arc::clone(&graphics_device),
            rendering_surface,
        )?;

        let mut assets = Assets::new(
            Arc::clone(&graphics_device),
            Arc::clone(renderer.mipmapper_generator()),
        );

        lookup_table::initialize_default_lookup_tables(
            &mut assets,
            &mut renderer.gpu_resource_group_manager().write().unwrap(),
        )?;

        let material_library = MaterialLibrary::new();

        let mut mesh_repository = MeshRepository::new();
        mesh_repository.create_default_meshes();

        let mut instance_feature_manager = InstanceFeatureManager::new();
        model::register_model_feature_types(&mut instance_feature_manager);
        material::register_material_feature_types(&mut instance_feature_manager);
        voxel::register_voxel_feature_types(&mut instance_feature_manager);

        let voxel_manager = VoxelManager::new(voxel_type_registry);

        let scene = Scene::new(
            mesh_repository,
            material_library,
            instance_feature_manager,
            voxel_manager,
        );

        Ok(Self {
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
            screen_capturer: ScreenCapturer::new(),
        })
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
            .save_screenshot_if_requested(self.renderer())?;

        self.screen_capturer
            .save_omnidirectional_light_shadow_maps_if_requested(self.renderer())?;

        self.screen_capturer
            .save_unidirectional_light_shadow_maps_if_requested(self.renderer())
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
        io::obj::load_models_from_obj_file(&mut assets, &mut mesh_repository, obj_file_path)
    }

    /// Reads the Wavefront OBJ file at the given path and adds the contained
    /// mesh to the mesh repository if it does not already exist. If there
    /// are multiple meshes in the file, they are merged into a single mesh.
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

    /// Reads the Wavefront OBJ file at the given path and adds the contained
    /// mesh to the mesh repository if it does not already exist, after
    /// generating texture coordinates for the mesh using the given
    /// projection. If there are multiple meshes in the file, they are
    /// merged into a single mesh.
    ///
    /// # Returns
    /// The [`MeshComp`] representing the mesh.
    ///
    /// # Errors
    /// Returns an error if the file can not be found or loaded as a mesh.
    pub fn load_mesh_from_obj_file_with_projection<P>(
        &self,
        obj_file_path: P,
        projection: &impl TextureProjection<f32>,
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

    /// Reads the PLY (Polygon File Format, also called Stanford Triangle
    /// Format) file at the given path and adds the contained mesh to the
    /// mesh repository if it does not already exist, after generating
    /// texture coordinates for the mesh using the given projection.
    ///
    /// # Returns
    /// The [`MeshComp`] representing the mesh.
    ///
    /// # Errors
    /// Returns an error if the file can not be found or loaded as a mesh.
    pub fn load_mesh_from_ply_file_with_projection<P>(
        &self,
        ply_file_path: P,
        projection: &impl TextureProjection<f32>,
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

    /// Sets the given skybox as the skybox for the current scene.
    pub fn set_skybox_for_current_scene(&self, skybox: Skybox) {
        self.scene()
            .read()
            .unwrap()
            .skybox()
            .write()
            .unwrap()
            .replace(skybox);

        self.renderer()
            .read()
            .unwrap()
            .declare_render_resources_desynchronized();
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
            control::orientation::systems::update_rotation_of_controlled_entities(
                &ecs_world,
                orientation_controller.lock().unwrap().as_mut(),
                time_step_duration,
            );
        }

        if let Some(motion_controller) = &self.motion_controller {
            control::motion::systems::update_motion_of_controlled_entities(
                &ecs_world,
                motion_controller.lock().unwrap().as_ref(),
                time_step_duration,
            );
        }
    }

    pub fn toggle_wireframe_mode(&self) {
        let mut renderer = self.renderer().write().unwrap();
        renderer.toggle_wireframe_mode();
    }

    /// Toggles temporal anti-aliasing.
    pub fn toggle_temporal_anti_aliasing(&self) {
        let renderer = self.renderer().read().unwrap();
        let mut postprocessor = renderer.postprocessor().write().unwrap();
        let scene = self.scene().read().unwrap();
        let mut scene_camera = scene.scene_camera().write().unwrap();

        postprocessor.toggle_temporal_anti_aliasing();

        if let Some(camera) = scene_camera.as_mut() {
            camera.set_jitter_enabled(postprocessor.temporal_anti_aliasing_enabled());
            renderer.declare_render_resources_desynchronized();
        }
    }

    /// Increases the sensitivity of the capturing camera by a small
    /// multiplicative factor.
    pub fn increase_camera_sensitivity(&self) {
        self.renderer()
            .read()
            .unwrap()
            .postprocessor()
            .write()
            .unwrap()
            .capturing_camera_mut()
            .increase_sensitivity();
    }

    /// Decreases the sensitivity of the capturing camera by a small
    /// multiplicative factor.
    pub fn decrease_camera_sensitivity(&self) {
        self.renderer()
            .read()
            .unwrap()
            .postprocessor()
            .write()
            .unwrap()
            .capturing_camera_mut()
            .decrease_sensitivity();
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

    /// Sets the [`SceneEntityFlags::IS_DISABLED`] flag for the given entity.
    ///
    /// # Errors
    /// Returns an error if the entity does not exist or does not have the
    /// [`SceneEntityFlagsComp`] component.
    pub fn enable_scene_entity(&self, entity: &Entity) -> Result<()> {
        self.with_component_mut(entity, |flags: &mut SceneEntityFlagsComp| {
            flags.0.remove(SceneEntityFlags::IS_DISABLED);
            Ok(())
        })
    }

    /// Unsets the [`SceneEntityFlags::IS_DISABLED`] flag for the given entity.
    ///
    /// # Errors
    /// Returns an error if the entity does not exist or does not have the
    /// [`SceneEntityFlagsComp`] component.
    pub fn disable_scene_entity(&self, entity: &Entity) -> Result<()> {
        self.with_component_mut(entity, |flags: &mut SceneEntityFlagsComp| {
            flags.0.insert(SceneEntityFlags::IS_DISABLED);
            Ok(())
        })
    }

    /// Performs any setup required before starting the game loop.
    pub fn perform_setup_for_game_loop(&self) {
        self.simulator
            .read()
            .unwrap()
            .perform_setup_for_game_loop(self.ecs_world());
    }

    /// Resets the scene and ECS world to the initial empty state.
    pub fn clear_world(&self) {
        self.ecs_world.write().unwrap().remove_all_entities();
        self.scene.read().unwrap().clear();
        self.renderer
            .read()
            .unwrap()
            .declare_render_resources_desynchronized();
    }

    fn with_component_mut<C: Component, R>(
        &self,
        entity: &Entity,
        f: impl FnOnce(&mut C) -> Result<R>,
    ) -> Result<R> {
        let ecs_world = self.ecs_world.read().unwrap();

        let entity_entry = ecs_world
            .get_entity(entity)
            .ok_or_else(|| anyhow!("Missing entity: {:?}", entity))?;

        let mut component_entry = entity_entry.get_component_mut().ok_or_else(|| {
            anyhow!(
                "Missing component {:?} for entity: {:?}",
                C::component_id(),
                entity
            )
        })?;

        let component: &mut C = component_entry.access();

        f(component)
    }
}
