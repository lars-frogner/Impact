//! Rendering resources for voxel objects.

use crate::{
    assert_uniform_valid,
    assets::Assets,
    gpu::{
        buffer::{GPUBuffer, GPUBufferType},
        indirect::{DrawIndexedIndirectArgs, DrawIndirectArgs},
        storage,
        texture::{
            self, ColorSpace, Sampler, SamplerConfig, TexelDescription, Texture,
            TextureAddressingConfig, TextureConfig, TextureFilteringConfig,
        },
        uniform::{self, UniformBufferable},
        GraphicsDevice,
    },
    mesh::buffer::{
        create_vertex_buffer_layout_for_vertex, MeshVertexAttributeLocation, VertexBufferable,
    },
    voxel::{
        chunks::ChunkedVoxelObject,
        mesh::{
            ChunkedVoxelObjectMesh, VoxelMeshIndex, VoxelMeshIndexMaterials,
            VoxelMeshVertexNormalVector, VoxelMeshVertexPosition,
        },
        voxel_types::{FixedVoxelMaterialProperties, VoxelTypeRegistry},
        VoxelObjectID,
    },
};
use anyhow::Result;
use impact_utils::ConstStringHash64;
use std::{borrow::Cow, sync::OnceLock};

/// Owner and manager of the GPU resources for all voxel materials.
#[derive(Debug)]
pub struct VoxelMaterialGPUResourceManager {
    n_voxel_types: usize,
    _fixed_property_buffer: GPUBuffer,
    bind_group: wgpu::BindGroup,
}

/// Owner and manager of GPU buffers for a [`ChunkedVoxelObject`].
#[derive(Debug)]
pub struct VoxelObjectGPUBufferManager {
    chunk_extent: f64,
    position_buffer: GPUBuffer,
    normal_vector_buffer: GPUBuffer,
    index_material_buffer: GPUBuffer,
    index_buffer: GPUBuffer,
    n_indices: usize,
    chunk_submesh_buffer: GPUBuffer,
    n_chunks: usize,
    indirect_argument_buffer: GPUBuffer,
    indexed_indirect_argument_buffer: GPUBuffer,
    position_and_normal_buffer_bind_group: wgpu::BindGroup,
    chunk_submesh_and_argument_buffer_bind_group: wgpu::BindGroup,
    chunk_submesh_and_indexed_argument_buffer_bind_group: wgpu::BindGroup,
}

/// Binding location of a specific type of voxel mesh vertex attribute (matches
/// the correponding locations in [`MeshVertexAttributeLocation`] when
/// applicable).
#[repr(u32)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum VoxelMeshVertexAttributeLocation {
    Position = MeshVertexAttributeLocation::Position as u32,
    NormalVector = MeshVertexAttributeLocation::NormalVector as u32,
    Indices = MeshVertexAttributeLocation::NormalVector as u32 + 1,
    MaterialWeights = MeshVertexAttributeLocation::NormalVector as u32 + 2,
    MaterialIndices = MeshVertexAttributeLocation::NormalVector as u32 + 3,
}

static MATERIAL_RESOURCES_BIND_GROUP_LAYOUT: OnceLock<wgpu::BindGroupLayout> = OnceLock::new();

static POSITION_AND_NORMAL_BUFFER_BIND_GROUP_LAYOUT: OnceLock<wgpu::BindGroupLayout> =
    OnceLock::new();

static CHUNK_SUBMESH_AND_ARGUMENT_BUFFER_BIND_GROUP_LAYOUT: OnceLock<wgpu::BindGroupLayout> =
    OnceLock::new();

impl VoxelMaterialGPUResourceManager {
    pub const fn fixed_properties_binding() -> u32 {
        0
    }

    pub const fn color_texture_array_binding() -> u32 {
        1
    }

    pub const fn sampler_binding() -> u32 {
        2
    }

