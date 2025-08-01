//! Manager for all systems and data in the engine.

pub mod components;
pub mod entity;
pub mod game_loop;

#[cfg(feature = "window")]
pub mod window;

use crate::{
    application::Application,
    command::{self, EngineCommand},
    component::ComponentRegistry,
    game_loop::{GameLoopConfig, GameLoopController},
    gizmo::{self, GizmoConfig, GizmoManager},
    gpu::GraphicsContext,
    instrumentation::{EngineMetrics, InstrumentationConfig, timing::TaskTimer},
    physics::PhysicsSimulator,
    rendering::{RenderingConfig, RenderingSystem, screen_capture::ScreenCapturer},
    resource::{ResourceConfig, ResourceManager},
    scene::Scene,
};
use anyhow::{Result, anyhow};
use impact_controller::{ControllerConfig, MotionController, OrientationController};
use impact_ecs::{
    component::Component,
    world::{EntityID, EntityStager, World as ECSWorld},
};
use impact_gpu::device::GraphicsDevice;
use impact_physics::PhysicsConfig;
use impact_scene::model::InstanceFeatureManager;
use impact_texture::{SamplerRegistry, TextureRegistry};
use impact_thread::ThreadPoolTaskErrors;
use impact_voxel::{VoxelConfig, voxel_types::VoxelTypeRegistry};
use parking_lot::{Mutex, RwLock, RwLockReadGuard};
use serde::{Deserialize, Serialize};
use std::{
    fmt::Debug,
    num::NonZeroU32,
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

/// Manager for all systems and data in the engine.
#[derive(Debug)]
pub struct Engine {
    app: Arc<dyn Application>,
    graphics_device: Arc<GraphicsDevice>,
    component_registry: RwLock<ComponentRegistry>,
    ecs_world: RwLock<ECSWorld>,
    entity_stager: Mutex<EntityStager>,
    renderer: RwLock<RenderingSystem>,
    resource_manager: RwLock<ResourceManager>,
    scene: RwLock<Scene>,
    simulator: RwLock<PhysicsSimulator>,
    gizmo_manager: RwLock<GizmoManager>,
    motion_controller: Option<Mutex<Box<dyn MotionController>>>,
    orientation_controller: Option<Mutex<Box<dyn OrientationController>>>,
    screen_capturer: ScreenCapturer,
    task_timer: TaskTimer,
    metrics: RwLock<EngineMetrics>,
    game_loop_controller: Mutex<GameLoopController>,
    controls_enabled: AtomicBool,
    shutdown_requested: AtomicBool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct EngineConfig {
    pub resources: ResourceConfig,
    pub rendering: RenderingConfig,
    pub physics: PhysicsConfig,
    pub voxel: VoxelConfig,
    pub controller: ControllerConfig,
    pub ecs: ECSConfig,
    pub gizmo: GizmoConfig,
    pub instrumentation: InstrumentationConfig,
    pub game_loop: GameLoopConfig,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ECSConfig {
    pub seed: u64,
}

impl Engine {
    /// Creates a new instance of the engine.
    pub fn new(
        config: EngineConfig,
        app: Arc<dyn Application>,
        graphics: GraphicsContext,
    ) -> Result<Self> {
        let mut component_registry = ComponentRegistry::new();
        components::register_all_components(&mut component_registry)?;

        let ecs_world = ECSWorld::new(config.ecs.seed);

        let mut texture_registry = TextureRegistry::new();
        let mut sampler_registry = SamplerRegistry::new();

        let voxel_type_registry = VoxelTypeRegistry::from_config(
            &mut texture_registry,
            &mut sampler_registry,
            config.voxel,
        )?;

        let mut resource_manager = ResourceManager::new(
            config.resources,
            texture_registry,
            sampler_registry,
            voxel_type_registry,
        );

        resource_manager.load_builtin_resources()?;
        resource_manager.load_resources_declared_in_config()?;

        gizmo::mesh::generate_gizmo_meshes(&mut resource_manager);

        let mut instance_feature_manager = InstanceFeatureManager::new();
        impact_model::register_model_feature_types(&mut instance_feature_manager);
        impact_material::register_material_feature_types(&mut instance_feature_manager);
        impact_voxel::register_voxel_feature_types(&mut instance_feature_manager);
        gizmo::initialize_buffers_for_gizmo_models(&mut instance_feature_manager);

        let scene = Scene::new(instance_feature_manager);

        let graphics_device = Arc::new(graphics.device);
        let rendering_surface = graphics.surface;

        let renderer = RenderingSystem::new(
            config.rendering,
            Arc::clone(&graphics_device),
            rendering_surface,
            &resource_manager,
        )?;

        let simulator = PhysicsSimulator::new(config.physics)?;

        let gizmo_manager = GizmoManager::new(config.gizmo);

        let (motion_controller, orientation_controller) =
            impact_controller::create_controllers(config.controller);

        let game_loop_controller = GameLoopController::new(config.game_loop);

        let engine = Self {
            app,
            graphics_device,
            component_registry: RwLock::new(component_registry),
            ecs_world: RwLock::new(ecs_world),
            entity_stager: Mutex::new(EntityStager::new()),
            renderer: RwLock::new(renderer),
            resource_manager: RwLock::new(resource_manager),
            scene: RwLock::new(scene),
            simulator: RwLock::new(simulator),
            gizmo_manager: RwLock::new(gizmo_manager),
            motion_controller: motion_controller.map(Mutex::new),
            orientation_controller: orientation_controller.map(Mutex::new),
            screen_capturer: ScreenCapturer::new(),
            task_timer: TaskTimer::new(config.instrumentation.task_timing_enabled),
            metrics: RwLock::new(EngineMetrics::default()),
            game_loop_controller: Mutex::new(game_loop_controller),
            controls_enabled: AtomicBool::new(false),
            shutdown_requested: AtomicBool::new(false),
        };

        Ok(engine)
    }

    /// Returns a reference to the [`Application`].
    pub(crate) fn app(&self) -> &dyn Application {
        self.app.as_ref()
    }

    /// Returns a reference to the [`GraphicsDevice`].
    pub fn graphics_device(&self) -> &GraphicsDevice {
        &self.graphics_device
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

    /// Returns a reference to the [`EntityStager`], guarded by a [`Mutex`].
    pub fn entity_stager(&self) -> &Mutex<EntityStager> {
        &self.entity_stager
    }

    /// Returns a reference to the [`RenderingSystem`], guarded by a [`RwLock`].
    pub fn renderer(&self) -> &RwLock<RenderingSystem> {
        &self.renderer
    }

    /// Returns a reference to the [`ResourceManager`], guarded by a [`RwLock`].
    pub fn resource_manager(&self) -> &RwLock<ResourceManager> {
        &self.resource_manager
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

    /// Returns a reference to the [`GizmoManager`], guarded by a [`RwLock`].
    pub fn gizmo_manager(&self) -> &RwLock<GizmoManager> {
        &self.gizmo_manager
    }

    /// Returns a reference to the [`MotionController`], guarded by a [`Mutex`],
    /// or [`None`] if there is no motion controller.
    pub fn motion_controller(&self) -> Option<&Mutex<Box<dyn MotionController>>> {
        self.motion_controller.as_ref()
    }

    /// Returns a reference to the [`ScreenCapturer`].
    pub fn screen_capturer(&self) -> &ScreenCapturer {
        &self.screen_capturer
    }

    /// Returns a reference to the [`TaskTimer`].
    pub fn task_timer(&self) -> &TaskTimer {
        &self.task_timer
    }

    /// Returns the current [`EngineMetrics`], wrapped in a read guard.
    pub fn metrics(&self) -> RwLockReadGuard<'_, EngineMetrics> {
        self.metrics.read()
    }

    /// Returns a reference to the [`GameLoopController`], guarded by a
    /// [`Mutex`].
    pub fn game_loop_controller(&self) -> &Mutex<GameLoopController> {
        &self.game_loop_controller
    }

    /// Captures and saves a screenshot to the specified path, or, if not
    /// specified, to a timestamped PNG file in the current directory.
    pub fn capture_screenshot(&self, output_path: Option<&Path>) -> Result<()> {
        self.screen_capturer.request_screenshot_save();
        self.screen_capturer
            .save_screenshot_if_requested(self.renderer(), output_path)
    }

    /// Captures and saves any screenshots or related textures requested through
    /// the [`ScreenCapturer`].
    pub fn save_requested_screenshots(&self) -> Result<()> {
        self.screen_capturer
            .save_screenshot_if_requested(self.renderer(), None)?;

        self.screen_capturer
            .save_omnidirectional_light_shadow_maps_if_requested(self.renderer())?;

        self.screen_capturer
            .save_unidirectional_light_shadow_maps_if_requested(self.renderer())
    }

    /// Sets a new size for the rendering surface and updates
    /// the aspect ratio of all cameras.
    pub fn resize_rendering_surface(&self, new_width: NonZeroU32, new_height: NonZeroU32) {
        let mut renderer = self.renderer().write();

        renderer.resize_rendering_surface(new_width, new_height);

        let new_aspect_ratio = renderer.rendering_surface().surface_aspect_ratio();

        drop(renderer);

        let render_resources_desynchronized = self
            .scene()
            .read()
            .handle_aspect_ratio_changed(new_aspect_ratio);

        if render_resources_desynchronized.is_yes() {
            self.renderer()
                .read()
                .declare_render_resources_desynchronized();
        }
    }

    pub fn update_pixels_per_point(&self, pixels_per_point: f64) {
        self.renderer()
            .write()
            .update_pixels_per_point(pixels_per_point);
    }

    /// Updates the orientation controller with the given mouse displacement.
    pub fn update_orientation_controller(&self, mouse_displacement: (f64, f64)) {
        if !self.controls_enabled() {
            return;
        }
        if let Some(orientation_controller) = &self.orientation_controller {
            impact_log::trace!(
                "Updating orientation controller by mouse delta ({}, {})",
                mouse_displacement.0,
                mouse_displacement.1
            );

            let (_, window_height) = self
                .renderer
                .read()
                .rendering_surface()
                .surface_dimensions();

            orientation_controller
                .lock()
                .update_orientation_change(window_height, mouse_displacement);
        }
    }

    /// Updates the orientations and motion of all controlled entities.
    pub fn update_controlled_entities(&self) {
        if !self.controls_enabled() {
            return;
        }
        let ecs_world = self.ecs_world().read();
        let simulator = self.simulator.read();
        let mut rigid_body_manager = simulator.rigid_body_manager().write();
        let time_step_duration = simulator.scaled_time_step_duration();

        if let Some(orientation_controller) = &self.orientation_controller {
            impact_controller::systems::update_controlled_entity_angular_velocities(
                &ecs_world,
                &mut rigid_body_manager,
                orientation_controller.lock().as_mut(),
                time_step_duration,
            );
        }

        if let Some(motion_controller) = &self.motion_controller {
            impact_controller::systems::update_controlled_entity_velocities(
                &ecs_world,
                &mut rigid_body_manager,
                motion_controller.lock().as_ref(),
            );
        }
    }

    /// Resets the scene, ECS world and physics simulator to the initial empty
    /// state and sets the simulation time to zero.
    pub fn reset_world(&self) {
        self.ecs_world.write().remove_all_entities();
        self.scene.read().clear();
        self.simulator.write().reset();
        self.renderer
            .read()
            .declare_render_resources_desynchronized();
    }

    pub fn controls_enabled(&self) -> bool {
        self.controls_enabled.load(Ordering::Relaxed)
    }

    pub fn set_controls_enabled(&self, enabled: bool) {
        self.controls_enabled.store(enabled, Ordering::Relaxed);

        if !enabled {
            let ecs_world = self.ecs_world.read();
            let simulator = self.simulator.read();
            let mut rigid_body_manager = simulator.rigid_body_manager().write();

            if let Some(motion_controller) = &self.motion_controller {
                let mut motion_controller = motion_controller.lock();
                motion_controller.stop();

                impact_controller::systems::update_controlled_entity_velocities(
                    &ecs_world,
                    &mut rigid_body_manager,
                    motion_controller.as_ref(),
                );
            }

            if let Some(orientation_controller) = &self.orientation_controller {
                let mut orientation_controller = orientation_controller.lock();
                orientation_controller.reset_orientation_change();

                impact_controller::systems::update_controlled_entity_angular_velocities(
                    &ecs_world,
                    &mut rigid_body_manager,
                    orientation_controller.as_mut(),
                    simulator.time_step_duration(),
                );
            }
        }
    }

    pub fn gather_metrics_after_completed_frame(&self, smooth_frame_duration: Duration) {
        let mut metrics = self.metrics.write();
        metrics.current_smooth_frame_duration = smooth_frame_duration;

        self.task_timer
            .report_task_execution_times(&mut metrics.last_task_execution_times);

        self.simulator()
            .write()
            .update_time_step_duration(&smooth_frame_duration);
    }

    pub fn shutdown_requested(&self) -> bool {
        self.shutdown_requested.load(Ordering::Relaxed)
    }

    pub fn request_shutdown(&self) {
        self.shutdown_requested.store(true, Ordering::Relaxed);
    }

    pub fn execute_command(&self, command: EngineCommand) -> Result<()> {
        command::execute_engine_command(self, command)
    }

    /// Identifies errors that need special handling in the given set of task
    /// errors and handles them.
    pub fn handle_task_errors(&self, task_errors: &mut ThreadPoolTaskErrors) {
        self.renderer.read().handle_task_errors(task_errors);
    }

    fn with_component_mut<C: Component, R>(
        &self,
        entity_id: EntityID,
        f: impl FnOnce(&mut C) -> Result<R>,
    ) -> Result<R> {
        let ecs_world = self.ecs_world.read();

        let entity_entry = ecs_world
            .get_entity(entity_id)
            .ok_or_else(|| anyhow!("Missing entity with ID {:?}", entity_id))?;

        let mut component_entry = entity_entry.get_component_mut().ok_or_else(|| {
            anyhow!(
                "Missing component {:?} for entity with ID {:?}",
                C::component_id(),
                entity_id
            )
        })?;

        let component: &mut C = component_entry.access();

        f(component)
    }
}

impl EngineConfig {
    /// Parses the configuration from the RON file at the given path and
    /// resolves any specified paths.
    pub fn from_ron_file(file_path: impl AsRef<Path>) -> Result<Self> {
        let file_path = file_path.as_ref();
        let mut config: Self = impact_io::parse_ron_file(file_path)?;
        if let Some(root_path) = file_path.parent() {
            config.resolve_paths(root_path);
        }
        Ok(config)
    }

    /// Resolves all paths in the configuration by prepending the given root
    /// path to all paths.
    fn resolve_paths(&mut self, root_path: &Path) {
        self.resources.resolve_paths(root_path);
        self.physics.resolve_paths(root_path);
        self.voxel.resolve_paths(root_path);
    }
}
