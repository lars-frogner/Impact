//! Public engine API.

use super::Engine;
use crate::{
    command::{AdminCommand, UserCommand},
    instrumentation::timing::TimedTaskID,
    lock_order::{OrderedMutex, OrderedRwLock},
    physics::SimulatorConfig,
    setup,
};
use anyhow::{Result, anyhow};
use impact_ecs::{
    archetype::ArchetypeComponents,
    component::{
        Component, ComponentArray, ComponentID, ComponentInstance, ComponentStorage, ComponentView,
        SingleInstance,
    },
    world::{PrototypeEntities, QueryableWorld},
};
use impact_gizmo::{GizmoParameters, GizmoType, GizmoVisibilities, GizmoVisibility};
use impact_id::EntityID;
use impact_material::values::UniformColorPhysicalMaterialValues;
use impact_model::{InstanceFeature, ModelInstanceID};
use impact_physics::{
    constraint::solver::ConstraintSolverConfig,
    force::alignment_torque::{AlignmentTorqueGenerator, AlignmentTorqueGeneratorID},
    rigid_body::{DynamicRigidBody, DynamicRigidBodyID},
};
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
use impact_voxel::{
    VoxelObjectID,
    interaction::absorption::{AbsorbedVoxels, VoxelAbsorbingCapsuleID, VoxelAbsorbingSphereID},
    mesh::MeshedChunkedVoxelObject,
};
use std::sync::atomic::Ordering;
use std::time::Duration;