    /// Initializes the material GPU resources for all voxel types in the given
    /// registry.
    pub fn for_voxel_type_registry(
        graphics_device: &GraphicsDevice,
        assets: &mut Assets,
        voxel_type_registry: &VoxelTypeRegistry,
    ) -> Result<Self> {
        let fixed_property_buffer = GPUBuffer::new_full_uniform_buffer(
            graphics_device,
            voxel_type_registry.fixed_material_properties(),
            Cow::Borrowed("Fixed voxel material properties"),
        );

        let color_texture_array_id = assets.load_texture_array_from_paths(
            voxel_type_registry.color_texture_paths(),
            TextureConfig {
                color_space: ColorSpace::Srgb,
                max_mip_level_count: None,
            },
            Some(SamplerConfig {
                addressing: TextureAddressingConfig::REPEATING,
                filtering: TextureFilteringConfig::BASIC,
            }),
        )?;

        let color_texture_array = &assets.textures[&color_texture_array_id];
        let sampler = &assets.samplers[&color_texture_array.sampler_id().unwrap()];

        let bind_group_layout = Self::get_or_create_bind_group_layout(graphics_device);
        let bind_group = Self::create_bind_group(
            graphics_device.device(),
            bind_group_layout,
            &fixed_property_buffer,
            color_texture_array,
            sampler,
        );

        Ok(Self {
            n_voxel_types: voxel_type_registry.n_voxel_types(),
            _fixed_property_buffer: fixed_property_buffer,
            bind_group,
        })
    }

    /// Returns the number of registered voxel types.
    pub fn n_voxel_types(&self) -> usize {
        self.n_voxel_types
    }

    /// Returns the layout of the bind group for the voxel material resources,
    /// after creating and caching it if it has not already been created.
    pub fn get_or_create_bind_group_layout(
        graphics_device: &GraphicsDevice,
    ) -> &wgpu::BindGroupLayout {
        MATERIAL_RESOURCES_BIND_GROUP_LAYOUT
            .get_or_init(|| Self::create_bind_group_layout(graphics_device.device()))
    }

    /// Returns the bind group for the voxel material resources.
    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }

    fn create_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                FixedVoxelMaterialProperties::create_bind_group_layout_entry(
                    Self::fixed_properties_binding(),
                    wgpu::ShaderStages::FRAGMENT,
                ),
                texture::create_texture_bind_group_layout_entry(
                    Self::color_texture_array_binding(),
                    wgpu::ShaderStages::FRAGMENT,
                    TexelDescription::Rgba8(ColorSpace::Srgb).texture_format(),
                    wgpu::TextureViewDimension::D2Array,
                ),
                texture::create_sampler_bind_group_layout_entry(
                    Self::sampler_binding(),
                    wgpu::ShaderStages::FRAGMENT,
                    wgpu::SamplerBindingType::Filtering,
                ),
            ],
            label: Some("Voxel material bind group layout"),
        })
    }

    fn create_bind_group(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        fixed_property_buffer: &GPUBuffer,
        color_texture_array: &Texture,
        sampler: &Sampler,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout,
            entries: &[
                fixed_property_buffer.create_bind_group_entry(Self::fixed_properties_binding()),
                color_texture_array.create_bind_group_entry(Self::color_texture_array_binding()),
                sampler.create_bind_group_entry(Self::sampler_binding()),
            ],
            label: Some("Voxel material bind group"),
        })
    }
}

