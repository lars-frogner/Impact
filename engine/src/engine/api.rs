//! Public engine API.

use super::Engine;
use crate::{
    command::{AdminCommand, UserCommand},
    gizmo::{GizmoParameters, GizmoType, GizmoVisibilities, GizmoVisibility},
    lock_order::{OrderedMutex, OrderedRwLock},
    physics::SimulatorConfig,
    setup,
};
use anyhow::Result;
use impact_ecs::{
    archetype::ArchetypeComponents,
    component::{ComponentArray, SingleInstance},
    world::EntityID,
};
use impact_physics::{constraint::solver::ConstraintSolverConfig, fph};
use impact_rendering::{
    BasicRenderingConfig,
    attachment::RenderAttachmentQuantity,
    postprocessing::{
        ambient_occlusion::AmbientOcclusionConfig,
        capturing::{
            CameraSettings, average_luminance::AverageLuminanceComputationConfig,
            bloom::BloomConfig, dynamic_range_compression::DynamicRangeCompressionConfig,
        },
        temporal_anti_aliasing::TemporalAntiAliasingConfig,
    },
};
use std::sync::atomic::Ordering;
use std::time::Duration;

impl Engine {
    pub fn stage_entity_for_creation_with_id<A, E>(
        &self,
        entity_id: EntityID,
        components: impl TryInto<SingleInstance<ArchetypeComponents<A>>, Error = E>,
    ) -> Result<()>
    where
        A: ComponentArray,
        E: Into<anyhow::Error>,
    {
        self.entity_stager
            .olock()
            .stage_entity_for_creation_with_id(entity_id, components)
    }

    pub fn stage_entity_for_creation<A, E>(
        &self,
        components: impl TryInto<SingleInstance<ArchetypeComponents<A>>, Error = E>,
    ) -> Result<()>
    where
        A: ComponentArray,
        E: Into<anyhow::Error>,
    {
        self.entity_stager
            .olock()
            .stage_entity_for_creation(components)
    }

    pub fn stage_entities_for_creation<A, E>(
        &self,
        components: impl TryInto<ArchetypeComponents<A>, Error = E>,
    ) -> Result<()>
    where
        A: ComponentArray,
        E: Into<anyhow::Error>,
    {
        self.entity_stager
            .olock()
            .stage_entities_for_creation(components)
    }

    pub fn stage_entity_for_removal(&self, entity_id: EntityID) {
        self.entity_stager
            .olock()
            .stage_entity_for_removal(entity_id);
    }

    pub fn create_entity_with_id<A, E>(
        &self,
        entity_id: EntityID,
        components: impl TryInto<SingleInstance<ArchetypeComponents<A>>, Error = E>,
    ) -> Result<()>
    where
        A: ComponentArray,
        E: Into<anyhow::Error>,
    {
        let mut components = components
            .try_into()
            .map_err(E::into)?
            .into_inner()
            .into_storage();

        setup::perform_setup_for_new_entities(self, &mut components)?;

        self.ecs_world
            .owrite()
            .create_entity_with_id(entity_id, SingleInstance::new(components))
    }

