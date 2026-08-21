//! Rendering resources for particles.

use crate::{
    BasicParticleEmitterID, BasicParticleSimulation, Particle, ParticleEmitterID,
    ParticleEmitterManager,
    material::{FixedParticleMaterialProperties, ParticleMaterialID, ParticleMaterialRegistry},
    shader_templates::{
        ParticleSimulationShaderTemplate, basic_simulation::BasicParticleSimulationShaderTemplate,
    },
};
use anyhow::{Result, anyhow};
use bytemuck::{Pod, Zeroable};
use impact_alloc::{arena::ArenaPool, avec};
use impact_containers::{HashMap, RangeAllocator};
use impact_gpu::{
    assert_uniform_item_valid,
    bind_group_layout::BindGroupLayoutRegistry,
    buffer::GPUBuffer,
    device::GraphicsDevice,
    immediate::{Immediate, ImmediateGroup},
    shader::template::SpecificShaderTemplate,
    storage,
    texture::{self, ColorSpace, Sampler, TexelDescription, Texture},
    uniform::{
        self, MultiUniformGPUBuffer, UncountedMultiUniformGPUBufferWithMetadata, UniformBuffer,
        UniformBufferable, UniformMetadata, UniformTransferResult,
    },
    wgpu,
};
use impact_math::{hash::ConstStringHash64, vector::Vector2};
use impact_mesh::gpu_resource::{
    IndexBufferable, VertexBufferable, create_vertex_buffer_layout_for_vertex,
};
use impact_rendering::immediate::BasicImmediate;
use impact_texture::gpu_resource::{SamplerMap, TextureMap};
use std::{borrow::Cow, fmt, hash::Hash, marker::PhantomData, mem, ops::Range};

pub trait ParticleResourceRegistries {
    fn particle_material(&self) -> &ParticleMaterialRegistry;
}

pub trait ParticleGPUResources {
    /// Returns the GPU resources for particle materials, or [`None`] if they
    /// have not been initialized.
    fn particle_materials(&self) -> Option<&ParticleMaterialGPUResources>;

    /// Returns the GPU resources for particle simulation.
    fn particle_simulations(&self) -> &ParticleSimulationGPUResources;
}

/// GPU resources for particle materials.
#[derive(Debug)]
pub struct ParticleMaterialGPUResources {
    n_particle_materials: u32,
    quad_index_buffer: GPUBuffer,
    quad_texture_coord_buffer: GPUBuffer,
    _fixed_property_buffer: GPUBuffer,
    bind_group: wgpu::BindGroup,
}

/// GPU resources for particle simulation.
#[derive(Debug)]
pub struct ParticleSimulationGPUResources {
    emitter_metadata: HashMap<ParticleEmitterID, ParticleEmitterGPUMetadata>,
    particle_buffer: Option<ParticleGPUBuffer>,
    basic_simulation_buffer: Option<BasicSimulationGPUBuffer>,
    effective_time_step_duration: f32,
}

#[derive(Debug, Clone)]
pub struct ParticleEmitterGPUMetadata {
    pub particle_range: Range<usize>,
    pub material_id: ParticleMaterialID,
}

/// The GPU buffer containing all particles.
#[derive(Debug)]
pub struct ParticleGPUBuffer {
    range_allocator: RangeAllocator,
    buffer: GPUBuffer,
    inactive_buffer: GPUBuffer,
    render_bind_group: wgpu::BindGroup,
    /// For keeping track of when the buffer has been reallocated.
    version: u64,
}

/// GPU buffer containing all uniforms with simulation-specific data for a
/// particular simulation type.
#[derive(Debug)]
pub struct SimulationGPUBuffer<S: GPUParticleSimulation> {
    buffer: UncountedMultiUniformGPUBufferWithMetadata<S::EmitterID>,
    bind_group: wgpu::BindGroup,
    particle_buffer_version: u64,
    _phantom: PhantomData<S>,
}

pub type ParticleImmediateGroup = ImmediateGroup<ParticleImmediate>;