impl VoxelObjectGPUBufferManager {
    /// Creates a new manager of GPU resources for the given
    /// [`ChunkedVoxelObject`]. This involves creating the
    /// [`ChunkedVoxelObjectMesh`] for the object and initializing GPU buffers
    /// for the associated data.
    pub fn for_voxel_object(
        graphics_device: &GraphicsDevice,
        voxel_object_id: VoxelObjectID,
        voxel_object: &ChunkedVoxelObject,
    ) -> Self {
        let mesh = ChunkedVoxelObjectMesh::create(voxel_object);

        // The position buffer is bound as a vertex buffer for indexed drawing (when
        // updating shadow maps) and as a storage buffer for the geometry pass
        let position_buffer = GPUBuffer::new_full_with_additional_usages(
            graphics_device,
            GPUBufferType::Vertex,
            bytemuck::cast_slice(mesh.positions()),
            wgpu::BufferUsages::STORAGE,
            Cow::Owned(format!("{} position", voxel_object_id)),
        );

        // The normal vector is bound as a storage buffer for the geometry pass
        let normal_vector_buffer = GPUBuffer::new_storage_buffer(
            graphics_device,
            mesh.normal_vectors(),
            Cow::Owned(format!("{} normal vector", voxel_object_id)),
        );

        // The the index material buffer is bound as a vertex buffer for the
        // geometry pass
        let index_material_buffer = GPUBuffer::new_full_vertex_buffer(
            graphics_device,
            mesh.index_materials(),
            Cow::Owned(format!("{} material", voxel_object_id)),
        );

        // The index buffer is bound as an ordinary index buffer for indexed drawing
        // (when updating shadow maps) and as a vertex buffer for the geometry
        // pass
        let index_buffer = GPUBuffer::new_full_with_additional_usages(
            graphics_device,
            GPUBufferType::Index,
            bytemuck::cast_slice(mesh.indices()),
            wgpu::BufferUsages::VERTEX,
            Cow::Owned(format!("{}", voxel_object_id)),
        );

        let chunk_submesh_buffer = GPUBuffer::new_storage_buffer(
            graphics_device,
            mesh.chunk_submeshes(),
            Cow::Owned(format!("{} chunk info", voxel_object_id)),
        );

        // Used for non-indexed indirect draw calls
        let indirect_argument_buffer = GPUBuffer::new_draw_indirect_buffer(
            graphics_device,
            &vec![DrawIndirectArgs::default(); mesh.n_chunks()],
            Cow::Owned(format!("{} draw argument", voxel_object_id)),
        );

        // Used for indexed indirect draw calls
        let indexed_indirect_argument_buffer = GPUBuffer::new_draw_indexed_indirect_buffer(
            graphics_device,
            &vec![DrawIndexedIndirectArgs::default(); mesh.n_chunks()],
            Cow::Owned(format!("{} indexed draw argument", voxel_object_id)),
        );

        let position_and_normal_buffer_bind_group_layout =
            Self::get_or_create_position_and_normal_buffer_bind_group_layout(graphics_device);

        let chunk_submesh_and_argument_buffer_bind_group_layout =
            Self::get_or_create_submesh_and_argument_buffer_bind_group_layout(graphics_device);

        let position_and_normal_buffer_bind_group =
            Self::create_position_and_normal_buffer_bind_group(
                graphics_device.device(),
                &position_buffer,
                &normal_vector_buffer,
                position_and_normal_buffer_bind_group_layout,
            );

        let chunk_submesh_and_argument_buffer_bind_group =
            Self::create_submesh_and_argument_buffer_bind_group(
                graphics_device.device(),
                &chunk_submesh_buffer,
                &indirect_argument_buffer,
                chunk_submesh_and_argument_buffer_bind_group_layout,
            );

        let chunk_submesh_and_indexed_argument_buffer_bind_group =
            Self::create_submesh_and_argument_buffer_bind_group(
                graphics_device.device(),
                &chunk_submesh_buffer,
                &indexed_indirect_argument_buffer,
                chunk_submesh_and_argument_buffer_bind_group_layout,
            );

        Self {
            chunk_extent: voxel_object.chunk_extent(),
            position_buffer,
            normal_vector_buffer,
            index_material_buffer,
            index_buffer,
            n_indices: mesh.indices().len(),
            chunk_submesh_buffer,
            n_chunks: mesh.n_chunks(),
            indirect_argument_buffer,
            indexed_indirect_argument_buffer,
            position_and_normal_buffer_bind_group,
            chunk_submesh_and_argument_buffer_bind_group,
            chunk_submesh_and_indexed_argument_buffer_bind_group,
        }
    }

    /// Returns the extent of a single voxel chunk in the object.
    pub fn chunk_extent(&self) -> f64 {
        self.chunk_extent
    }

    /// Return a reference to the [`GPUBuffer`] holding all the vertex positions
    /// in the object's mesh.
    pub fn vertex_position_gpu_buffer(&self) -> &GPUBuffer {
        &self.position_buffer
    }

