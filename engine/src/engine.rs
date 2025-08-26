//! Manager for all systems and data in the engine.

pub mod api;
pub mod entity;
pub mod game_loop;

#[cfg(feature = "window")]
pub mod window;

use crate::{
    application::Application,
    command::{self, EngineCommand, queue::CommandQueue},
    game_loop::{GameLoopConfig, GameLoopController},
    gizmo::{self, GizmoConfig, GizmoManager},
    gpu::GraphicsContext,
    instrumentation::{EngineMetrics, InstrumentationConfig, timing::TaskTimer},
    lock_order::OrderedRwLock,
    physics::{PhysicsConfig, PhysicsSimulator},
    rendering::{
        RenderingConfig, RenderingSystem,
        screen_capture::{ScreenCaptureConfig, ScreenCapturer},
    },
    resource::{ResourceConfig, ResourceManager},
    scene::Scene,
};
use allocator_api2::alloc::Allocator;
use anyhow::{Result, anyhow};
use impact_controller::{ControllerConfig, MotionController, OrientationController};
use impact_ecs::{
    component::Component,
    metadata::ComponentMetadataRegistry,
    world::{EntityID, EntityStager, World as ECSWorld},
};
use impact_gpu::device::GraphicsDevice;
use impact_scene::model::ModelInstanceManager;
use impact_texture::{SamplerRegistry, TextureRegistry};
use impact_thread::ThreadPoolTaskErrors;
use impact_voxel::{VoxelConfig, voxel_types::VoxelTypeRegistry};
use parking_lot::{Mutex, RwLock};
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
    component_metadata_registry: ComponentMetadataRegistry,
    game_loop_controller: RwLock<GameLoopController>,
    entity_stager: Mutex<EntityStager>,
    ecs_world: RwLock<ECSWorld>,
    resource_manager: RwLock<ResourceManager>,
    scene: RwLock<Scene>,
    simulator: RwLock<PhysicsSimulator>,
    renderer: RwLock<RenderingSystem>,
    motion_controller: Option<Mutex<Box<dyn MotionController>>>,
    orientation_controller: Option<Mutex<Box<dyn OrientationController>>>,
    gizmo_manager: RwLock<GizmoManager>,
    metrics: RwLock<EngineMetrics>,
    command_queue: CommandQueue<EngineCommand>,
    screen_capturer: ScreenCapturer,
    task_timer: TaskTimer,
    controls_enabled: AtomicBool,
    shutdown_requested: AtomicBool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct EngineConfig {
    pub game_loop: GameLoopConfig,
    pub ecs: ECSConfig,
    pub resources: ResourceConfig,
    pub voxel: VoxelConfig,
    pub physics: PhysicsConfig,
    pub rendering: RenderingConfig,
    pub controller: ControllerConfig,
    pub gizmo: GizmoConfig,
    pub instrumentation: InstrumentationConfig,
    pub screen_capture: ScreenCaptureConfig,
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
        let mut component_metadata_registry = ComponentMetadataRegistry::new();
        crate::component::register_metadata_for_all_components(&mut component_metadata_registry)?;

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

        let mut model_instance_manager = ModelInstanceManager::new();
        impact_model::register_model_feature_types(&mut model_instance_manager);
        impact_material::register_material_feature_types(&mut model_instance_manager);
        impact_voxel::register_voxel_feature_types(&mut model_instance_manager);
        gizmo::initialize_buffers_for_gizmo_models(&mut model_instance_manager);

        let scene = Scene::new(model_instance_manager);

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
            component_metadata_registry,
            game_loop_controller: RwLock::new(game_loop_controller),
            entity_stager: Mutex::new(EntityStager::new()),
            ecs_world: RwLock::new(ecs_world),
            resource_manager: RwLock::new(resource_manager),
            scene: RwLock::new(scene),
            simulator: RwLock::new(simulator),
            renderer: RwLock::new(renderer),
            motion_controller: motion_controller.map(Mutex::new),
            orientation_controller: orientation_controller.map(Mutex::new),
            gizmo_manager: RwLock::new(gizmo_manager),
            metrics: RwLock::new(EngineMetrics::default()),
            command_queue: CommandQueue::new(),
            screen_capturer: ScreenCapturer::new(config.screen_capture),
            task_timer: TaskTimer::new(config.instrumentation.task_timing_enabled),
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

    /// Returns a reference to the [`GraphicsDevice`].
    pub fn component_metadata_registry(&self) -> &ComponentMetadataRegistry {
        &self.component_metadata_registry
    }

    /// Returns a reference to the [`GameLoopController`], guarded by a
    /// [`RwLock`].
    pub fn game_loop_controller(&self) -> &RwLock<GameLoopController> {
        &self.game_loop_controller
    }

    /// Returns a reference to the [`EntityStager`], guarded by a [`Mutex`].
    pub fn entity_stager(&self) -> &Mutex<EntityStager> {
        &self.entity_stager
    }

    /// Returns a reference to the ECS [`World`](impact_ecs::world::World),
    /// guarded by a [`RwLock`].
    pub fn ecs_world(&self) -> &RwLock<ECSWorld> {
        &self.ecs_world
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

    /// Returns a reference to the [`RenderingSystem`], guarded by a [`RwLock`].
    pub fn renderer(&self) -> &RwLock<RenderingSystem> {
        &self.renderer
    }

    /// Returns a reference to the [`MotionController`], guarded by a [`Mutex`],
    /// or [`None`] if there is no motion controller.
    pub fn motion_controller(&self) -> Option<&Mutex<Box<dyn MotionController>>> {
        self.motion_controller.as_ref()
    }

    /// Returns a reference to the [`GizmoManager`], guarded by a [`RwLock`].
    pub fn gizmo_manager(&self) -> &RwLock<GizmoManager> {
        &self.gizmo_manager
    }

    /// Returns the current [`EngineMetrics`], guarded by a [`RwLock`].
    pub fn metrics(&self) -> &RwLock<EngineMetrics> {
        &self.metrics
    }

    /// Returns a reference to the [`ScreenCapturer`].
    pub fn screen_capturer(&self) -> &ScreenCapturer {
        &self.screen_capturer
    }

    /// Returns a reference to the [`TaskTimer`].
    pub fn task_timer(&self) -> &TaskTimer {
        &self.task_timer
    }

    /// Captures and saves any screenshots or related textures requested through
    /// the [`ScreenCapturer`].
    pub fn save_requested_screenshots<A>(&self, arena: A) -> Result<()>
    where
        A: Copy + Allocator,
    {
        let frame_number = self.game_loop_controller.oread().iteration();

        self.screen_capturer
            .save_screenshot_if_requested(arena, self.renderer(), frame_number)?;

        self.screen_capturer
            .save_omnidirectional_light_shadow_maps_if_requested(
                arena,
                self.renderer(),
                frame_number,
            )?;

        self.screen_capturer
            .save_unidirectional_light_shadow_maps_if_requested(
                arena,
                self.renderer(),
                frame_number,
            )
    }

    /// Sets a new size for the rendering surface and updates
    /// the aspect ratio of all cameras.
    pub fn resize_rendering_surface(&self, new_width: NonZeroU32, new_height: NonZeroU32) {
        let mut renderer = self.renderer().owrite();

        renderer.resize_rendering_surface(new_width, new_height);

        let new_aspect_ratio = renderer.rendering_surface().surface_aspect_ratio();

        drop(renderer);

        self.scene()
            .oread()
            .handle_aspect_ratio_changed(new_aspect_ratio);
    }

    pub fn update_pixels_per_point(&self, pixels_per_point: f64) {
        self.renderer()
            .owrite()
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
                .oread()
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

        if let Some(orientation_controller) = &self.orientation_controller {
            let ecs_world = self.ecs_world.oread();
            let simulator = self.simulator.oread();
            let mut rigid_body_manager = simulator.rigid_body_manager().owrite();
            let time_step_duration = simulator.scaled_time_step_duration();

            impact_controller::systems::update_controlled_entity_angular_velocities(
                &ecs_world,
                &mut rigid_body_manager,
                orientation_controller.lock().as_mut(),
                time_step_duration,
            );

            if let Some(motion_controller) = &self.motion_controller {
                impact_controller::systems::update_controlled_entity_velocities(
                    &ecs_world,
                    &mut rigid_body_manager,
                    motion_controller.lock().as_ref(),
                );
            }
        } else if let Some(motion_controller) = &self.motion_controller {
            let ecs_world = self.ecs_world.oread();
            let simulator = self.simulator.oread();
            let mut rigid_body_manager = simulator.rigid_body_manager().owrite();

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
        self.ecs_world.owrite().remove_all_entities();
        self.scene.oread().clear();
        self.simulator.owrite().reset();
    }

    pub fn set_controls_enabled(&self, enabled: bool) {
        self.controls_enabled.store(enabled, Ordering::Relaxed);

        if !enabled {
            let ecs_world = self.ecs_world.oread();
            let simulator = self.simulator.oread();
            let mut rigid_body_manager = simulator.rigid_body_manager().owrite();

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
        let mut metrics = self.metrics.owrite();

        metrics.current_smooth_frame_duration = smooth_frame_duration;

        self.task_timer
            .report_task_execution_times(&mut metrics.last_task_execution_times);

        drop(metrics);

        let mut simulator = self.simulator.owrite();
        simulator.update_time_step_duration(&smooth_frame_duration);
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

    pub fn execute_enqueued_commands(&self) -> Result<()> {
        self.command_queue
            .try_execute_commands(|command| command::execute_engine_command(self, command))
    }

    /// Identifies errors that need special handling in the given set of task
    /// errors and handles them.
    pub fn handle_task_errors(&self, task_errors: &mut ThreadPoolTaskErrors) {
        self.renderer.oread().handle_task_errors(task_errors);
    }

    fn with_component_mut<C: Component, R>(
        &self,
        entity_id: EntityID,
        f: impl FnOnce(&mut C) -> Result<R>,
    ) -> Result<R> {
        let ecs_world = self.ecs_world.oread();

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