/// The meaning of a piece of immediate data used for particle simulation or
/// rendering.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ParticleImmediate {
    ParticleRange,
    SimulationIdx,
    TimeStep,
    TimeStepDuration,
    MaterialIdx,
    Rendering(BasicImmediate),
}

/// Represents a uniform object containing the simulation-specific data for a
/// particular simulation type.
pub trait GPUParticleSimulation: UniformBufferable + fmt::Debug {
    type EmitterID: Copy + Hash + Eq + fmt::Debug + UniformMetadata + Into<ParticleEmitterID>;
    type ShaderTemplate: SpecificShaderTemplate + ParticleSimulationShaderTemplate;

    const BIND_GROUP_LAYOUT_ID: ConstStringHash64;
    const LABEL: &'static str;
    const BIND_GROUP_LAYOUT_LABEL: &'static str;
    const BIND_GROUP_LABEL: &'static str;
    const PIPELINE_LAYOUT_LABEL: &'static str;
    const PIPELINE_LABEL: &'static str;
}

type BasicSimulationGPUBuffer = SimulationGPUBuffer<BasicParticleSimulation>;

/// The (u, v) texture coordinates at a vertex of the particle billboard quad.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct ParticleVertexTextureCoords(pub Vector2);

impl ParticleMaterialGPUResources {
    pub const fn fixed_properties_binding() -> u32 {
        0
    }

    pub const fn normal_and_alpha_atlas_binding() -> u32 {
        1
    }

    pub const fn normal_and_alpha_sampler_binding() -> u32 {
        2
    }

    const MATERIAL_RESOURCES_BIND_GROUP_LAYOUT_ID: ConstStringHash64 =
        ConstStringHash64::new("ParticleMaterialResources");

    /// Initializes the material GPU resources for all particle materials in the
    /// given registry.
    pub fn for_particle_material_registry(
        graphics_device: &GraphicsDevice,
        textures: &TextureMap,
        samplers: &SamplerMap,
        particle_material_registry: &ParticleMaterialRegistry,
        bind_group_layout_registry: &BindGroupLayoutRegistry,
    ) -> Result<Self> {
        let quad_index_buffer = impact_mesh::gpu_resource::new_full_index_gpu_buffer(
            graphics_device,
            &create_quad_indices(),
            Cow::Borrowed("Particle quad indices"),
        );

        let quad_texture_coord_buffer = impact_mesh::gpu_resource::new_full_vertex_gpu_buffer(
            graphics_device,
            &create_quad_texture_coords(),
            Cow::Borrowed("Particle quad texture coordinates"),
        );

        let fixed_property_buffer = GPUBuffer::new_full_uniform_buffer(
            graphics_device,
            particle_material_registry.fixed_material_properties(),
            Cow::Borrowed("Fixed particle material properties"),
        );

        let normal_and_alpha_atlas_id = particle_material_registry
            .normal_and_alpha_atlas_id()
            .unwrap();
        let normal_and_alpha_atlas = textures.get(normal_and_alpha_atlas_id).ok_or_else(|| {
            anyhow!("Missing normal and alpha texture atlas for particle materials")
        })?;

        let normal_and_alpha_sampler_id = normal_and_alpha_atlas.sampler_id.ok_or_else(|| {
            anyhow!("Normal and alpha texture atlas for particle materials has no sampler")
        })?;
        let normal_and_alpha_sampler =
            samplers.get(normal_and_alpha_sampler_id).ok_or_else(|| {
                anyhow!("Missing sampler for particle material normal and alpha texture atlas")
            })?;

        let bind_group_layout =
            Self::get_or_create_bind_group_layout(graphics_device, bind_group_layout_registry);

        let bind_group = Self::create_bind_group(
            graphics_device.device(),
            &bind_group_layout,
            &fixed_property_buffer,
            &normal_and_alpha_atlas.texture,
            normal_and_alpha_sampler,
        );

        Ok(Self {
            n_particle_materials: particle_material_registry.n_particle_materials(),
            quad_index_buffer,
            quad_texture_coord_buffer,
            _fixed_property_buffer: fixed_property_buffer,
            bind_group,
        })
    }