    /// Return a reference to the [`GPUBuffer`] holding all the vertex normal
    /// vectors in the object's mesh.
    pub fn vertex_normal_vector_gpu_buffer(&self) -> &GPUBuffer {
        &self.normal_vector_buffer
    }

    /// Return a reference to the [`GPUBuffer`] holding all the index materials
    /// in the object's mesh.
    pub fn index_material_gpu_buffer(&self) -> &GPUBuffer {
        &self.index_material_buffer
    }

    /// Return a reference to the [`GPUBuffer`] holding all the indices defining
    /// the triangles in the object's mesh.
    pub fn index_gpu_buffer(&self) -> &GPUBuffer {
        &self.index_buffer
    }

    /// Returns the total number of indices in the index buffer.
    pub fn n_indices(&self) -> usize {
        self.n_indices
    }

    /// Returns the GPU buffer containing the submesh data for each chunk.
    pub fn chunk_submesh_gpu_buffer(&self) -> &GPUBuffer {
        &self.chunk_submesh_buffer
    }

    /// Returns the total number of chunks in the chunk submesh buffer.
    pub fn n_chunks(&self) -> usize {
        self.n_chunks
    }

    /// Returns the GPU buffer containing the non-indexed indirect draw call
    /// arguments for each chunk.
    pub fn indirect_argument_gpu_buffer(&self) -> &GPUBuffer {
        &self.indirect_argument_buffer
    }

    /// Returns the GPU buffer containing the indexed indirect draw call
    /// arguments for each chunk.
    pub fn indexed_indirect_argument_gpu_buffer(&self) -> &GPUBuffer {
        &self.indexed_indirect_argument_buffer
    }

    /// Returns the layout of the bind group for the position and normal
    /// buffers, after creating and caching it if it has not already been
    /// created.
    pub fn get_or_create_position_and_normal_buffer_bind_group_layout(
        graphics_device: &GraphicsDevice,
    ) -> &wgpu::BindGroupLayout {
        POSITION_AND_NORMAL_BUFFER_BIND_GROUP_LAYOUT.get_or_init(|| {
            Self::create_position_and_normal_buffer_bind_group_layout(graphics_device.device())
        })
    }

    /// Returns the layout of the bind group for the chunk submesh and indirect
    /// argument buffers, after creating and caching it if it has not already
    /// been created.
    pub fn get_or_create_submesh_and_argument_buffer_bind_group_layout(
        graphics_device: &GraphicsDevice,
    ) -> &wgpu::BindGroupLayout {
        CHUNK_SUBMESH_AND_ARGUMENT_BUFFER_BIND_GROUP_LAYOUT.get_or_init(|| {
            Self::create_submesh_and_argument_buffer_bind_group_layout(graphics_device.device())
        })
    }

    /// Returns a reference to the bind group for the position and normal
    /// buffers.
    pub fn position_and_normal_buffer_bind_group(&self) -> &wgpu::BindGroup {
        &self.position_and_normal_buffer_bind_group
    }

    /// Returns a reference to the bind group for the chunk submesh and
    /// non-indexed indirect argument buffers.
    pub fn submesh_and_argument_buffer_bind_group(&self) -> &wgpu::BindGroup {
        &self.chunk_submesh_and_argument_buffer_bind_group
    }

    /// Returns a reference to the bind group for the chunk submesh and indexed
    /// indirect argument buffers.
    pub fn submesh_and_indexed_argument_buffer_bind_group(&self) -> &wgpu::BindGroup {
        &self.chunk_submesh_and_indexed_argument_buffer_bind_group
    }

