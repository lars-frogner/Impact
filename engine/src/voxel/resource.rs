//! Rendering resources for voxel objects.

use crate::{
    assets::Assets,
    voxel::{
        VoxelObjectID,
        mesh::{
            ChunkSubmesh, CullingFrustum, MeshedChunkedVoxelObject, VoxelMeshIndex,
            VoxelMeshIndexMaterials, VoxelMeshModifications, VoxelMeshVertexNormalVector,
            VoxelMeshVertexPosition,
        },
        voxel_types::{FixedVoxelMaterialProperties, VoxelTypeRegistry},
    },
};
use anyhow::Result;
use bytemuck::Pod;
use impact_gpu::{
    assert_uniform_valid,
    bind_group_layout::BindGroupLayoutRegistry,
    buffer::{GPUBuffer, GPUBufferType},
    device::GraphicsDevice,
    indirect::{DrawIndexedIndirectArgs, DrawIndirectArgs},
    push_constant::{PushConstant, PushConstantGroup, PushConstantVariant},
    storage,
    texture::{
        self, ColorSpace, Sampler, SamplerConfig, TexelDescription, Texture,
        TextureAddressingConfig, TextureConfig, TextureFilteringConfig,
    },
    uniform::{self, UniformBufferable},
    wgpu,
};
use impact_math::ConstStringHash64;
use impact_mesh::buffer::{
    MeshVertexAttributeLocation, VertexBufferable, create_vertex_buffer_layout_for_vertex,
    new_vertex_gpu_buffer_with_spare_capacity,
};
use impact_rendering::push_constant::BasicPushConstantVariant;
use std::{borrow::Cow, mem, ops::Range};

/// Owner and manager of the GPU resources for all voxel materials.
#[derive(Debug)]
pub struct VoxelMaterialGPUResourceManager {
    n_voxel_types: usize,
    _fixed_property_buffer: GPUBuffer,
    bind_group: wgpu::BindGroup,
}

/// Owner and manager of GPU buffers for a
/// [`ChunkedVoxelObject`](crate::voxel::chunks::ChunkedVoxelObject).
#[derive(Debug)]
pub struct VoxelObjectGPUBufferManager {
    chunk_extent: f64,
    origin_offset_in_root: [f32; 3],
    position_buffer: GPUBuffer,
    normal_vector_buffer: GPUBuffer,
    index_material_buffer: GPUBuffer,
    index_buffer: GPUBuffer,
    chunk_submesh_buffer: GPUBuffer,
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

pub type VoxelPushConstant = PushConstant<VoxelPushConstantVariant>;
pub type VoxelPushConstantGroup = PushConstantGroup<VoxelPushConstantVariant>;

/// The meaning of a push constant used for voxel rendering
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum VoxelPushConstantVariant {
    CullingFrustum,
    ChunkCount,
    Rendering(BasicPushConstantVariant),
}

impl VoxelMaterialGPUResourceManager {
    const MATERIAL_RESOURCES_BIND_GROUP_LAYOUT_ID: ConstStringHash64 =
        ConstStringHash64::new("VoxelMaterialResources");
}

impl VoxelObjectGPUBufferManager {
    const POSITION_AND_NORMAL_BUFFER_BIND_GROUP_LAYOUT_ID: ConstStringHash64 =
        ConstStringHash64::new("VoxelPositionAndNormalBuffer");
    const CHUNK_SUBMESH_AND_ARGUMENT_BUFFER_BIND_GROUP_LAYOUT_ID: ConstStringHash64 =
        ConstStringHash64::new("VoxelChunkSubmeshAndArgumentBuffer");
}

impl VoxelMaterialGPUResourceManager {
    pub const fn fixed_properties_binding() -> u32 {
        0
    }

    pub const fn color_texture_array_binding() -> u32 {
        1
    }

    pub const fn roughness_texture_array_binding() -> u32 {
        2
    }

    pub const fn normal_texture_array_binding() -> u32 {
        3
    }

    pub const fn sampler_binding() -> u32 {
        4
    }