    /// Returns the format of the indices for the particle billboard quad.
    pub fn quad_index_format() -> wgpu::IndexFormat {
        u16::INDEX_FORMAT
    }

    /// Returns the layout of the texture coordinate buffer for the particle
    /// billboard quad.
    pub fn quad_texture_coord_buffer_layout() -> wgpu::VertexBufferLayout<'static> {
        ParticleVertexTextureCoords::BUFFER_LAYOUT
    }

    /// Returns the number of registered particle materials.
    pub fn n_particle_materials(&self) -> u32 {
        self.n_particle_materials
    }

    /// Returns a reference to the index buffer for the particle billboard quad.
    pub fn quad_index_buffer(&self) -> &GPUBuffer {
        &self.quad_index_buffer
    }

    /// Returns a reference to the texture coordinate buffer for the particle
    /// billboard quad.
    pub fn quad_texture_coord_buffer(&self) -> &GPUBuffer {
        &self.quad_texture_coord_buffer
    }

    /// Returns the layout of the bind group for the particle material
    /// resources, after creating and caching it if it has not already been
    /// created.
    pub fn get_or_create_bind_group_layout(
        graphics_device: &GraphicsDevice,
        bind_group_layout_registry: &BindGroupLayoutRegistry,
    ) -> wgpu::BindGroupLayout {
        bind_group_layout_registry
            .get_or_create_layout(Self::MATERIAL_RESOURCES_BIND_GROUP_LAYOUT_ID.hash(), || {
                Self::create_bind_group_layout(graphics_device.device())
            })
    }

    /// Returns the bind group for the particle material resources.
    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }

    fn create_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                FixedParticleMaterialProperties::create_bind_group_layout_entry(
                    Self::fixed_properties_binding(),
                    wgpu::ShaderStages::VERTEX,
                ),
                texture::create_texture_bind_group_layout_entry(
                    Self::normal_and_alpha_atlas_binding(),
                    wgpu::ShaderStages::FRAGMENT,
                    TexelDescription::Rgba8(ColorSpace::Linear).texture_format(),
                    wgpu::TextureViewDimension::D2,
                ),
                texture::create_sampler_bind_group_layout_entry(
                    Self::normal_and_alpha_sampler_binding(),
                    wgpu::ShaderStages::FRAGMENT,
                    wgpu::SamplerBindingType::Filtering,
                ),
            ],
            label: Some("Particle material bind group layout"),
        })
    }

    fn create_bind_group(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        fixed_property_buffer: &GPUBuffer,
        normal_and_alpha_atlas: &Texture,
        sampler: &Sampler,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout,
            entries: &[
                fixed_property_buffer.create_bind_group_entry(Self::fixed_properties_binding()),
                normal_and_alpha_atlas
                    .create_bind_group_entry(Self::normal_and_alpha_atlas_binding()),
                sampler.create_bind_group_entry(Self::normal_and_alpha_sampler_binding()),
            ],
            label: Some("Particle material bind group"),
        })
    }
}

impl ParticleSimulationGPUResources {
    pub fn new() -> Self {
        Self {
            emitter_metadata: HashMap::default(),
            particle_buffer: None,
            basic_simulation_buffer: None,
            effective_time_step_duration: 0.0,
        }
    }

    /// The current total number of particle emitters.
    pub fn n_particle_emitters(&self) -> usize {
        self.emitter_metadata.len()
    }

    /// Whether there are particle emitters.
    pub fn has_particle_emitters(&self) -> bool {
        !self.emitter_metadata.is_empty()
    }

    /// Returns the metadata for the given particle emitter if it is present.
    pub fn get_emitter_metadata(
        &self,
        id: ParticleEmitterID,
    ) -> Option<&ParticleEmitterGPUMetadata> {
        self.emitter_metadata.get(&id)
    }