    pub fn create_entity<A, E>(
        &self,
        components: impl TryInto<SingleInstance<ArchetypeComponents<A>>, Error = E>,
    ) -> Result<EntityID>
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
    ) -> Result<Vec<EntityID>>
    where
        A: ComponentArray,
        E: Into<anyhow::Error>,
    {
        let mut components = components.try_into().map_err(E::into)?.into_storage();
        setup::perform_setup_for_new_entities(self, &mut components)?;
        self.ecs_world.owrite().create_entities(components)
    }

    pub fn remove_entity(&self, entity_id: EntityID) -> Result<()> {
        let mut ecs_world = self.ecs_world.owrite();
        setup::perform_cleanup_for_removed_entity(self, &ecs_world.entity(entity_id))?;
        ecs_world.remove_entity(entity_id)
    }

    pub fn enqueue_user_command(&self, command: UserCommand) {
        match command {
            UserCommand::Scene(command) => {
                self.command_queues.scene.enqueue_command(command);
            }
            UserCommand::Controller(command) => {
                self.command_queues.controller.enqueue_command(command);
            }
        }
    }

    pub fn enqueue_admin_command(&self, command: AdminCommand) {
        match command {
            AdminCommand::Rendering(command) => {
                self.command_queues.rendering.enqueue_command(command);
            }
            AdminCommand::Physics(command) => {
                self.command_queues.physics.enqueue_command(command);
            }
            AdminCommand::Control(command) => {
                self.command_queues.control.enqueue_command(command);
            }
            AdminCommand::Capture(command) => {
                self.command_queues.capture.enqueue_command(command);
            }
            AdminCommand::Instrumentation(command) => {
                self.command_queues.instrumentation.enqueue_command(command);
            }
            AdminCommand::GameLoop(command) => {
                self.command_queues.game_loop.enqueue_command(command);
            }
            AdminCommand::Gizmo(command) => {
                self.command_queues.gizmo.enqueue_command(command);
            }
            AdminCommand::System(command) => {
                self.command_queues.system.enqueue_command(command);
            }
        }
    }

    /// Resets the scene, ECS world and physics simulator to the initial empty
    /// state and sets the simulation time to zero.
    pub fn reset_world(&self) {
        impact_log::info!("Resetting world");
        self.ecs_world.owrite().remove_all_entities();
        self.scene.oread().clear();
        self.simulator.owrite().reset();
    }

    pub fn controls_enabled(&self) -> bool {
        self.controls_enabled.load(Ordering::Relaxed)
    }

    /// Returns the current gizmo visibilities.
    pub fn gizmo_visibilities(&self) -> GizmoVisibilities {
        self.gizmo_manager.oread().visibilities().clone()
    }

    /// Returns the current gizmo parameters.
    pub fn gizmo_parameters(&self) -> GizmoParameters {
        self.gizmo_manager.oread().parameters().clone()
    }

    /// Returns the visibility state for a specific gizmo type.
    pub fn gizmo_visibility(&self, gizmo_type: GizmoType) -> GizmoVisibility {
        self.gizmo_manager
            .oread()
            .visibilities()
            .get_for(gizmo_type)
    }

    /// Returns the current basic rendering configuration.
    pub fn basic_rendering_config(&self) -> BasicRenderingConfig {
        self.renderer().oread().basic_config().clone()
    }

    /// Returns whether shadow mapping is enabled.
    pub fn shadow_mapping_enabled(&self) -> bool {
        self.renderer().oread().shadow_mapping_config().enabled
    }

    /// Returns the current ambient occlusion configuration.
    pub fn ambient_occlusion_config(&self) -> AmbientOcclusionConfig {
        self.renderer()
            .oread()
            .postprocessor()
            .oread()
            .ambient_occlusion_config()
            .clone()
    }

    /// Returns whether ambient occlusion is enabled.
    pub fn ambient_occlusion_enabled(&self) -> bool {
        self.renderer()
            .oread()
            .postprocessor()
            .oread()
            .ambient_occlusion_config()
            .enabled
    }

    /// Returns the current temporal anti-aliasing configuration.
    pub fn temporal_anti_aliasing_config(&self) -> TemporalAntiAliasingConfig {
        self.renderer()
            .oread()
            .postprocessor()
            .oread()
            .temporal_anti_aliasing_config()
            .clone()
    }

    /// Returns whether temporal anti-aliasing is enabled.
    pub fn temporal_anti_aliasing_enabled(&self) -> bool {
        self.renderer()
            .oread()
            .postprocessor()
            .oread()
            .temporal_anti_aliasing_config()
            .enabled
    }

    /// Returns the current camera settings.
    pub fn camera_settings(&self) -> CameraSettings {
        self.renderer()
            .oread()
            .postprocessor()
            .oread()
            .capturing_camera()
            .settings()
            .clone()
    }

    /// Returns the current bloom configuration.
    pub fn bloom_config(&self) -> BloomConfig {
        self.renderer()
            .oread()
            .postprocessor()
            .oread()
            .capturing_camera()
            .bloom_config()
            .clone()
    }

    /// Returns whether bloom is enabled.
    pub fn bloom_enabled(&self) -> bool {
        self.renderer()
            .oread()
            .postprocessor()
            .oread()
            .capturing_camera()
            .bloom_config()
            .enabled
    }

    /// Returns the current average luminance computation configuration.
    pub fn average_luminance_computation_config(&self) -> AverageLuminanceComputationConfig {
        self.renderer()
            .oread()
            .postprocessor()
            .oread()
            .capturing_camera()
            .average_luminance_computation_config()
            .clone()
    }

    /// Returns the current dynamic range compression configuration.
    pub fn dynamic_range_compression_config(&self) -> DynamicRangeCompressionConfig {
        self.renderer()
            .oread()
            .postprocessor()
            .oread()
            .capturing_camera()
            .dynamic_range_compression_config()
            .clone()
    }

    /// Returns the currently visualized render attachment quantity.
    pub fn visualized_render_attachment_quantity(&self) -> Option<RenderAttachmentQuantity> {
        self.renderer()
            .oread()
            .postprocessor()
            .oread()
            .visualized_render_attachment_quantity()
    }

    /// Returns the current simulation time.
    pub fn simulation_time(&self) -> f64 {
        self.simulator().oread().current_simulation_time()
    }

    /// Returns the current FPS from metrics.
    pub fn current_fps(&self) -> f64 {
        self.metrics().oread().current_smooth_fps().into()
    }

    /// Returns whether physics simulation is enabled.
    pub fn physics_simulation_enabled(&self) -> bool {
        self.simulator().oread().enabled()
    }

    /// Returns the current simulator configuration.
    pub fn simulator_config(&self) -> SimulatorConfig {
        let simulator = self.simulator().oread();
        SimulatorConfig {
            enabled: simulator.enabled(),
            n_substeps: simulator.n_substeps(),
            initial_time_step_duration: simulator.time_step_duration(),
            match_frame_duration: simulator.matches_frame_duration(),
            max_auto_time_step_duration: simulator.max_auto_time_step_duration(),
        }
    }

    /// Returns the current constraint solver configuration.
    pub fn constraint_solver_config(&self) -> ConstraintSolverConfig {
        self.simulator()
            .oread()
            .constraint_manager()
            .oread()
            .solver()
            .config()
            .clone()
    }

    /// Returns the current simulation speed multiplier.
    pub fn simulation_speed_multiplier(&self) -> fph {
        self.simulator().oread().simulation_speed_multiplier()
    }

    /// Returns whether the simulation matches frame duration.
    pub fn simulation_matches_frame_duration(&self) -> bool {
        self.simulator().oread().matches_frame_duration()
    }

    /// Returns the current time step duration.
    pub fn time_step_duration(&self) -> fph {
        self.simulator().oread().time_step_duration()
    }

    /// Returns the current number of substeps.
    pub fn simulation_substeps(&self) -> u32 {
        self.simulator().oread().n_substeps()
    }

    /// Returns the last task execution times.
    pub fn task_execution_times(&self) -> crate::instrumentation::timing::TaskExecutionTimes {
        self.metrics().oread().last_task_execution_times.clone()
    }

    /// Returns the last render pass timing results.
    pub fn render_pass_timing_results(&self) -> Vec<(String, Duration)> {
        self.renderer()
            .oread()
            .timestamp_query_manager()
            .last_timing_results()
            .iter()
            .map(|(tag, duration)| (tag.as_ref().to_string(), *duration))
            .collect()
    }

    /// Returns whether task timings are enabled.
    pub fn task_timings_enabled(&self) -> bool {
        self.task_timer().enabled()
    }

    /// Returns whether render pass timings are enabled.
    pub fn render_pass_timings_enabled(&self) -> bool {
        self.renderer().oread().basic_config().timings_enabled
    }
}
