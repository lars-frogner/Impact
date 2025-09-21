//! Manager for all systems and data in the engine.

pub mod api;
pub mod entity;
pub mod game_loop;

#[cfg(feature = "window")]
pub mod window;

use crate::{
    application::Application,
    command::{self, EngineCommandQueues},
    game_loop::{GameLoopConfig, GameLoopController},
    gizmo::{self, GizmoConfig, GizmoManager},
    gpu::GraphicsContext,
    input::{
        InputConfig, InputEvent, InputManager,
        mouse::{MouseDragEvent, MouseMotionEvent},
    },
    instrumentation::{EngineMetrics, InstrumentationConfig, timing::TaskTimer},
    lock_order::{OrderedMutex, OrderedRwLock},
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
use impact_scene::{camera::CameraContext, model::ModelInstanceManager};
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
    input_manager: Mutex<InputManager>,
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
    command_queues: EngineCommandQueues,
    screen_capturer: ScreenCapturer,
    task_timer: TaskTimer,
    controls_enabled: AtomicBool,
    shutdown_requested: AtomicBool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct EngineConfig {
    pub game_loop: GameLoopConfig,
    pub input: InputConfig,
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
    pub(crate) fn new(
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

        let camera_context = CameraContext {
            aspect_ratio: graphics.surface.surface_aspect_ratio(),
            jitter_enabled: config.rendering.temporal_anti_aliasing.enabled,
        };
        let scene = Scene::new(camera_context, model_instance_manager);

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

        let input_manager = InputManager::new(config.input);

        let engine = Self {
            app,
            graphics_device,
            component_metadata_registry,
            game_loop_controller: RwLock::new(game_loop_controller),
            input_manager: Mutex::new(input_manager),
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
            command_queues: EngineCommandQueues::default(),
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
    pub(crate) fn graphics_device(&self) -> &GraphicsDevice {
        &self.graphics_device
    }

    /// Returns a reference to the [`GraphicsDevice`].
    pub(crate) fn component_metadata_registry(&self) -> &ComponentMetadataRegistry {
        &self.component_metadata_registry
    }

    /// Returns a reference to the [`GameLoopController`], guarded by a
    /// [`RwLock`].
    pub(crate) fn game_loop_controller(&self) -> &RwLock<GameLoopController> {
        &self.game_loop_controller
    }

    /// Returns a reference to the [`EntityStager`], guarded by a [`Mutex`].
    pub(crate) fn entity_stager(&self) -> &Mutex<EntityStager> {
        &self.entity_stager
    }

    /// Returns a reference to the ECS [`World`](impact_ecs::world::World),
    /// guarded by a [`RwLock`].
    pub(crate) fn ecs_world(&self) -> &RwLock<ECSWorld> {
        &self.ecs_world
    }

    /// Returns a reference to the [`ResourceManager`], guarded by a [`RwLock`].
    pub(crate) fn resource_manager(&self) -> &RwLock<ResourceManager> {
        &self.resource_manager
    }

    /// Returns a reference to the [`Scene`], guarded by a [`RwLock`].
    pub(crate) fn scene(&self) -> &RwLock<Scene> {
        &self.scene
    }

    /// Returns a reference to the [`PhysicsSimulator`], guarded by a
    /// [`RwLock`].
    pub(crate) fn simulator(&self) -> &RwLock<PhysicsSimulator> {
        &self.simulator
    }

    /// Returns a reference to the [`RenderingSystem`], guarded by a [`RwLock`].
    pub(crate) fn renderer(&self) -> &RwLock<RenderingSystem> {
        &self.renderer
    }

    /// Returns a reference to the [`MotionController`], guarded by a [`Mutex`],
    /// or [`None`] if there is no motion controller.
    pub(crate) fn motion_controller(&self) -> Option<&Mutex<Box<dyn MotionController>>> {
        self.motion_controller.as_ref()
    }

    /// Returns a reference to the [`GizmoManager`], guarded by a [`RwLock`].
    pub(crate) fn gizmo_manager(&self) -> &RwLock<GizmoManager> {
        &self.gizmo_manager
    }

    /// Returns the current [`EngineMetrics`], guarded by a [`RwLock`].
    pub(crate) fn metrics(&self) -> &RwLock<EngineMetrics> {
        &self.metrics
    }

    /// Returns a reference to the [`ScreenCapturer`].
    pub(crate) fn screen_capturer(&self) -> &ScreenCapturer {
        &self.screen_capturer
    }

    /// Returns a reference to the [`TaskTimer`].
    pub(crate) fn task_timer(&self) -> &TaskTimer {
        &self.task_timer
    }

    /// Captures and saves any screenshots or related textures requested through
    /// the [`ScreenCapturer`].
    pub(crate) fn save_requested_screenshots<A>(&self, arena: A) -> Result<()>
    where
        A: Copy + Allocator,
    {
        let current_frame_number = self.game_loop_controller.oread().iteration();

        // The screenshot we can save now represents the previous frame
        let frame_number_for_image = current_frame_number.saturating_sub(1);

        self.screen_capturer.save_screenshot_if_requested(
            arena,
            self.renderer(),
            frame_number_for_image,
        )?;

        self.screen_capturer
            .save_omnidirectional_light_shadow_maps_if_requested(
                arena,
                self.renderer(),
                frame_number_for_image,
            )?;

        self.screen_capturer
            .save_unidirectional_light_shadow_maps_if_requested(
                arena,
                self.renderer(),
                frame_number_for_image,
            )
    }

    /// Sets a new size for the rendering surface and updates
    /// the aspect ratio of all cameras.
    pub(crate) fn resize_rendering_surface(&self, new_width: NonZeroU32, new_height: NonZeroU32) {
        let mut renderer = self.renderer().owrite();

        renderer.resize_rendering_surface(new_width, new_height);

        let new_aspect_ratio = renderer.rendering_surface().surface_aspect_ratio();

        drop(renderer);

        self.scene()
            .oread()
            .handle_aspect_ratio_changed(new_aspect_ratio);
    }

    pub(crate) fn update_pixels_per_point(&self, pixels_per_point: f64) {
        self.renderer()
            .owrite()
            .update_pixels_per_point(pixels_per_point);
    }

    pub(crate) fn handle_queued_input_events(&self) -> Result<()> {
        let mut input_manager = self.input_manager.olock();
        let input_manager = &mut **input_manager;
        for event in input_manager.event_queue.drain(..) {
            match event {
                InputEvent::Keyboard(event) => {
                    self.app().handle_keyboard_event(event)?;
                }
                InputEvent::MouseButton(event) => {
                    input_manager.state.record_mouse_button_event(event);
                    self.app().handle_mouse_button_event(event)?;
                }
                InputEvent::MouseMotion(MouseMotionEvent {
                    ang_delta_x,
                    ang_delta_y,
                }) => {
                    self.update_orientation_controller(ang_delta_x, ang_delta_y);
                    self.app().handle_mouse_drag_event(MouseDragEvent {
                        ang_delta_x,
                        ang_delta_y,
                        pressed: input_manager.state.pressed_mouse_buttons,
                        cursor: input_manager.state.cursor_direction,
                    })?;
                }
                InputEvent::MouseScroll(event) => {
                    self.app().handle_mouse_scroll_event(event)?;
                }
                InputEvent::CursorMoved(event) => {
                    input_manager.state.record_cursor_moved_event(event);
                }
            }
        }
        Ok(())
    }

    /// Updates the orientation controller with the given angular mouse
    /// displacement.
    pub(crate) fn update_orientation_controller(&self, delta_x: f64, delta_y: f64) {
        if !self.controls_enabled() {
            return;
        }
        if let Some(orientation_controller) = &self.orientation_controller {
            impact_log::trace!(
                "Updating orientation controller by angular mouse deltas ({delta_x}, {delta_y})",
            );
            orientation_controller
                .olock()
                .update_orientation_change(delta_x, delta_y);
        }
    }

    /// Updates the motion of all controlled entities.
    pub(crate) fn update_controlled_entity_motion(&self) {
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
                orientation_controller.olock().as_mut(),
                time_step_duration,
            );

            if let Some(motion_controller) = &self.motion_controller {
                impact_controller::systems::update_controlled_entity_velocities(
                    &ecs_world,
                    &mut rigid_body_manager,
                    motion_controller.olock().as_ref(),
                );
            }
        } else if let Some(motion_controller) = &self.motion_controller {
            let ecs_world = self.ecs_world.oread();
            let simulator = self.simulator.oread();
            let mut rigid_body_manager = simulator.rigid_body_manager().owrite();

            impact_controller::systems::update_controlled_entity_velocities(
                &ecs_world,
                &mut rigid_body_manager,
                motion_controller.olock().as_ref(),
            );
        }
    }

    pub(crate) fn set_controls_enabled(&self, enabled: bool) {
        let were_enabled = self.controls_enabled.swap(enabled, Ordering::Relaxed);

        if were_enabled && !enabled {
            let ecs_world = self.ecs_world.oread();
            let simulator = self.simulator.oread();
            let mut rigid_body_manager = simulator.rigid_body_manager().owrite();

            if let Some(motion_controller) = &self.motion_controller {
                let mut motion_controller = motion_controller.olock();
                motion_controller.stop();

                impact_controller::systems::update_controlled_entity_velocities(
                    &ecs_world,
                    &mut rigid_body_manager,
                    motion_controller.as_ref(),
                );
            }

            if let Some(orientation_controller) = &self.orientation_controller {
                let mut orientation_controller = orientation_controller.olock();
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

    pub(crate) fn gather_metrics_after_completed_frame(&self, smooth_frame_duration: Duration) {
        let mut metrics = self.metrics.owrite();

        metrics.current_smooth_frame_duration = smooth_frame_duration;

        self.task_timer
            .report_task_execution_times(&mut metrics.last_task_execution_times);
    }

    pub(crate) fn update_simulation_time_step_duration(&self, smooth_frame_duration: Duration) {
        let mut simulator = self.simulator.owrite();
        simulator.update_time_step_duration(&smooth_frame_duration);
    }

    pub(crate) fn shutdown_requested(&self) -> bool {
        self.shutdown_requested.load(Ordering::Relaxed)
    }

    pub(crate) fn request_shutdown(&self) {
        self.shutdown_requested.store(true, Ordering::Relaxed);
    }

    pub(crate) fn execute_enqueued_scene_commands(&self) -> Result<()> {
        self.command_queues
            .scene
            .try_execute_commands(|command| command::execute_scene_command(self, command))
    }

    pub(crate) fn execute_enqueued_controller_commands(&self) -> Result<()> {
        self.command_queues
            .controller
            .try_execute_commands(|command| command::execute_controller_command(self, command))
    }

    pub(crate) fn execute_enqueued_rendering_commands(&self) -> Result<()> {
        self.command_queues
            .rendering
            .try_execute_commands(|command| command::execute_rendering_command(self, command))
    }

    pub(crate) fn execute_enqueued_physics_commands(&self) -> Result<()> {
        self.command_queues
            .physics
            .try_execute_commands(|command| command::execute_physics_command(self, command))
    }

    pub(crate) fn execute_enqueued_control_commands(&self) -> Result<()> {
        self.command_queues
            .control
            .try_execute_commands(|command| command::execute_control_command(self, command))
    }

    pub(crate) fn execute_enqueued_capture_commands(&self) -> Result<()> {
        self.command_queues
            .capture
            .try_execute_commands(|command| command::execute_capture_command(self, command))
    }

    pub(crate) fn execute_enqueued_instrumentation_commands(&self) -> Result<()> {
        self.command_queues
            .instrumentation
            .try_execute_commands(|command| command::execute_instrumentation_command(self, command))
    }

    pub(crate) fn execute_enqueued_game_loop_commands(&self) -> Result<()> {
        self.command_queues
            .game_loop
            .try_execute_commands(|command| command::execute_game_loop_command(self, command))
    }

    pub(crate) fn execute_enqueued_gizmo_commands(&self) -> Result<()> {
        self.command_queues
            .gizmo
            .try_execute_commands(|command| command::execute_gizmo_command(self, command))
    }

    pub(crate) fn execute_enqueued_system_commands(&self) -> Result<()> {
        self.command_queues
            .system
            .try_execute_commands(|command| command::execute_system_command(self, command))
    }

    /// Identifies errors that need special handling in the given set of task
    /// errors and handles them.
    pub(crate) fn handle_task_errors(&self, task_errors: &mut ThreadPoolTaskErrors) {
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