    /// Returns an iterator over the metadata of each particle emitter.
    pub fn all_emitter_metadata(&self) -> impl Iterator<Item = &ParticleEmitterGPUMetadata> {
        self.emitter_metadata.values()
    }

    /// Returns the current time step duration (zero if simulations are
    /// disabled).
    pub fn effective_time_step_duration(&self) -> f32 {
        self.effective_time_step_duration
    }

    /// Returns a reference to the particle GPU buffer if it has been created.
    pub fn particle_buffer(&self) -> Option<&ParticleGPUBuffer> {
        self.particle_buffer.as_ref()
    }

    /// Returns a reference to the GPU buffer for basic particle simulation
    /// uniforms if it has been created.
    pub fn basic_simulation_buffer(&self) -> Option<&BasicSimulationGPUBuffer> {
        self.basic_simulation_buffer.as_ref()
    }

    /// Returns the layout of the bind group for the particle buffer, after
    /// creating and caching it if it has not already been created.
    pub fn get_or_create_particle_buffer_bind_group_layout(
        graphics_device: &GraphicsDevice,
        bind_group_layout_registry: &BindGroupLayoutRegistry,
    ) -> wgpu::BindGroupLayout {
        ParticleGPUBuffer::get_or_create_render_bind_group_layout(
            graphics_device,
            bind_group_layout_registry,
        )
    }

    /// Performs any required updates for keeping the particle simulation GPU
    /// resources in sync with the particle emitter data.
    ///
    /// GPU resources whose source data no longer exists will be removed, and
    /// missing GPU resources for new source data will be created.
    pub fn sync_buffers_with_manager(
        &mut self,
        graphics_device: &GraphicsDevice,
        staging_belt: &mut wgpu::util::StagingBelt,
        command_encoder: &mut wgpu::CommandEncoder,
        bind_group_layout_registry: &BindGroupLayoutRegistry,
        effective_time_step_duration: f32,
        emitter_manager: &ParticleEmitterManager,
    ) {
        // TODO: Make sure particles are cleared on world reset

        // Cache the current time step duration so it can be accessed when
        // recording render commands
        self.effective_time_step_duration = effective_time_step_duration;

        let particle_buffer = self.particle_buffer.get_or_insert_with(|| {
            // TODO: Account for current particle count when creating
            ParticleGPUBuffer::new(graphics_device, bind_group_layout_registry)
        });

        let mut particle_ranges_were_freed = false;

        // Remove cached metadata and free the particle buffer space for all
        // emitters that have been removed from the manager
        self.emitter_metadata.retain(|emitter_id, metadata| {
            let exists = emitter_manager.has_emitter(emitter_id);
            if !exists {
                particle_buffer.free_particle_range(&metadata.particle_range);
                particle_ranges_were_freed = true;
            }
            exists
        });

        if particle_ranges_were_freed {
            particle_buffer.maintain_range_allocator_after_frees();
        }

        // Add metadata for new emitters and update existing metadata, making
        // sure the emitters get their required space in the particle buffer
        for (emitter_id, metadata) in emitter_manager.emitter_metadata() {
            self.emitter_metadata
                .entry(*emitter_id)
                .and_modify(|old_metadata| {
                    if old_metadata.particle_range.len() != metadata.particle_count() {
                        old_metadata.particle_range = particle_buffer
                            .reallocate_range_for_particles(
                                graphics_device,
                                command_encoder,
                                bind_group_layout_registry,
                                &old_metadata.particle_range,
                                metadata.log2_particle_count,
                            );
                    }
                    old_metadata.material_id = metadata.material_id;
                })
                .or_insert_with(|| {
                    let particle_range = particle_buffer.allocate_range_for_particles(
                        graphics_device,
                        command_encoder,
                        bind_group_layout_registry,
                        metadata.log2_particle_count,
                    );
                    ParticleEmitterGPUMetadata {
                        particle_range,
                        material_id: metadata.material_id,
                    }
                });
        }

        // Create or synchronize simulation GPU buffers
        if let Some(gpu_buffer) = &mut self.basic_simulation_buffer {
            gpu_buffer.sync_gpu_buffer(
                graphics_device,
                staging_belt,
                command_encoder,
                bind_group_layout_registry,
                emitter_manager.basic_simulation_buffer(),
                particle_buffer,
            );
        } else {
            self.basic_simulation_buffer = Some(BasicSimulationGPUBuffer::new(
                graphics_device,
                bind_group_layout_registry,
                emitter_manager.basic_simulation_buffer(),
                particle_buffer,
            ));
        }
    }
}

