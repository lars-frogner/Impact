//! Physics simulation GPU commands for particles.

use crate::{
    BasicParticleSimulation, ParticleEmitterManager,
    gpu_resource::{
        GPUParticleSimulation, ParticleGPUResources, ParticleImmediate, ParticleImmediateGroup,
        ParticleResourceRegistries, ParticleSimulationGPUResources, SimulationGPUBuffer,
    },
    shader_templates::ParticleSimulationShaderTemplate,
};
use anyhow::{Result, anyhow};
use impact_gpu::{
    bind_group_layout::BindGroupLayoutRegistry, device::GraphicsDevice, immediate::ImmediateStager,
    shader::ShaderManager, timestamp_query::TimestampQueryRegistry, wgpu,
};
use impact_rendering::compute;
use std::{borrow::Cow, marker::PhantomData, ops::Range};

/// GPU commands for performing particle simulation.
#[derive(Debug)]
pub struct ParticleSimulationCommands {
    basic_simulation: SpecificSimulationCommands<BasicParticleSimulation>,
}

#[derive(Debug)]
struct SpecificSimulationCommands<S> {
    immediates: ParticleImmediateGroup,
    pipeline_layout: wgpu::PipelineLayout,
    pipeline: wgpu::ComputePipeline,
    max_simulation_count: usize,
    _phantom: PhantomData<S>,
}

impl ParticleSimulationCommands {
    /// Initializes the pipelines for particle simulation, including creating
    /// the required shaders.
    pub fn new(
        graphics_device: &GraphicsDevice,
        shader_manager: &mut ShaderManager,
        resource_registries: &impl ParticleResourceRegistries,
        bind_group_layout_registry: &BindGroupLayoutRegistry,
    ) -> Option<Self> {
        let material_registry = resource_registries.particle_material();

        if material_registry.n_particle_materials() == 0 {
            // No materials configured, so no particle simulation needed
            return None;
        }

        let basic_simulation = SpecificSimulationCommands::new(
            graphics_device,
            shader_manager,
            bind_group_layout_registry,
        );

        Some(Self { basic_simulation })
    }

    /// Recreates simulation pipelines that are out of sync with the current GPU
    /// resources.
    pub fn sync_with_gpu_resources(
        &mut self,
        graphics_device: &GraphicsDevice,
        shader_manager: &mut ShaderManager,
        gpu_resources: &impl ParticleGPUResources,
    ) -> Result<()> {
        let simulation_resources = gpu_resources.particle_simulations();

        let basic_simulation_buffer = simulation_resources
            .basic_simulation_buffer()
            .ok_or_else(|| anyhow!("Missing basic particle simulation GPU buffer"))?;

        self.basic_simulation.sync_with_simulation_gpu_buffer(
            graphics_device,
            shader_manager,
            basic_simulation_buffer,
        );

        Ok(())
    }

    /// Records the GPU commands for particle simulation.
    pub fn record(
        &self,
        gpu_resources: &impl ParticleGPUResources,
        timestamp_recorder: &mut TimestampQueryRegistry<'_>,
        command_encoder: &mut wgpu::CommandEncoder,
        current_time_step: u64,
    ) -> Result<()> {
        let simulation_resources = gpu_resources.particle_simulations();

        if !simulation_resources.has_particle_emitters() {
            return Ok(());
        }

        let basic_simulation_buffer = simulation_resources
            .basic_simulation_buffer()
            .ok_or_else(|| anyhow!("Missing basic particle simulation GPU buffer"))?;

        let (timestamp_writes, _timestamp_span_guard) = timestamp_recorder
            .register_timestamp_writes_for_single_compute_pass(Cow::Borrowed(
                "Particle simulation pass",
            ));

        let mut compute_pass = command_encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            timestamp_writes,
            label: Some("Particle simulation compute pass"),
        });

        self.basic_simulation.dispatch(
            simulation_resources,
            basic_simulation_buffer,
            &mut compute_pass,
            current_time_step,
        )?;

        Ok(())
    }
}