    fn create_position_and_normal_buffer_bind_group_layout(
        device: &wgpu::Device,
    ) -> wgpu::BindGroupLayout {
        let position_buffer_layout = storage::create_storage_buffer_bind_group_layout_entry(
            0,
            wgpu::ShaderStages::VERTEX,
            true,
        );

        let normal_buffer_layout = storage::create_storage_buffer_bind_group_layout_entry(
            1,
            wgpu::ShaderStages::VERTEX,
            true,
        );

        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[position_buffer_layout, normal_buffer_layout],
            label: Some("Voxel object position and normal buffer bind group layout"),
        })
    }

    fn create_submesh_and_argument_buffer_bind_group_layout(
        device: &wgpu::Device,
    ) -> wgpu::BindGroupLayout {
        let chunk_submesh_buffer_layout = storage::create_storage_buffer_bind_group_layout_entry(
            0,
            wgpu::ShaderStages::COMPUTE,
            true,
        );

        let indirect_argument_buffer_layout =
            storage::create_storage_buffer_bind_group_layout_entry(
                1,
                wgpu::ShaderStages::COMPUTE,
                false,
            );

        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[chunk_submesh_buffer_layout, indirect_argument_buffer_layout],
            label: Some("Voxel object submesh and indirect argument buffer bind group layout"),
        })
    }

    fn create_position_and_normal_buffer_bind_group(
        device: &wgpu::Device,
        position_buffer: &GPUBuffer,
        normal_buffer: &GPUBuffer,
        layout: &wgpu::BindGroupLayout,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout,
            entries: &[
                position_buffer.create_bind_group_entry(0),
                normal_buffer.create_bind_group_entry(1),
            ],
            label: Some("Voxel object position and normal buffer bind group"),
        })
    }

    fn create_submesh_and_argument_buffer_bind_group(
        device: &wgpu::Device,
        chunk_submesh_buffer: &GPUBuffer,
        indirect_argument_buffer: &GPUBuffer,
        layout: &wgpu::BindGroupLayout,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout,
            entries: &[
                chunk_submesh_buffer.create_bind_group_entry(0),
                indirect_argument_buffer.create_bind_group_entry(1),
            ],
            label: Some("Voxel object submesh and indirect argument buffer bind group"),
        })
    }

    pub fn sync_with_voxel_object(
        &mut self,
        _graphics_device: &GraphicsDevice,
        _voxel_object: &ChunkedVoxelObject,
    ) {
    }
}

impl UniformBufferable for FixedVoxelMaterialProperties {
    const ID: ConstStringHash64 = ConstStringHash64::new("Fixed voxel material properties");

    fn create_bind_group_layout_entry(
        binding: u32,
        visibility: wgpu::ShaderStages,
    ) -> wgpu::BindGroupLayoutEntry {
        uniform::create_uniform_buffer_bind_group_layout_entry(binding, visibility)
    }
}
assert_uniform_valid!(FixedVoxelMaterialProperties);

impl VertexBufferable for VoxelMeshVertexPosition {
    const BUFFER_LAYOUT: wgpu::VertexBufferLayout<'static> =
        create_vertex_buffer_layout_for_vertex::<Self>(&wgpu::vertex_attr_array![
            VoxelMeshVertexAttributeLocation::Position as u32 => Float32x3,
        ]);
}

impl VertexBufferable for VoxelMeshVertexNormalVector {
    const BUFFER_LAYOUT: wgpu::VertexBufferLayout<'static> =
        create_vertex_buffer_layout_for_vertex::<Self>(&wgpu::vertex_attr_array![
            VoxelMeshVertexAttributeLocation::NormalVector as u32 => Float32x3,
        ]);
}

impl VertexBufferable for VoxelMeshIndexMaterials {
    const BUFFER_LAYOUT: wgpu::VertexBufferLayout<'static> =
        create_vertex_buffer_layout_for_vertex::<Self>(&wgpu::vertex_attr_array![
            VoxelMeshVertexAttributeLocation::MaterialIndices as u32 => Uint8x4,
            VoxelMeshVertexAttributeLocation::MaterialWeights as u32 => Uint8x4,
        ]);
}

impl VoxelMeshIndex {
    pub const fn format() -> wgpu::IndexFormat {
        wgpu::IndexFormat::Uint32
    }
}

impl VertexBufferable for VoxelMeshIndex {
    const BUFFER_LAYOUT: wgpu::VertexBufferLayout<'static> =
        create_vertex_buffer_layout_for_vertex::<Self>(&wgpu::vertex_attr_array![
            VoxelMeshVertexAttributeLocation::Indices as u32 => Uint32,
        ]);
}