impl Default for ParticleSimulationGPUResources {
    fn default() -> Self {
        Self::new()
    }
}

impl ParticleEmitterGPUMetadata {
    pub fn particle_range_u32(&self) -> Range<u32> {
        self.particle_range.start as u32..self.particle_range.end as u32
    }
}

impl ParticleGPUBuffer {
    const RENDER_BIND_GROUP_LAYOUT_ID: ConstStringHash64 = ConstStringHash64::new("ParticleBuffer");

    const LOG2_INITIAL_PARTICLE_CAPACITY: usize = 2;
    const INITIAL_PARTICLE_CAPACITY: usize = 1 << Self::LOG2_INITIAL_PARTICLE_CAPACITY;

    fn new(
        graphics_device: &GraphicsDevice,
        bind_group_layout_registry: &BindGroupLayoutRegistry,
    ) -> Self {
        let capacity = Self::INITIAL_PARTICLE_CAPACITY;

        let range_allocator = RangeAllocator::with_free_range(&(0..capacity));

        let render_bind_group_layout = Self::get_or_create_render_bind_group_layout(
            graphics_device,
            bind_group_layout_registry,
        );

        let buffer = Self::create_buffer(graphics_device, capacity);
        let inactive_buffer = Self::create_buffer(graphics_device, 0);

        let render_bind_group = Self::create_render_bind_group(
            graphics_device.device(),
            &buffer,
            &render_bind_group_layout,
        );

        Self {
            range_allocator,
            buffer,
            inactive_buffer,
            render_bind_group,
            version: 0,
        }
    }

    /// Returns the bind group containing only the particle buffer, for use
    /// during the render pass.
    pub fn render_bind_group(&self) -> &wgpu::BindGroup {
        &self.render_bind_group
    }

    fn buffer(&self) -> &GPUBuffer {
        &self.buffer
    }

    fn version(&self) -> u64 {
        self.version
    }

    fn current_capacity(&self) -> usize {
        self.buffer.buffer_size() / mem::size_of::<Particle>()
    }

    fn free_particle_range(&mut self, range: &Range<usize>) {
        assert!(range.end <= self.current_capacity());
        self.range_allocator.free_range(range);
    }

    fn allocate_range_for_particles(
        &mut self,
        graphics_device: &GraphicsDevice,
        command_encoder: &mut wgpu::CommandEncoder,
        bind_group_layout_registry: &BindGroupLayoutRegistry,
        log2_particle_count: u32,
    ) -> Range<usize> {
        let added_particle_count = 1 << (log2_particle_count as usize);

        if let Some(range) = self.range_allocator.allocate_range(added_particle_count) {
            // There was a free range of the required size in the existing
            // buffer
            return range;
        }

        // The existing buffer can't fit the allocation, so we allocate a larger
        // buffer and copy over the old data before allocating the requested
        // particle range

        let old_capacity = self.current_capacity();

        let required_capacity = old_capacity
            .checked_add(added_particle_count)
            .unwrap()
            .next_power_of_two();

        let new_capacity = old_capacity.checked_mul(2).unwrap().max(required_capacity);

        let new_buffer = Self::create_buffer(graphics_device, new_capacity);

        // Set the new buffer as current and the current as inactive
        self.inactive_buffer = new_buffer;
        mem::swap(&mut self.inactive_buffer, &mut self.buffer);

        let bind_group_layout = Self::get_or_create_render_bind_group_layout(
            graphics_device,
            bind_group_layout_registry,
        );

        let new_bind_group = Self::create_render_bind_group(
            graphics_device.device(),
            &self.buffer,
            &bind_group_layout,
        );

        self.render_bind_group = new_bind_group;

        // Copy the full contents of the old buffer into the beginning of the
        // new buffer
        command_encoder.copy_buffer_to_buffer(
            self.inactive_buffer.buffer(),
            0,
            self.buffer.buffer(),
            0,
            self.inactive_buffer.buffer_size() as u64,
        );

        // Increment the version to signal that we are now using a new buffer
        self.version += 1;

        // Mark the new excess space as free
        self.range_allocator
            .free_range(&(old_capacity..new_capacity));

        // Excess free space might be mergeable with free space at end of old
        // buffer
        self.range_allocator.merge_consecutive_ranges();

        // Finally we can allocate the requested range (and know it will
        // succeed)
        self.range_allocator
            .allocate_range(added_particle_count)
            .unwrap()
    }

