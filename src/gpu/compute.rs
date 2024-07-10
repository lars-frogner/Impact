//! Management of resources for GPU computation.

use crate::gpu::{
    shader::ComputeShaderInput, storage::StorageGPUBuffer, texture::Texture,
    uniform::SingleUniformGPUBuffer,
};
use impact_utils::stringhash64_newtype;
use std::collections::{hash_map::Entry, HashMap};

stringhash64_newtype!(
    /// Identifier for a specific GPU computation. Wraps a
    /// [`StringHash64`](impact_utils::StringHash64).
    [pub] GPUComputationID
);

/// A GPU computation description specifying the resources used in the
/// computation and the input to the compute shader.
#[derive(Debug)]
pub struct GPUComputationSpecification {
    resources: Option<GPUComputationResourceGroup>,
    shader_input: ComputeShaderInput,
}

/// A group of resources residing on the GPU for a specific GPU computation.
#[derive(Debug)]
pub struct GPUComputationResourceGroup {
    _single_uniform_buffers: Vec<SingleUniformGPUBuffer>,
    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
}

/// Container for GPU computation specifications.
#[derive(Debug)]
pub struct GPUComputationLibrary {
    computation_specifications: HashMap<GPUComputationID, GPUComputationSpecification>,
}

impl GPUComputationSpecification {
    /// Creates a new GPU computation specification with the given resources and
    /// shader input.
    pub fn new(
        resources: Option<GPUComputationResourceGroup>,
        shader_input: ComputeShaderInput,
    ) -> Self {
        Self {
            resources,
            shader_input,
        }
    }

    /// Returns a reference to the resources used by the GPU computation, or
    /// [`None`] if the computation uses no resources.
    pub fn resources(&self) -> Option<&GPUComputationResourceGroup> {
        self.resources.as_ref()
    }

    /// Returns the input required for generating a shader for the computation.
    pub fn shader_input(&self) -> &ComputeShaderInput {
        &self.shader_input
    }
}

impl GPUComputationResourceGroup {
    /// Gathers the given sets of uniform and storage buffers and textures into
    /// a group of resources used in a specific GPU computation.
    ///
    /// The resources will be gathered in a single bind group, and the binding
    /// for each resource will correspond to what its index would have been in
    /// the concatenated list of resources: `single_uniform_buffers +
    /// storage_buffers + textures`.
    pub fn new(
        device: &wgpu::Device,
        single_uniform_buffers: Vec<SingleUniformGPUBuffer>,
        storage_buffers: &[&StorageGPUBuffer],
        textures: &[&Texture],
        label: &str,
    ) -> Self {
        let n_entries = single_uniform_buffers.len() + storage_buffers.len() + textures.len();
        let mut bind_group_layout_entries = Vec::with_capacity(n_entries);
        let mut bind_group_entries = Vec::with_capacity(n_entries);
        let mut binding = 0;

        for buffer in &single_uniform_buffers {
            bind_group_layout_entries.push(buffer.create_bind_group_layout_entry(binding));
            bind_group_entries.push(buffer.create_bind_group_entry(binding));
            binding += 1;
        }

        for buffer in storage_buffers {
            bind_group_layout_entries
                .push(buffer.create_bind_group_layout_entry(binding, wgpu::ShaderStages::COMPUTE));
            bind_group_entries.push(buffer.create_bind_group_entry(binding));
            binding += 1;
        }

        for texture in textures {
            bind_group_layout_entries.push(
                texture
                    .create_texture_bind_group_layout_entry(binding, wgpu::ShaderStages::COMPUTE),
            );
            bind_group_entries.push(texture.create_texture_bind_group_entry(binding));
            binding += 1;
        }

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &bind_group_layout_entries,
            label: Some(&format!("{} bind group layout", label)),
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &bind_group_entries,
            label: Some(&format!("{} bind group", label)),
        });

        Self {
            _single_uniform_buffers: single_uniform_buffers,
            bind_group_layout,
            bind_group,
        }
    }

    /// Returns a reference to the bind group layout for the compute resources.
    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.bind_group_layout
    }

    /// Returns a reference to the bind group for the compute resources.
    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
}

impl GPUComputationLibrary {
    /// Creates a new library with no GPU computations.
    pub fn new() -> Self {
        Self {
            computation_specifications: HashMap::new(),
        }
    }

    /// Returns a reference to the [`HashMap`] storing all
    /// [`GPUComputationSpecification`]s.
    pub fn computation_specifications(
        &self,
    ) -> &HashMap<GPUComputationID, GPUComputationSpecification> {
        &self.computation_specifications
    }

    /// Returns the specification of the computation with the given ID, or
    /// [`None`] if the computation does not exist.
    pub fn get_computation_specification(
        &self,
        computation_id: GPUComputationID,
    ) -> Option<&GPUComputationSpecification> {
        self.computation_specifications.get(&computation_id)
    }

    /// Returns a hashmap entry for the specification of the computation with
    /// the given ID.
    pub fn computation_specification_entry(
        &mut self,
        computation_id: GPUComputationID,
    ) -> Entry<'_, GPUComputationID, GPUComputationSpecification> {
        self.computation_specifications.entry(computation_id)
    }

    /// Adds the given computation specification to the manager under the given
    /// ID. If a computation with the same ID exists, it will be overwritten.
    pub fn add_computation_specification(
        &mut self,
        computation_id: GPUComputationID,
        computation_specification: GPUComputationSpecification,
    ) {
        self.computation_specifications
            .insert(computation_id, computation_specification);
    }
}

impl Default for GPUComputationLibrary {
    fn default() -> Self {
        Self::new()
    }
}