    /// Initializes the material GPU resources for all voxel types in the given
    /// registry.
    pub fn for_voxel_type_registry(
        graphics_device: &GraphicsDevice,
        assets: &mut Assets,
        voxel_type_registry: &VoxelTypeRegistry,
        bind_group_layout_registry: &BindGroupLayoutRegistry,
    ) -> Result<Self> {
        let fixed_property_buffer = GPUBuffer::new_full_uniform_buffer(
            graphics_device,
            voxel_type_registry.fixed_material_properties(),
            Cow::Borrowed("Fixed voxel material properties"),
        );

        let color_texture_array_id = assets.load_texture_array_from_paths(
            "voxel_color_texture_array",
            voxel_type_registry.color_texture_paths(),
            TextureConfig {
                color_space: ColorSpace::Srgb,
                max_mip_level_count: None,
            },
            Some(SamplerConfig {
                addressing: TextureAddressingConfig::Repeating,
                filtering: TextureFilteringConfig::Basic,
            }),
        )?;

        let roughness_texture_array_id = assets.load_texture_array_from_paths(
            "voxel_roughness_texture_array",
            voxel_type_registry.roughness_texture_paths(),
            TextureConfig {
                color_space: ColorSpace::Linear,
                max_mip_level_count: None,
            },
            None,
        )?;

        let normal_texture_array_id = assets.load_texture_array_from_paths(
            "voxel_normal_texture_array",
            voxel_type_registry.normal_texture_paths(),
            TextureConfig {
                color_space: ColorSpace::Linear,
                max_mip_level_count: None,
            },
            None,
        )?;

        let color_texture_array = &assets.textures[&color_texture_array_id];
        let roughness_texture_array = &assets.textures[&roughness_texture_array_id];
        let normal_texture_array = &assets.textures[&normal_texture_array_id];

        let sampler = &assets.samplers[&color_texture_array.sampler_id().unwrap()];

        let bind_group_layout =
            Self::get_or_create_bind_group_layout(graphics_device, bind_group_layout_registry);
        let bind_group = Self::create_bind_group(
            graphics_device.device(),
            &bind_group_layout,
            &fixed_property_buffer,
            color_texture_array,
            roughness_texture_array,
            normal_texture_array,
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
        bind_group_layout_registry: &BindGroupLayoutRegistry,
    ) -> wgpu::BindGroupLayout {
        bind_group_layout_registry
            .get_or_create_layout(Self::MATERIAL_RESOURCES_BIND_GROUP_LAYOUT_ID, || {
                Self::create_bind_group_layout(graphics_device.device())
            })
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
                texture::create_texture_bind_group_layout_entry(
                    Self::roughness_texture_array_binding(),
                    wgpu::ShaderStages::FRAGMENT,
                    TexelDescription::Grayscale8.texture_format(),
                    wgpu::TextureViewDimension::D2Array,
                ),
                texture::create_texture_bind_group_layout_entry(
                    Self::normal_texture_array_binding(),
                    wgpu::ShaderStages::FRAGMENT,
                    TexelDescription::Rgba8(ColorSpace::Linear).texture_format(),
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
        roughness_texture_array: &Texture,
        normal_texture_array: &Texture,
        sampler: &Sampler,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout,
            entries: &[
                fixed_property_buffer.create_bind_group_entry(Self::fixed_properties_binding()),
                color_texture_array.create_bind_group_entry(Self::color_texture_array_binding()),
                roughness_texture_array
                    .create_bind_group_entry(Self::roughness_texture_array_binding()),
                normal_texture_array.create_bind_group_entry(Self::normal_texture_array_binding()),
                sampler.create_bind_group_entry(Self::sampler_binding()),
            ],
            label: Some("Voxel material bind group"),
        })
    }
}

impl VoxelObjectGPUBufferManager {
    /// Creates a new manager of GPU resources for the given
    /// [`MeshedChunkedVoxelObject`]. This involves initializing GPU buffers for
    /// the relevant data in the object's
    /// [`ChunkedVoxelObjectMesh`](crate::voxel::mesh::ChunkedVoxelObjectMesh).
    pub fn for_voxel_object(
        graphics_device: &GraphicsDevice,
        voxel_object_id: VoxelObjectID,
        voxel_object: &MeshedChunkedVoxelObject,
        bind_group_layout_registry: &BindGroupLayoutRegistry,
    ) -> Self {
        let mesh = voxel_object.mesh();

        let position_buffer = Self::create_position_buffer(
            graphics_device,
            mesh.positions(),
            Cow::Owned(format!("{voxel_object_id} vertex position")),
        );

        let normal_vector_buffer = Self::create_normal_vector_buffer(
            graphics_device,
            mesh.normal_vectors(),
            Cow::Owned(format!("{voxel_object_id} normal vector")),
        );

        let index_material_buffer = Self::create_index_material_buffer(
            graphics_device,
            mesh.index_materials(),
            Cow::Owned(format!("{voxel_object_id} index material")),
        );

        let index_buffer = Self::create_index_buffer(
            graphics_device,
            mesh.indices(),
            Cow::Owned(format!("{voxel_object_id}")),
        );

        let chunk_submesh_buffer = Self::create_chunk_submesh_buffer(
            graphics_device,
            mesh.chunk_submeshes(),
            Cow::Owned(format!("{voxel_object_id} chunk info")),
        );

        let indirect_argument_buffer = Self::create_indirect_argument_buffer(
            graphics_device,
            mesh.n_chunks(),
            Cow::Owned(format!("{voxel_object_id} draw argument")),
        );

        let indexed_indirect_argument_buffer = Self::create_indexed_indirect_argument_buffer(
            graphics_device,
            mesh.n_chunks(),
            Cow::Owned(format!("{voxel_object_id} indexed draw argument")),
        );

        let position_and_normal_buffer_bind_group_layout =
            Self::get_or_create_position_and_normal_buffer_bind_group_layout(
                graphics_device,
                bind_group_layout_registry,
            );

        let chunk_submesh_and_argument_buffer_bind_group_layout =
            Self::get_or_create_submesh_and_argument_buffer_bind_group_layout(
                graphics_device,
                bind_group_layout_registry,
            );

        let position_and_normal_buffer_bind_group =
            Self::create_position_and_normal_buffer_bind_group(
                graphics_device.device(),
                &position_buffer,
                &normal_vector_buffer,
                &position_and_normal_buffer_bind_group_layout,
            );

        let chunk_submesh_and_argument_buffer_bind_group =
            Self::create_submesh_and_argument_buffer_bind_group(
                graphics_device.device(),
                &chunk_submesh_buffer,
                &indirect_argument_buffer,
                &chunk_submesh_and_argument_buffer_bind_group_layout,
            );

        let chunk_submesh_and_indexed_argument_buffer_bind_group =
            Self::create_submesh_and_argument_buffer_bind_group(
                graphics_device.device(),
                &chunk_submesh_buffer,
                &indexed_indirect_argument_buffer,
                &chunk_submesh_and_argument_buffer_bind_group_layout,
            );

        Self {
            chunk_extent: voxel_object.object().chunk_extent(),
            origin_offset_in_root: voxel_object.object().origin_offset_in_root(),
            position_buffer,
            normal_vector_buffer,
            index_material_buffer,
            index_buffer,
            chunk_submesh_buffer,
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

    /// Returns the offsets of the origin of this object compared to the origin
    /// of the original unsplit object this object was disconnected from, in the
    /// reference frame of the original object (the disconnected object has the
    /// same orientation as the original object after splitting, only the offset
    /// is different). This does not account for any relative motion of the
    /// objects after splitting. If this object has not been disconnected from a
    /// larger object, the offsets are zero.
    ///
    /// This is needed to offset the texture coordinates for triplanar texture
    /// mapping in disconnected objects. Without this offset, their texture
    /// coordinates would change discontinuously right after a split because the
    /// vertices of the disconnected object are computed relative to the new
    /// origin.
    pub fn origin_offset_in_root(&self) -> [f32; 3] {
        self.origin_offset_in_root
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

    /// Returns the GPU buffer containing the submesh data for each chunk.
    pub fn chunk_submesh_gpu_buffer(&self) -> &GPUBuffer {
        &self.chunk_submesh_buffer
    }

    /// Returns the total number of chunks in the chunk submesh buffer.
    pub fn n_chunks(&self) -> usize {
        self.chunk_submesh_buffer.n_valid_bytes() / mem::size_of::<ChunkSubmesh>()
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
        bind_group_layout_registry: &BindGroupLayoutRegistry,
    ) -> wgpu::BindGroupLayout {
        bind_group_layout_registry.get_or_create_layout(
            Self::POSITION_AND_NORMAL_BUFFER_BIND_GROUP_LAYOUT_ID,
            || Self::create_position_and_normal_buffer_bind_group_layout(graphics_device.device()),
        )
    }

    /// Returns the layout of the bind group for the chunk submesh and indirect
    /// argument buffers, after creating and caching it if it has not already
    /// been created.
    pub fn get_or_create_submesh_and_argument_buffer_bind_group_layout(
        graphics_device: &GraphicsDevice,
        bind_group_layout_registry: &BindGroupLayoutRegistry,
    ) -> wgpu::BindGroupLayout {
        bind_group_layout_registry.get_or_create_layout(
            Self::CHUNK_SUBMESH_AND_ARGUMENT_BUFFER_BIND_GROUP_LAYOUT_ID,
            || Self::create_submesh_and_argument_buffer_bind_group_layout(graphics_device.device()),
        )
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

    /// Synchronizes any updated data in the voxel object mesh with the
    /// appropriate GPU buffers.
    pub fn sync_with_voxel_object(
        &mut self,
        graphics_device: &GraphicsDevice,
        voxel_object: &mut MeshedChunkedVoxelObject,
        bind_group_layout_registry: &BindGroupLayoutRegistry,
    ) {
        let mesh = voxel_object.mesh();

        let VoxelMeshModifications {
            updated_chunk_submesh_data_ranges: updated_data_ranges,
            chunks_were_removed,
        } = mesh.mesh_modifications();

        if updated_data_ranges.is_empty() && !chunks_were_removed {
            return;
        }

        if updated_data_ranges.is_empty() && chunks_were_removed {
            // If the only modifications are removed chunks, we just copy over the new chunk
            // buffer and call it a day
            self.chunk_submesh_buffer.update_valid_bytes(
                graphics_device,
                bytemuck::cast_slice(mesh.chunk_submeshes()),
            );

            voxel_object.report_gpu_resources_synchronized();

            return;
        }

        if mem::size_of_val(mesh.positions()) > self.position_buffer.buffer_size() {
            // If the current size of the mesh's position slice exceeds the
            // current buffer size, we have to recreate the position and normal
            // vector buffers (each of these have room for the same number of
            // elements) and their bind group

            self.position_buffer = Self::create_position_buffer(
                graphics_device,
                mesh.positions(),
                self.position_buffer.label().clone(),
            );
            self.normal_vector_buffer = Self::create_normal_vector_buffer(
                graphics_device,
                mesh.normal_vectors(),
                self.normal_vector_buffer.label().clone(),
            );

            let position_and_normal_buffer_bind_group_layout =
                Self::get_or_create_position_and_normal_buffer_bind_group_layout(
                    graphics_device,
                    bind_group_layout_registry,
                );

            self.position_and_normal_buffer_bind_group =
                Self::create_position_and_normal_buffer_bind_group(
                    graphics_device.device(),
                    &self.position_buffer,
                    &self.normal_vector_buffer,
                    &position_and_normal_buffer_bind_group_layout,
                );
        } else {
            // If the updated vertex data still fits in the existing buffers, we write each
            // updated range to the buffers
            for ranges in updated_data_ranges {
                let vertex_range = ranges.vertex_range.clone();
                Self::update_buffer_range(
                    graphics_device,
                    &self.position_buffer,
                    mesh.positions(),
                    vertex_range.clone(),
                );
                Self::update_buffer_range(
                    graphics_device,
                    &self.normal_vector_buffer,
                    mesh.normal_vectors(),
                    vertex_range,
                );
            }
        }

        if mem::size_of_val(mesh.indices()) > self.index_buffer.buffer_size() {
            // If the current size of the mesh's index slice exceeds the current buffer
            // size, we have to recreate the index and index material buffers (each of these
            // have room for the same number of elements)

            self.index_material_buffer = Self::create_index_material_buffer(
                graphics_device,
                mesh.index_materials(),
                self.index_material_buffer.label().clone(),
            );
            self.index_buffer = Self::create_index_buffer(
                graphics_device,
                mesh.indices(),
                self.index_buffer.label().clone(),
            );
        } else {
            // If the updated index data still fits in the existing buffers, we write each
            // updated range to the buffers
            for ranges in updated_data_ranges {
                let index_range = ranges.index_range.clone();
                Self::update_buffer_range(
                    graphics_device,
                    &self.index_material_buffer,
                    mesh.index_materials(),
                    index_range.clone(),
                );
                Self::update_buffer_range(
                    graphics_device,
                    &self.index_buffer,
                    mesh.indices(),
                    index_range,
                );
            }
        }

        if mem::size_of_val(mesh.chunk_submeshes()) > self.chunk_submesh_buffer.buffer_size() {
            // If the current size of the mesh's chunk slice exceeds the current buffer
            // size, we have to recreate the relevant buffers (each of these
            // have room for the same number of elements) and their bind groups

            self.chunk_submesh_buffer = Self::create_chunk_submesh_buffer(
                graphics_device,
                mesh.chunk_submeshes(),
                self.chunk_submesh_buffer.label().clone(),
            );
            self.indirect_argument_buffer = Self::create_indirect_argument_buffer(
                graphics_device,
                mesh.n_chunks(),
                self.indirect_argument_buffer.label().clone(),
            );
            self.indexed_indirect_argument_buffer = Self::create_indexed_indirect_argument_buffer(
                graphics_device,
                mesh.n_chunks(),
                self.indexed_indirect_argument_buffer.label().clone(),
            );

            let chunk_submesh_and_argument_buffer_bind_group_layout =
                Self::get_or_create_submesh_and_argument_buffer_bind_group_layout(
                    graphics_device,
                    bind_group_layout_registry,
                );

            self.chunk_submesh_and_argument_buffer_bind_group =
                Self::create_submesh_and_argument_buffer_bind_group(
                    graphics_device.device(),
                    &self.chunk_submesh_buffer,
                    &self.indirect_argument_buffer,
                    &chunk_submesh_and_argument_buffer_bind_group_layout,
                );

            self.chunk_submesh_and_indexed_argument_buffer_bind_group =
                Self::create_submesh_and_argument_buffer_bind_group(
                    graphics_device.device(),
                    &self.chunk_submesh_buffer,
                    &self.indexed_indirect_argument_buffer,
                    &chunk_submesh_and_argument_buffer_bind_group_layout,
                );
        } else {
            // If the updated chunks still fit in the existing buffer, we simply overwrite
            // the existing chunk buffer with the new data (since this buffer is relatively
            // small, we don't bother writing only the parts that actually changed)
            self.chunk_submesh_buffer.update_valid_bytes(
                graphics_device,
                bytemuck::cast_slice(mesh.chunk_submeshes()),
            );
        }

        voxel_object.report_gpu_resources_synchronized();
    }

    fn create_position_buffer(
        graphics_device: &GraphicsDevice,
        positions: &[VoxelMeshVertexPosition],
        label: Cow<'static, str>,
    ) -> GPUBuffer {
        let buffer_size = mem::size_of::<VoxelMeshVertexPosition>()
            .checked_mul(Self::add_spare_buffer_capacity(positions.len()))
            .unwrap();

        // The position buffer is bound as a vertex buffer for indexed drawing (when
        // updating shadow maps) and as a storage buffer for the geometry pass
        let usage = GPUBufferType::Vertex.usage() | wgpu::BufferUsages::STORAGE;

        GPUBuffer::new_with_spare_capacity(
            graphics_device,
            buffer_size,
            bytemuck::cast_slice(positions),
            usage,
            label,
        )
    }

    fn create_normal_vector_buffer(
        graphics_device: &GraphicsDevice,
        normal_vectors: &[VoxelMeshVertexNormalVector],
        label: Cow<'static, str>,
    ) -> GPUBuffer {
        let total_capacity = Self::add_spare_buffer_capacity(normal_vectors.len());

        // The normal vector is bound as a storage buffer for the geometry pass
        GPUBuffer::new_storage_buffer_with_spare_capacity(
            graphics_device,
            total_capacity,
            normal_vectors,
            label,
        )
    }

    fn create_index_material_buffer(
        graphics_device: &GraphicsDevice,
        index_materials: &[VoxelMeshIndexMaterials],
        label: Cow<'static, str>,
    ) -> GPUBuffer {
        let total_capacity = Self::add_spare_buffer_capacity(index_materials.len());

        // The the index material buffer is bound as a vertex buffer for the
        // geometry pass
        new_vertex_gpu_buffer_with_spare_capacity(
            graphics_device,
            total_capacity,
            index_materials,
            label,
        )
    }

    fn create_index_buffer(
        graphics_device: &GraphicsDevice,
        indices: &[VoxelMeshIndex],
        label: Cow<'static, str>,
    ) -> GPUBuffer {
        let buffer_size = mem::size_of::<VoxelMeshIndex>()
            .checked_mul(Self::add_spare_buffer_capacity(indices.len()))
            .unwrap();

        // The index buffer is bound as an ordinary index buffer for indexed drawing
        // (when updating shadow maps) and as a vertex buffer for the geometry
        // pass
        let usage = GPUBufferType::Index.usage() | wgpu::BufferUsages::VERTEX;

        GPUBuffer::new_with_spare_capacity(
            graphics_device,
            buffer_size,
            bytemuck::cast_slice(indices),
            usage,
            label,
        )
    }

    fn create_chunk_submesh_buffer(
        graphics_device: &GraphicsDevice,
        chunk_submeshes: &[ChunkSubmesh],
        label: Cow<'static, str>,
    ) -> GPUBuffer {
        let total_capacity = Self::add_spare_buffer_capacity(chunk_submeshes.len());

        // The normal vector is bound as a storage buffer for the geometry pass
        GPUBuffer::new_storage_buffer_with_spare_capacity(
            graphics_device,
            total_capacity,
            chunk_submeshes,
            label,
        )
    }

    fn create_indirect_argument_buffer(
        graphics_device: &GraphicsDevice,
        n_chunks: usize,
        label: Cow<'static, str>,
    ) -> GPUBuffer {
        let total_capacity = Self::add_spare_buffer_capacity(n_chunks);

        // Used for non-indexed indirect draw calls
        GPUBuffer::new_draw_indirect_buffer_with_spare_capacity(
            graphics_device,
            total_capacity,
            &vec![DrawIndirectArgs::default(); n_chunks],
            label,
        )
    }

    fn create_indexed_indirect_argument_buffer(
        graphics_device: &GraphicsDevice,
        n_chunks: usize,
        label: Cow<'static, str>,
    ) -> GPUBuffer {
        let total_capacity = Self::add_spare_buffer_capacity(n_chunks);

        // Used for indexed indirect draw calls
        GPUBuffer::new_draw_indexed_indirect_buffer_with_spare_capacity(
            graphics_device,
            total_capacity,
            &vec![DrawIndexedIndirectArgs::default(); n_chunks],
            label,
        )
    }

    /// To avoid frequent buffer recreation as the mesh is updated, we make room
    /// for some extra elements whenever we create a buffer.
    fn add_spare_buffer_capacity(current_size: usize) -> usize {
        3 * current_size / 2
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

    fn update_buffer_range<T: Pod>(
        graphics_device: &GraphicsDevice,
        buffer: &GPUBuffer,
        all_values: &[T],
        range: Range<usize>,
    ) {
        let byte_offset = mem::size_of::<T>().checked_mul(range.start).unwrap();

        buffer.update_bytes_from_offset(
            graphics_device,
            byte_offset,
            bytemuck::cast_slice(&all_values[range]),
        );
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

impl PushConstantVariant for VoxelPushConstantVariant {
    fn size(&self) -> u32 {
        match self {
            Self::CullingFrustum => mem::size_of::<CullingFrustum>() as u32,
            Self::ChunkCount => mem::size_of::<u32>() as u32,
            Self::Rendering(variant) => variant.size(),
        }
    }
}