    fn reallocate_range_for_particles(
        &mut self,
        graphics_device: &GraphicsDevice,
        command_encoder: &mut wgpu::CommandEncoder,
        bind_group_layout_registry: &BindGroupLayoutRegistry,
        old_range: &Range<usize>,
        log2_particle_count: u32,
    ) -> Range<usize> {
        self.free_particle_range(old_range);
        self.allocate_range_for_particles(
            graphics_device,
            command_encoder,
            bind_group_layout_registry,
            log2_particle_count,
        )
    }

    fn maintain_range_allocator_after_frees(&mut self) {
        self.range_allocator.merge_consecutive_ranges();
    }

    fn create_buffer(graphics_device: &GraphicsDevice, capacity: usize) -> GPUBuffer {
        let buffer_size = mem::size_of::<Particle>().checked_mul(capacity).unwrap();

        // It's important to initialize the particle data with zeros so the
        // simulation shader can see the zero lifetime and spawn the particle
        let arena = ArenaPool::get_arena();
        let bytes = avec![in &arena; 0; buffer_size];

        GPUBuffer::new_storage_buffer_with_bytes(
            graphics_device,
            &bytes,
            Cow::Borrowed("Particle buffer"),
        )
    }

    fn get_or_create_render_bind_group_layout(
        graphics_device: &GraphicsDevice,
        bind_group_layout_registry: &BindGroupLayoutRegistry,
    ) -> wgpu::BindGroupLayout {
        bind_group_layout_registry
            .get_or_create_layout(Self::RENDER_BIND_GROUP_LAYOUT_ID.hash(), || {
                Self::create_render_bind_group_layout(graphics_device.device())
            })
    }

    fn create_render_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        let layout = Self::create_render_bind_group_layout_entry(0);
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[layout],
            label: Some("Particle buffer render bind group layout"),
        })
    }

    fn create_render_bind_group_layout_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
        // For rendering, we only read from the particle buffer in the vertex
        // stage
        storage::create_storage_buffer_bind_group_layout_entry(
            binding,
            wgpu::ShaderStages::VERTEX,
            true,
        )
    }

    fn create_simulation_bind_group_layout_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
        // For simulation, we read from and write to the particle buffer in a
        // compute stage
        storage::create_storage_buffer_bind_group_layout_entry(
            binding,
            wgpu::ShaderStages::COMPUTE,
            false,
        )
    }

    fn create_render_bind_group(
        device: &wgpu::Device,
        particle_buffer: &GPUBuffer,
        layout: &wgpu::BindGroupLayout,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout,
            entries: &[particle_buffer.create_bind_group_entry(0)],
            label: Some("Particle buffer render bind group"),
        })
    }
}