impl<S: GPUParticleSimulation> SpecificSimulationCommands<S> {
    fn new(
        graphics_device: &GraphicsDevice,
        shader_manager: &mut ShaderManager,
        bind_group_layout_registry: &BindGroupLayoutRegistry,
    ) -> Self {
        let bind_group_layout =
            SimulationGPUBuffer::<S>::get_or_create_simulation_and_particle_buffer_bind_group_layout(
                graphics_device,
                bind_group_layout_registry,
            );

        let immediates = S::ShaderTemplate::immediates();

        let pipeline_layout = compute::create_compute_pipeline_layout(
            graphics_device.device(),
            &[Some(&bind_group_layout)],
            immediates.total_size(),
            S::PIPELINE_LAYOUT_LABEL,
        );

        let max_simulation_count = ParticleEmitterManager::INITIAL_SIMULATION_CAPACITY;

        let (_, shader) = shader_manager.get_or_create_compute_shader_from_template(
            graphics_device,
            &S::ShaderTemplate::new(max_simulation_count),
        );

        let pipeline = compute::create_compute_pipeline(
            graphics_device.device(),
            &pipeline_layout,
            shader,
            S::PIPELINE_LABEL,
        );

        Self {
            immediates,
            pipeline_layout,
            pipeline,
            max_simulation_count,
            _phantom: PhantomData,
        }
    }

    fn sync_with_simulation_gpu_buffer(
        &mut self,
        graphics_device: &GraphicsDevice,
        shader_manager: &mut ShaderManager,
        simulation_gpu_buffer: &SimulationGPUBuffer<S>,
    ) {
        let max_simulation_count = simulation_gpu_buffer.max_simulation_count();

        if max_simulation_count != self.max_simulation_count {
            let shader_template = S::ShaderTemplate::new(max_simulation_count);
            let (_, shader) = shader_manager
                .get_or_create_compute_shader_from_template(graphics_device, &shader_template);

            self.pipeline = compute::create_compute_pipeline(
                graphics_device.device(),
                &self.pipeline_layout,
                shader,
                S::PIPELINE_LABEL,
            );

            self.max_simulation_count = max_simulation_count;
        }
    }

    fn stage_constant_immediates(
        stager: &mut ImmediateStager<'_, ParticleImmediate>,
        current_time_step: u64,
        effective_time_step_duration: f32,
    ) {
        stager.stage_immediate_data_if_present(ParticleImmediate::TimeStep, || {
            current_time_step as u32
        });

        stager.stage_immediate_data_if_present(ParticleImmediate::TimeStepDuration, || {
            effective_time_step_duration
        });
    }

    fn stage_per_emitter_immediates(
        stager: &mut ImmediateStager<'_, ParticleImmediate>,
        particle_range: Range<u32>,
        simulation_idx: usize,
    ) {
        stager.stage_immediate_data_if_present(ParticleImmediate::ParticleRange, || {
            ParticleImmediate::particle_range_value(particle_range)
        });

        stager.stage_immediate_data_if_present(ParticleImmediate::SimulationIdx, || {
            simulation_idx as u32
        });
    }

    fn dispatch(
        &self,
        simulation_resources: &ParticleSimulationGPUResources,
        simulation_gpu_buffer: &SimulationGPUBuffer<S>,
        compute_pass: &mut wgpu::ComputePass<'_>,
        current_time_step: u64,
    ) -> Result<()> {
        let emitter_ids = simulation_gpu_buffer.emitter_ids();

        if emitter_ids.is_empty() {
            return Ok(());
        }

        compute_pass.set_pipeline(&self.pipeline);

        compute_pass.set_bind_group(
            0,
            simulation_gpu_buffer.simulation_and_particle_buffer_bind_group(),
            &[],
        );

        let mut immediate_stager = self.immediates.create_data_stager();
        Self::stage_constant_immediates(
            &mut immediate_stager,
            current_time_step,
            simulation_resources.effective_time_step_duration(),
        );

        for (simulation_idx, &emitter_id) in emitter_ids.iter().enumerate() {
            let Some(emitter_metadata) =
                simulation_resources.get_emitter_metadata(emitter_id.into())
            else {
                continue;
            };
            let particle_range = emitter_metadata.particle_range_u32();
            let particle_count = particle_range.len() as u32;

            Self::stage_per_emitter_immediates(
                &mut immediate_stager,
                particle_range,
                simulation_idx,
            );
            immediate_stager.set_staged_immediate_data(compute_pass);

            let [x, y, z] = S::ShaderTemplate::determine_workgroup_counts(particle_count);
            compute_pass.dispatch_workgroups(x, y, z);
        }

        log::trace!(
            "Dispatched {} workgroups for {} emitters",
            S::LABEL,
            emitter_ids.len()
        );

        Ok(())
    }
}
