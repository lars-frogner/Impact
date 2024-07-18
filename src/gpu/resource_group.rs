//! Groups of GPU data resources.

use crate::gpu::{
    storage::StorageGPUBuffer, texture::Texture, uniform::SingleUniformGPUBuffer, GraphicsDevice,
};
use impact_utils::stringhash64_newtype;
use std::collections::{hash_map::Entry, HashMap};

stringhash64_newtype!(
    /// Identifier for a specific GPU resource group. Wraps a
    /// [`StringHash64`](impact_utils::StringHash64).
    [pub] GPUResourceGroupID
);

/// A bindable group of resources residing on the GPU.
#[derive(Debug)]
pub struct GPUResourceGroup {
    single_uniform_buffers: Vec<SingleUniformGPUBuffer>,
    n_storage_buffers: usize,
    n_unsampled_textures: usize,
    n_sampled_textures: usize,
    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
}

/// Container for GPU resource groups.
#[derive(Debug)]
pub struct GPUResourceGroupManager {
    resource_groups: HashMap<GPUResourceGroupID, GPUResourceGroup>,
}

impl GPUResourceGroup {
    /// Gathers the given sets of uniform buffers, storage buffers, unsampled
    /// textures and sampled textures into a group of GPU resources.
    ///
    /// The resources will be gathered in a single bind group, and the binding
    /// for each resource can be obtained by calling the appropriate
    /// `Self::<resource>_binding()` method with the index of the resource in
    /// the respective input slice.
    pub fn new(
        graphics_device: &GraphicsDevice,
        single_uniform_buffers: Vec<SingleUniformGPUBuffer>,
        storage_buffers: &[&StorageGPUBuffer],
        unsampled_textures: &[&Texture],
        sampled_textures: &[&Texture],
        visibility: wgpu::ShaderStages,
        label: &str,
    ) -> Self {
        let n_entries = single_uniform_buffers.len()
            + storage_buffers.len()
            + unsampled_textures.len()
            + 2 * sampled_textures.len();

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
                .push(buffer.create_bind_group_layout_entry(binding, visibility));
            bind_group_entries.push(buffer.create_bind_group_entry(binding));
            binding += 1;
        }

        for texture in unsampled_textures {
            bind_group_layout_entries
                .push(texture.create_texture_bind_group_layout_entry(binding, visibility));
            bind_group_entries.push(texture.create_texture_bind_group_entry(binding));
            binding += 1;
        }

        for texture in sampled_textures {
            bind_group_layout_entries
                .push(texture.create_texture_bind_group_layout_entry(binding, visibility));
            bind_group_entries.push(texture.create_texture_bind_group_entry(binding));
            binding += 1;

            bind_group_layout_entries
                .push(texture.create_sampler_bind_group_layout_entry(binding, visibility));
            bind_group_entries.push(texture.create_sampler_bind_group_entry(binding));
            binding += 1;
        }

        let bind_group_layout =
            graphics_device
                .device()
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    entries: &bind_group_layout_entries,
                    label: Some(&format!("{} bind group layout", label)),
                });

        let bind_group = graphics_device
            .device()
            .create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &bind_group_layout,
                entries: &bind_group_entries,
                label: Some(&format!("{} bind group", label)),
            });

        Self {
            single_uniform_buffers,
            n_storage_buffers: storage_buffers.len(),
            n_unsampled_textures: unsampled_textures.len(),
            n_sampled_textures: sampled_textures.len(),
            bind_group_layout,
            bind_group,
        }
    }

    /// Returns the binding for the single uniform buffer at the given index, or
    /// [`None`] if the index is out of bounds.
    pub fn single_uniform_buffer_binding(&self, idx: usize) -> Option<u32> {
        if idx < self.single_uniform_buffers.len() {
            Some(idx as u32)
        } else {
            None
        }
    }

    /// Returns the binding for the storage buffer at the given index, or
    /// [`None`] if the index is out of bounds.
    pub fn storage_buffer_binding(&self, idx: usize) -> Option<u32> {
        let offset = self.single_uniform_buffers.len();
        if idx < self.n_storage_buffers {
            Some((offset + idx) as u32)
        } else {
            None
        }
    }

    /// Returns the binding for the unsampled texture at the given index, or
    /// [`None`] if the index is out of bounds.
    pub fn unsampled_texture_binding(&self, idx: usize) -> Option<u32> {
        let offset = self.single_uniform_buffers.len() + self.n_storage_buffers;
        if idx < self.n_unsampled_textures {
            Some((offset + idx) as u32)
        } else {
            None
        }
    }

    /// Returns the binding for the sampled texture at the given index, or
    /// [`None`] if the index is out of bounds.
    pub fn sampled_texture_binding(&self, idx: usize) -> Option<u32> {
        let offset =
            self.single_uniform_buffers.len() + self.n_storage_buffers + self.n_unsampled_textures;
        if idx < self.n_sampled_textures {
            Some((offset + 2 * idx) as u32)
        } else {
            None
        }
    }

    /// Returns the binding for the sampler for the sampled texture at the given
    /// index, or [`None`] if the index is out of bounds.
    pub fn sampler_binding(&self, idx: usize) -> Option<u32> {
        self.sampled_texture_binding(idx).map(|binding| binding + 1)
    }

    /// Returns a reference to the bind group layout for the resources.
    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.bind_group_layout
    }

    /// Returns a reference to the bind group for the resources.
    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
}

impl GPUResourceGroupManager {
    /// Creates a new empty resource group manager.
    pub fn new() -> Self {
        Self {
            resource_groups: HashMap::new(),
        }
    }

    /// Returns a reference to the [`HashMap`] storing all
    /// [`GPUResourceGroup`]s.
    pub fn resource_groups(&self) -> &HashMap<GPUResourceGroupID, GPUResourceGroup> {
        &self.resource_groups
    }

    /// Returns the resource group with the given ID, or [`None`] if the group
    /// does not exist.
    pub fn get_resource_group(
        &self,
        resource_group_id: GPUResourceGroupID,
    ) -> Option<&GPUResourceGroup> {
        self.resource_groups.get(&resource_group_id)
    }

    /// Removes the resource group with the given ID if it exists.
    pub fn remove_resource_group(&mut self, resource_group_id: GPUResourceGroupID) {
        self.resource_groups.remove(&resource_group_id);
    }

    /// Returns a hashmap entry for the resource group with the given ID.
    pub fn resource_group_entry(
        &mut self,
        resource_group_id: GPUResourceGroupID,
    ) -> Entry<'_, GPUResourceGroupID, GPUResourceGroup> {
        self.resource_groups.entry(resource_group_id)
    }

    /// Adds the given resource group to the manager under the given ID. If a
    /// group with the same ID exists, it will be overwritten.
    pub fn add_resource_group(
        &mut self,
        resource_group_id: GPUResourceGroupID,
        resource_group: GPUResourceGroup,
    ) {
        self.resource_groups
            .insert(resource_group_id, resource_group);
    }

    /// Removes all resource groups from the manager.
    pub fn clear_resource_groups(&mut self) {
        self.resource_groups.clear();
    }
}

impl Default for GPUResourceGroupManager {
    fn default() -> Self {
        Self::new()
    }
}