impl<S: GPUParticleSimulation> SimulationGPUBuffer<S> {
    fn new(
        graphics_device: &GraphicsDevice,
        bind_group_layout_registry: &BindGroupLayoutRegistry,
        cpu_buffer: &UniformBuffer<
            <S::EmitterID as UniformMetadata>::ID,
            <S::EmitterID as UniformMetadata>::Source,
        >,
        particle_buffer: &ParticleGPUBuffer,
    ) -> Self {
        let buffer = UncountedMultiUniformGPUBufferWithMetadata::for_uniform_buffer(
            graphics_device,
            cpu_buffer,
        );

        let bind_group_layout =
            Self::get_or_create_simulation_and_particle_buffer_bind_group_layout(
                graphics_device,
                bind_group_layout_registry,
            );

        let bind_group = Self::create_simulation_and_particle_buffer_bind_group(
            graphics_device.device(),
            buffer.buffer().gpu_buffer(),
            particle_buffer.buffer(),
            &bind_group_layout,
        );

        Self {
            buffer,
            bind_group,
            particle_buffer_version: particle_buffer.version(),
            _phantom: PhantomData,
        }
    }

    /// Returns the total number of simulation uniforms that can fit in the
    /// buffer.
    pub fn max_simulation_count(&self) -> usize {
        self.buffer.buffer().max_uniform_count()
    }

    /// Returns a slice with the ID of each emitter owning the simulation at the
    /// corresponding index in the simulation buffer.
    pub fn emitter_ids(&self) -> &[S::EmitterID] {
        self.buffer.metadata()
    }

    /// Returns the layout of the bind group for the simulation buffer and the
    /// particle buffer, after creating and caching it if it has not already
    /// been created.
    pub fn get_or_create_simulation_and_particle_buffer_bind_group_layout(
        graphics_device: &GraphicsDevice,
        bind_group_layout_registry: &BindGroupLayoutRegistry,
    ) -> wgpu::BindGroupLayout {
        bind_group_layout_registry.get_or_create_layout(S::BIND_GROUP_LAYOUT_ID.hash(), || {
            Self::create_simulation_and_particle_buffer_bind_group_layout(graphics_device.device())
        })
    }

    /// Returns a reference to the bind group for the simulation buffer and the
    /// particle buffer.
    pub fn simulation_and_particle_buffer_bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }

    fn sync_gpu_buffer(
        &mut self,
        graphics_device: &GraphicsDevice,
        staging_belt: &mut wgpu::util::StagingBelt,
        command_encoder: &mut wgpu::CommandEncoder,
        bind_group_layout_registry: &BindGroupLayoutRegistry,
        cpu_buffer: &UniformBuffer<
            <S::EmitterID as UniformMetadata>::ID,
            <S::EmitterID as UniformMetadata>::Source,
        >,
        particle_buffer: &ParticleGPUBuffer,
    ) {
        let transfer_result = self.buffer.transfer_uniforms_to_gpu_buffer(
            graphics_device,
            staging_belt,
            command_encoder,
            cpu_buffer,
        );

        // If the simulation GPU buffer had to be reallocated or the particle
        // buffer version changed (meaning the particle buffer was reallocated),
        // we need to recreate the bind group
        if transfer_result == UniformTransferResult::CreatedNewBuffer
            || self.particle_buffer_version != particle_buffer.version()
        {
            let bind_group_layout =
                Self::get_or_create_simulation_and_particle_buffer_bind_group_layout(
                    graphics_device,
                    bind_group_layout_registry,
                );

            self.bind_group = Self::create_simulation_and_particle_buffer_bind_group(
                graphics_device.device(),
                self.buffer.buffer().gpu_buffer(),
                particle_buffer.buffer(),
                &bind_group_layout,
            );

            self.particle_buffer_version = particle_buffer.version();
        }
    }

    fn create_simulation_and_particle_buffer_bind_group_layout(
        device: &wgpu::Device,
    ) -> wgpu::BindGroupLayout {
        let simulation_buffer_layout =
            S::create_bind_group_layout_entry(0, wgpu::ShaderStages::COMPUTE);

        let particle_buffer_layout =
            ParticleGPUBuffer::create_simulation_bind_group_layout_entry(1);

        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[simulation_buffer_layout, particle_buffer_layout],
            label: Some(S::BIND_GROUP_LAYOUT_LABEL),
        })
    }

    fn create_simulation_and_particle_buffer_bind_group(
        device: &wgpu::Device,
        simulation_buffer: &GPUBuffer,
        particle_buffer: &GPUBuffer,
        layout: &wgpu::BindGroupLayout,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout,
            entries: &[
                simulation_buffer.create_bind_group_entry(0),
                particle_buffer.create_bind_group_entry(1),
            ],
            label: Some(S::BIND_GROUP_LABEL),
        })
    }
}