impl Engine {
    pub fn queryable_world(&self) -> QueryableWorld<'_> {
        QueryableWorld::wrap(self.ecs_world.read())
    }

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

    pub fn stage_entity_for_update(
        &self,
        entity_id: EntityID,
        components: Vec<SingleInstance<ComponentStorage>>,
    ) {
        self.entity_stager
            .olock()
            .stage_entity_for_update(entity_id, components);
    }

    pub fn stage_entity_for_removal(&self, entity_id: EntityID) {
        self.entity_stager
            .olock()
            .stage_entity_for_removal(entity_id);
    }

    pub fn create_entity_with_id<CA, E>(
        &self,
        entity_id: EntityID,
        components: impl TryInto<SingleInstance<ArchetypeComponents<CA>>, Error = E>,
    ) -> Result<()>
    where
        CA: ComponentArray,
        E: Into<anyhow::Error>,
    {
        self.entity_id_manager
            .olock()
            .register_id_if_absent(entity_id);

        let mut entities = PrototypeEntities::new_single(entity_id, components)?;

        setup::perform_setup_for_new_entities(self, &mut entities)?;

        self.ecs_world
            .owrite()
            .create_prototype_entities(entities)?;

        Ok(())
    }

    pub fn create_entity<AC, E>(
        &self,
        components: impl TryInto<SingleInstance<ArchetypeComponents<AC>>, Error = E>,
    ) -> Result<EntityID>
    where
        AC: ComponentArray,
        E: Into<anyhow::Error>,
    {
        let entity_id = self.entity_id_manager.olock().provide_id();

        let mut entities = PrototypeEntities::new_single(entity_id, components)?;

        setup::perform_setup_for_new_entities(self, &mut entities)?;

        self.ecs_world
            .owrite()
            .create_prototype_entities(entities)?;

        Ok(entity_id)
    }

    pub fn create_entities<AC, E>(
        &self,
        components: impl TryInto<ArchetypeComponents<AC>, Error = E>,
    ) -> Result<Vec<EntityID>>
    where
        AC: ComponentArray,
        E: Into<anyhow::Error>,
    {
        let components = components.try_into().map_err(E::into)?;

        let entity_ids = self
            .entity_id_manager
            .olock()
            .provide_id_vec(components.instance_count());

        let mut entities = PrototypeEntities::new(entity_ids, components)?;

        setup::perform_setup_for_new_entities(self, &mut entities)?;

        let entity_ids = self
            .ecs_world
            .owrite()
            .create_prototype_entities(entities)?;

        Ok(entity_ids.into_vec())
    }

    pub fn update_entity<A>(
        &self,
        entity_id: EntityID,
        components: impl IntoIterator<Item = SingleInstance<A>>,
    ) -> Result<()>
    where
        A: ComponentArray,
    {
        let ecs_world = self.ecs_world.oread();

        let entity = ecs_world
            .get_entity(entity_id)
            .ok_or_else(|| anyhow!("Entity with ID {entity_id} not present"))?;

        for component in components {
            entity
                .get_component_bytes_mut(component.component_id())
                .ok_or_else(|| {
                    anyhow!(
                        "Entity with ID {entity_id} has no component with ID {}",
                        component.component_id().as_u64()
                    )
                })?
                .set(component.single_instance_view());
        }

        Ok(())
    }

    pub fn remove_entity(&self, entity_id: EntityID) -> Result<()> {
        let mut ecs_world = self.ecs_world.owrite();
        setup::perform_cleanup_for_removed_entity(self, entity_id, &ecs_world.entity(entity_id))?;
        ecs_world.remove_entity(entity_id)?;
        drop(ecs_world);
        self.entity_id_manager.olock().unregister_id(entity_id);
        Ok(())
    }

    pub fn for_entity_components<I>(
        &self,
        entity_id: EntityID,
        only_component_ids: impl IntoIterator<Item = ComponentID, IntoIter = I>,
        f: &mut impl FnMut(SingleInstance<ComponentView<'_>>),
    ) -> Result<()>
    where
        I: ExactSizeIterator<Item = ComponentID>,
    {
        let only_component_ids = only_component_ids.into_iter();

        let ecs_world = self.ecs_world.oread();

        let entity = ecs_world
            .get_entity(entity_id)
            .ok_or_else(|| anyhow!("Entity with ID {entity_id} not present"))?;

        let get_component = |component_id| {
            entity.get_component_bytes(component_id).ok_or_else(|| {
                anyhow!(
                    "Entity with ID {entity_id} has no component with ID {}",
                    component_id.as_u64()
                )
            })
        };

        if only_component_ids.len() == 0 {
            for component_id in entity.archetype().component_ids().iter().copied() {
                let component = get_component(component_id)?;
                f(component.access());
            }
        } else {
            for component_id in only_component_ids {
                let component = get_component(component_id)?;
                f(component.access());
            }
        }

        Ok(())
    }

    pub fn enqueue_user_command(&self, command: UserCommand) {
        match command {
            UserCommand::Scene(command) => {
                self.command_queues.user.scene.enqueue_command(command);
            }
            UserCommand::Control(command) => {
                self.command_queues.user.control.enqueue_command(command);
            }
            UserCommand::Physics(command) => {
                self.command_queues.user.physics.enqueue_command(command);
            }
        }
    }

    pub fn enqueue_admin_command(&self, command: AdminCommand) {
        match command {
            AdminCommand::Rendering(command) => {
                self.command_queues.admin.rendering.enqueue_command(command);
            }
            AdminCommand::Physics(command) => {
                self.command_queues.admin.physics.enqueue_command(command);
            }
            AdminCommand::Control(command) => {
                self.command_queues.admin.control.enqueue_command(command);
            }
            AdminCommand::Capture(command) => {
                self.command_queues.admin.capture.enqueue_command(command);
            }
            AdminCommand::Instrumentation(command) => {
                self.command_queues
                    .admin
                    .instrumentation
                    .enqueue_command(command);
            }
            AdminCommand::GameLoop(command) => {
                self.command_queues.admin.game_loop.enqueue_command(command);
            }
            AdminCommand::Gizmo(command) => {
                self.command_queues.admin.gizmo.enqueue_command(command);
            }
            AdminCommand::System(command) => {
                self.command_queues.admin.system.enqueue_command(command);
            }
        }
    }

    /// Resets the scene, ECS world and physics simulator to the initial empty
    /// state and sets the simulation time to zero.
    pub fn reset_world(&self) -> Result<()> {
        log::info!("Resetting world");
        self.ecs_world.owrite().remove_all_entities();
        self.scene.oread().clear();
        self.simulator.owrite().reset();

        self.command_queues.clear();

        self.renderer.owrite().synchronize_render_commands()?;
        self.sync_all_gpu_resources()?;

        Ok(())
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
    pub fn simulation_time(&self) -> f32 {
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
    pub fn simulation_speed_multiplier(&self) -> f32 {
        self.simulator().oread().simulation_speed_multiplier()
    }

    /// Returns whether the simulation matches frame duration.
    pub fn simulation_matches_frame_duration(&self) -> bool {
        self.simulator().oread().matches_frame_duration()
    }

    /// Returns the current time step duration.
    pub fn time_step_duration(&self) -> f32 {
        self.simulator().oread().time_step_duration()
    }

    /// Returns the current number of substeps.
    pub fn simulation_substeps(&self) -> u32 {
        self.simulator().oread().n_substeps()
    }

    /// Returns the last task execution times.
    pub fn collect_task_execution_times(&self, results: &mut impl Extend<(TimedTaskID, Duration)>) {
        results.extend(
            self.metrics()
                .oread()
                .last_task_execution_times
                .iter()
                .copied(),
        );
    }

    /// Returns the last render pass timing results.
    pub fn collect_render_pass_timing_results(
        &self,
        results: &mut impl Extend<(String, Duration)>,
    ) {
        results.extend(
            self.renderer()
                .oread()
                .timestamp_query_manager()
                .last_timing_results()
                .iter()
                .map(|(tag, duration)| (tag.as_ref().to_string(), *duration)),
        );
    }

    /// Returns whether task timings are enabled.
    pub fn task_timings_enabled(&self) -> bool {
        self.task_timer().enabled()
    }

    /// Returns whether render pass timings are enabled.
    pub fn render_pass_timings_enabled(&self) -> bool {
        self.renderer().oread().basic_config().timings_enabled
    }

    pub fn with_uniform_physical_material_property_values_mut<R>(
        &self,
        entity_id: EntityID,
        f: impl FnOnce(&mut UniformColorPhysicalMaterialValues) -> Result<R>,
    ) -> Result<R> {
        let scene = self.scene().oread();

        let scene_graph = scene.scene_graph().oread();

        let node = scene_graph
            .model_instance_nodes()
            .get_node(ModelInstanceID::from_entity_id(entity_id))
            .ok_or_else(|| {
                anyhow!("Tried to obtain material properties for missing model instance")
            })?;

        let feature_id = node
            .get_rendering_feature_id_of_type(UniformColorPhysicalMaterialValues::FEATURE_TYPE_ID)
            .ok_or_else(|| {
                anyhow!("Missing `UniformColorPhysicalMaterialValues` feature for model instance")
            })?;

        drop(scene_graph);

        let mut model_instance_manager = scene.model_instance_manager().owrite();

        f(model_instance_manager.feature_mut(feature_id))
    }

    pub fn with_dynamic_rigid_body<R>(
        &self,
        entity_id: EntityID,
        f: impl FnOnce(&DynamicRigidBody) -> Result<R>,
    ) -> Result<R> {
        let simulator = self.simulator().oread();
        let rigid_body_manager = simulator.rigid_body_manager().oread();

        let rigid_body_id = DynamicRigidBodyID::from_entity_id(entity_id);
        let rigid_body = rigid_body_manager
            .get_dynamic_rigid_body(rigid_body_id)
            .ok_or_else(|| anyhow!("Missing dynamic rigid body with ID {rigid_body_id:?}"))?;

        f(rigid_body)
    }

    pub fn with_alignment_torque_generator<R>(
        &self,
        entity_id: EntityID,
        f: impl FnOnce(&AlignmentTorqueGenerator) -> Result<R>,
    ) -> Result<R> {
        let simulator = self.simulator().oread();
        let force_generator_manager = simulator.force_generator_manager().oread();

        let generator_id = AlignmentTorqueGeneratorID::from_entity_id(entity_id);
        let generator = force_generator_manager
            .alignment_torques()
            .get_generator(&generator_id)
            .ok_or_else(|| {
                anyhow!("Missing alignment torque generator with ID {generator_id:?}")
            })?;

        f(generator)
    }

    pub fn add_voxel_object(
        &self,
        entity_id: EntityID,
        voxel_object: MeshedChunkedVoxelObject,
    ) -> Result<()> {
        let voxel_object_id = VoxelObjectID::from_entity_id(entity_id);
        self.scene()
            .oread()
            .voxel_manager()
            .owrite()
            .object_manager_mut()
            .add_voxel_object(voxel_object_id, voxel_object)
    }

    pub fn replace_voxel_object(
        &self,
        entity_id: EntityID,
        voxel_object: MeshedChunkedVoxelObject,
    ) {
        let voxel_object_id = VoxelObjectID::from_entity_id(entity_id);

        if let Some(existing_voxel_object) = self
            .scene()
            .oread()
            .voxel_manager()
            .owrite()
            .object_manager_mut()
            .get_voxel_object_mut(voxel_object_id)
        {
            *existing_voxel_object = voxel_object;
        }
        self.renderer
            .oread()
            .render_resource_manager()
            .owrite()
            .voxel_objects
            .remove_voxel_object_buffers(voxel_object_id);
    }

    pub fn remove_voxel_object(&self, voxel_object_id: VoxelObjectID) {
        self.scene()
            .oread()
            .voxel_manager()
            .owrite()
            .object_manager_mut()
            .remove_voxel_object(voxel_object_id);
    }

    pub fn with_absorbed_voxels_for_sphere<R>(
        &self,
        entity_id: EntityID,
        f: impl FnOnce(&[AbsorbedVoxels], &[f32]) -> Result<R>,
    ) -> Result<R> {
        let resource_manager = self.resource_manager().oread();
        let voxel_type_registry = &resource_manager.voxel_types;
        let scene = self.scene().oread();
        let voxel_manager = scene.voxel_manager().oread();
        let absorption_manager = voxel_manager.interaction_manager.absorption_manager();

        let absorber_id = VoxelAbsorbingSphereID::from_entity_id(entity_id);
        let absorbing_sphere = absorption_manager
            .get_absorbing_sphere(absorber_id)
            .ok_or_else(|| anyhow!("Missing voxel absorbing sphere with ID {absorber_id:?}"))?;

        let absorbed_voxels_by_type = absorbing_sphere.tracker.absorbed_voxels_by_type();
        let mass_densities = voxel_type_registry.mass_densities();
        f(
            &absorbed_voxels_by_type[0..mass_densities.len()],
            mass_densities,
        )
    }

    pub fn with_absorbed_voxels_for_capsule<R>(
        &self,
        entity_id: EntityID,
        f: impl FnOnce(&[AbsorbedVoxels], &[f32]) -> Result<R>,
    ) -> Result<R> {
        let resource_manager = self.resource_manager().oread();
        let voxel_type_registry = &resource_manager.voxel_types;
        let scene = self.scene().oread();
        let voxel_manager = scene.voxel_manager().oread();
        let absorption_manager = voxel_manager.interaction_manager.absorption_manager();

        let absorber_id = VoxelAbsorbingCapsuleID::from_entity_id(entity_id);
        let absorbing_capsule = absorption_manager
            .get_absorbing_capsule(absorber_id)
            .ok_or_else(|| anyhow!("Missing voxel absorbing capsule with ID {absorber_id:?}"))?;

        let absorbed_voxels_by_type = absorbing_capsule.tracker.absorbed_voxels_by_type();
        let mass_densities = voxel_type_registry.mass_densities();
        f(
            &absorbed_voxels_by_type[0..mass_densities.len()],
            mass_densities,
        )
    }

    pub fn with_component<C: Component, R>(
        &self,
        entity_id: EntityID,
        f: impl FnOnce(&C) -> Result<R>,
    ) -> Result<R> {
        let ecs_world = self.ecs_world.oread();

        let entity_entry = ecs_world
            .get_entity(entity_id)
            .ok_or_else(|| anyhow!("Missing entity with ID {:?}", entity_id))?;

        let component_entry = entity_entry.get_component().ok_or_else(|| {
            anyhow!(
                "Missing component {:?} for entity with ID {:?}",
                C::component_id(),
                entity_id
            )
        })?;

        let component: &C = component_entry.access();

        f(component)
    }

    pub fn with_component_mut<C: Component, R>(
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

    pub fn get_component_copy<C: Component>(&self, entity_id: EntityID) -> Result<C> {
        self.with_component(entity_id, |component| Ok(*component))
    }
}