impl ParticleImmediate {
    pub fn particle_range_value(range: Range<u32>) -> [u32; 2] {
        [range.start, range.end - range.start]
    }
}

impl Immediate for ParticleImmediate {
    #[allow(clippy::match_same_arms)]
    fn size_and_alignment(&self) -> (u32, u32) {
        match self {
            Self::ParticleRange => (8, 4), // u32 + u32
            Self::SimulationIdx | Self::TimeStep | Self::MaterialIdx => (4, 4), // u32
            Self::TimeStepDuration => (4, 4), // f32
            Self::Rendering(immediate) => immediate.size_and_alignment(),
        }
    }
}

impl UniformBufferable for FixedParticleMaterialProperties {
    const ID: ConstStringHash64 = ConstStringHash64::new("Fixed particle material properties");

    fn create_bind_group_layout_entry(
        binding: u32,
        visibility: wgpu::ShaderStages,
    ) -> wgpu::BindGroupLayoutEntry {
        uniform::create_uniform_buffer_bind_group_layout_entry(binding, visibility)
    }
}
assert_uniform_item_valid!(FixedParticleMaterialProperties);

impl VertexBufferable for ParticleVertexTextureCoords {
    const BUFFER_LAYOUT: wgpu::VertexBufferLayout<'static> =
        create_vertex_buffer_layout_for_vertex::<Self>(&wgpu::vertex_attr_array![
            0 => Float32x2,
        ]);
}

impl UniformMetadata for BasicParticleEmitterID {
    type ID = Self;
    type Source = BasicParticleSimulation;

    fn from_source(_source: &Self::Source, id: Self::ID) -> Self {
        id
    }
}

impl GPUParticleSimulation for BasicParticleSimulation {
    type EmitterID = BasicParticleEmitterID;
    type ShaderTemplate = BasicParticleSimulationShaderTemplate;

    const BIND_GROUP_LAYOUT_ID: ConstStringHash64 = ConstStringHash64::new("BasicSimulationBuffer");
    const LABEL: &'static str = "basic particle simulation";
    const BIND_GROUP_LAYOUT_LABEL: &'static str = "Basic particle simulation bind group layout";
    const BIND_GROUP_LABEL: &'static str = "Basic particle simulation bind group";
    const PIPELINE_LAYOUT_LABEL: &'static str = "Basic particle simulation compute pipeline layout";
    const PIPELINE_LABEL: &'static str = "Basic particle simulation compute pipeline";
}

impl UniformBufferable for BasicParticleSimulation {
    const ID: ConstStringHash64 = ConstStringHash64::new("Basic particle simulation");

    fn create_bind_group_layout_entry(
        binding: u32,
        visibility: wgpu::ShaderStages,
    ) -> wgpu::BindGroupLayoutEntry {
        uniform::create_uniform_buffer_bind_group_layout_entry(binding, visibility)
    }
}
assert_uniform_item_valid!(BasicParticleSimulation);

fn create_quad_indices() -> [u16; 6] {
    [0, 1, 2, 0, 2, 3]
}

fn create_quad_texture_coords() -> [ParticleVertexTextureCoords; 4] {
    [
        // This order gives a front-facing quad when converting to vertex
        // positions
        Vector2::new(0.0, 1.0),
        Vector2::new(1.0, 1.0),
        Vector2::new(1.0, 0.0),
        Vector2::new(0.0, 0.0),
    ]
    .map(ParticleVertexTextureCoords)
}
