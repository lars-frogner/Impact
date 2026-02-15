//! Rendering resources for voxel objects.

use crate::{
    VoxelObjectID, VoxelObjectManager,
    mesh::{
        ChunkSubmesh, CullingFrustum, MeshedChunkedVoxelObject, VoxelMeshIndex,
        VoxelMeshIndexMaterials, VoxelMeshModifications, VoxelMeshVertexNormalVector,
        VoxelMeshVertexPosition,
    },
    voxel_types::{FixedVoxelMaterialProperties, VoxelTypeRegistry},
};
use anyhow::{Result, anyhow};
use bytemuck::Pod;
use impact_containers::HashMap;
use impact_gpu::{
    assert_uniform_valid,
    bind_group_layout::BindGroupLayoutRegistry,
    buffer::{GPUBuffer, GPUBufferType},
    device::GraphicsDevice,
    indirect::{DrawIndexedIndirectArgs, DrawIndirectArgs},
    push_constant::{PushConstant, PushConstantGroup, PushConstantVariant},
    storage,
    texture::{self, ColorSpace, Sampler, TexelDescription, Texture},
    uniform::{self, UniformBufferable},
    wgpu,
};
use impact_math::hash::{ConstStringHash64, Hash64};
use impact_mesh::gpu_resource::{
    MeshVertexAttributeLocation, VertexBufferable, create_vertex_buffer_layout_for_vertex,
    new_vertex_gpu_buffer_with_spare_capacity_and_encoded_initialization,
};
use impact_model::{
    InstanceFeature, InstanceFeatureBufferRangeID, InstanceFeatureBufferRangeMap,
    transform::{InstanceModelLightTransform, InstanceModelViewTransformWithPrevious},
};
use impact_rendering::push_constant::BasicPushConstantVariant;
use impact_scene::model::{ModelID, ModelInstanceManager};
use impact_texture::gpu_resource::{SamplerMap, TextureMap};
use std::{borrow::Cow, mem, ops::Range, sync::LazyLock};

pub static VOXEL_MODEL_ID: LazyLock<ModelID> =
    LazyLock::new(|| ModelID::hash_only(Hash64::from_str("Voxel model")));

pub trait VoxelResourceRegistries {
    fn voxel_type(&self) -> &VoxelTypeRegistry;
}

pub trait VoxelGPUResources {
    /// Returns the GPU resources for voxel materials, or [`None`] if they have
    /// not been initialized.
    fn voxel_materials(&self) -> Option<&VoxelMaterialGPUResources>;

    /// Returns the GPU resources for voxel objects.
    fn voxel_objects(&self) -> &VoxelObjectGPUResources;
}

/// GPU resources for all voxel materials.
#[derive(Debug)]
pub struct VoxelMaterialGPUResources {
    n_voxel_types: usize,
    _fixed_property_buffer: GPUBuffer,
    bind_group: wgpu::BindGroup,
}

/// GPU resources for voxel objects.
#[derive(Debug)]
pub struct VoxelObjectGPUResources {
    visible_object_ids: Vec<VoxelObjectID>,
    visible_object_model_view_transforms: Vec<InstanceModelViewTransformWithPrevious>,
    visible_object_model_light_transforms: Vec<InstanceModelLightTransform>,
    id_ranges: InstanceFeatureBufferRangeMap,
    model_light_transform_ranges: InstanceFeatureBufferRangeMap,
    buffers: HashMap<VoxelObjectID, VoxelObjectGPUBuffers>,
}

/// GPU buffers for a [`ChunkedVoxelObject`](crate::chunks::ChunkedVoxelObject).
#[derive(Debug)]
pub struct VoxelObjectGPUBuffers {
    chunk_extent: f32,
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

impl VoxelMaterialGPUResources {
    const MATERIAL_RESOURCES_BIND_GROUP_LAYOUT_ID: ConstStringHash64 =
        ConstStringHash64::new("VoxelMaterialResources");
}

impl VoxelObjectGPUResources {
    pub fn new() -> Self {
        Self {
            visible_object_ids: Vec::new(),
            visible_object_model_view_transforms: Vec::new(),
            visible_object_model_light_transforms: Vec::new(),
            id_ranges: InstanceFeatureBufferRangeMap::empty(),
            model_light_transform_ranges: InstanceFeatureBufferRangeMap::empty(),
            buffers: HashMap::default(),
        }
    }

    /// Whether there are currently any voxel objects (visible or otherwise).
    pub fn has_voxel_objects(&self) -> bool {
        !self.buffers.is_empty()
    }

    /// Returns the current number of voxel objects (visible or otherwise).
    pub fn voxel_object_count(&self) -> usize {
        self.buffers.len()
    }

    /// Whether there are currently visible voxel objects.
    pub fn has_visible_voxel_objects(&self) -> bool {
        self.visible_voxel_object_count() > 0
    }

    /// Returns the number of currently visible voxel objects.
    pub fn visible_voxel_object_count(&self) -> usize {
        self.visible_object_ids.len()
    }

    /// Returns the IDs of the currently visible voxel objects in the initial
    /// instance range.
    pub fn visible_voxel_object_ids_in_initial_range(&self) -> &[VoxelObjectID] {
        self.get_visible_voxel_object_ids_in_range(InstanceFeatureBufferRangeMap::INITIAL_RANGE_ID)
            .unwrap()
    }

    /// Returns the IDs of the currently visible voxel objects in the specified
    /// instance range, or [`None`] if there is no such instance range.
    pub fn get_visible_voxel_object_ids_in_range(
        &self,
        instance_range_id: InstanceFeatureBufferRangeID,
    ) -> Option<&[VoxelObjectID]> {
        let range = self
            .id_ranges
            .get_range(instance_range_id, self.visible_object_ids.len() as u32)?;
        Some(&self.visible_object_ids[range.start as usize..range.end as usize])
    }

    /// Returns the model-light transforms of the currently visible voxel
    /// objects in the specified instance range, as well as the range itself.
    pub fn get_visible_object_model_light_transforms_in_range(
        &self,
        instance_range_id: InstanceFeatureBufferRangeID,
    ) -> Option<(&[InstanceModelLightTransform], Range<u32>)> {
        let range = self.model_light_transform_ranges.get_range(
            instance_range_id,
            self.visible_object_model_light_transforms.len() as u32,
        )?;
        let transforms =
            &self.visible_object_model_light_transforms[range.start as usize..range.end as usize];
        Some((transforms, range))
    }

    /// Returns the model-view transforms of all currently visible voxel
    /// objects, as well as their range in the instance feature buffer.
    pub fn visible_object_model_view_transforms(
        &self,
    ) -> (&[InstanceModelViewTransformWithPrevious], Range<u32>) {
        (
            &self.visible_object_model_view_transforms,
            0..self.visible_object_model_view_transforms.len() as u32,
        )
    }

    /// Returns the GPU buffers for the given voxel object identifier if the
    /// voxel object exists, otherwise returns [`None`].
    pub fn get_voxel_object_buffers(
        &self,
        voxel_object_id: VoxelObjectID,
    ) -> Option<&VoxelObjectGPUBuffers> {
        self.buffers.get(&voxel_object_id)
    }

    /// Removes the GPU buffers for the given voxel object identifier.
    pub fn remove_voxel_object_buffers(&mut self, voxel_object_id: VoxelObjectID) {
        self.buffers.remove(&voxel_object_id);
    }

    /// Performs any required updates for keeping the voxel object GPU resources
    /// in sync with the voxel object data.
    ///
    /// GPU resources whose source data no longer exists will be removed, and
    /// missing GPU resources for new source data will be created.
    pub fn sync_buffers_with_manager(
        &mut self,
        graphics_device: &GraphicsDevice,
        staging_belt: &mut wgpu::util::StagingBelt,
        command_encoder: &mut wgpu::CommandEncoder,
        bind_group_layout_registry: &BindGroupLayoutRegistry,
        voxel_object_manager: &mut VoxelObjectManager,
    ) {
        for (voxel_object_id, voxel_object) in voxel_object_manager.voxel_objects_mut() {
            if voxel_object.object().contains_only_empty_voxels() {
                continue;
            }
            self.buffers
                .entry(*voxel_object_id)
                .and_modify(|buffers| {
                    buffers.sync_with_voxel_object(
                        graphics_device,
                        staging_belt,
                        command_encoder,
                        voxel_object,
                        bind_group_layout_registry,
                    );
                })
                .or_insert_with(|| {
                    VoxelObjectGPUBuffers::for_voxel_object(
                        graphics_device,
                        staging_belt,
                        command_encoder,
                        *voxel_object_id,
                        voxel_object,
                        bind_group_layout_registry,
                    )
                });
        }

        // TODO: reuse orphaned buffers
        self.buffers.retain(|id, _| {
            voxel_object_manager
                .get_voxel_object(*id)
                .is_some_and(|voxel_object| !voxel_object.object().contains_only_empty_voxels())
        });
    }

    /// Updates the lists of properties for the visible voxel objects based on
    /// the currently buffered voxel object instances in the model instance
    /// manager.
    pub fn sync_visible_objects(&mut self, model_instance_manager: &ModelInstanceManager) {
        if self.buffers.is_empty() {
            return;
        }

        let instance_buffer = model_instance_manager
            .get_model_instance_buffer(&VOXEL_MODEL_ID)
            .expect("Missing model instance buffer for voxel objects");

        let id_buffer = instance_buffer
            .get_feature_buffer(VoxelObjectID::FEATURE_TYPE_ID)
            .expect("Missing voxel object ID instance feature buffer for voxel objects");

        let model_view_transform_buffer = instance_buffer
            .get_feature_buffer(InstanceModelViewTransformWithPrevious::FEATURE_TYPE_ID)
            .expect("Missing model view transform instance feature buffer for voxel objects");

        let model_light_transform_buffer = instance_buffer
            .get_feature_buffer(InstanceModelLightTransform::FEATURE_TYPE_ID)
            .expect("Missing model light transform instance feature buffer for voxel objects");

        self.visible_object_ids.clear();
        self.visible_object_ids
            .extend_from_slice(id_buffer.valid_features());

        self.visible_object_model_view_transforms.clear();
        self.visible_object_model_view_transforms
            .extend_from_slice(model_view_transform_buffer.valid_features());

        self.visible_object_model_light_transforms.clear();
        self.visible_object_model_light_transforms
            .extend_from_slice(model_light_transform_buffer.valid_features());

        id_buffer.update_range_map(&mut self.id_ranges);
        model_light_transform_buffer.update_range_map(&mut self.model_light_transform_ranges);

        // We expect there to be no ranges apart from the initial range for the
        // model view transforms
        assert_eq!(
            model_view_transform_buffer
                .initial_valid_feature_range()
                .len(),
            self.visible_object_model_view_transforms.len()
        );
    }
}

impl Default for VoxelObjectGPUResources {
    fn default() -> Self {
        Self::new()
    }
}

impl VoxelObjectGPUBuffers {
    const POSITION_AND_NORMAL_BUFFER_BIND_GROUP_LAYOUT_ID: ConstStringHash64 =
        ConstStringHash64::new("VoxelPositionAndNormalBuffer");
    const CHUNK_SUBMESH_AND_ARGUMENT_BUFFER_BIND_GROUP_LAYOUT_ID: ConstStringHash64 =
        ConstStringHash64::new("VoxelChunkSubmeshAndArgumentBuffer");
}

impl VoxelMaterialGPUResources {
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
        textures: &TextureMap,
        samplers: &SamplerMap,
        voxel_type_registry: &VoxelTypeRegistry,
        bind_group_layout_registry: &BindGroupLayoutRegistry,
    ) -> Result<Self> {
        let fixed_property_buffer = GPUBuffer::new_full_uniform_buffer(
            graphics_device,
            voxel_type_registry.fixed_material_properties(),
            Cow::Borrowed("Fixed voxel material properties"),
        );

        let color_texture_array = textures
            .get(voxel_type_registry.color_texture_array_id().unwrap())
            .ok_or_else(|| anyhow!("Missing voxel material color texture array"))?;

        let roughness_texture_array = textures
            .get(voxel_type_registry.roughness_texture_array_id().unwrap())
            .ok_or_else(|| anyhow!("Missing voxel material roughness texture array"))?;

        let normal_texture_array = textures
            .get(voxel_type_registry.normal_texture_array_id().unwrap())
            .ok_or_else(|| anyhow!("Missing voxel material normal texture array"))?;

        let sampler = samplers
            .get(
                color_texture_array
                    .sampler_id
                    .ok_or_else(|| anyhow!("Voxel material color texture array has no sampler"))?,
            )
            .ok_or_else(|| anyhow!("Missing voxel material texture sampler"))?;

        let bind_group_layout =
            Self::get_or_create_bind_group_layout(graphics_device, bind_group_layout_registry);

        let bind_group = Self::create_bind_group(
            graphics_device.device(),
            &bind_group_layout,
            &fixed_property_buffer,
            &color_texture_array.texture,
            &roughness_texture_array.texture,
            &normal_texture_array.texture,
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
            .get_or_create_layout(Self::MATERIAL_RESOURCES_BIND_GROUP_LAYOUT_ID.hash(), || {
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

impl VoxelObjectGPUBuffers {
    /// Creates new GPU buffers for the given [`MeshedChunkedVoxelObject`],
    /// initializing for the relevant data in the object's
    /// [`ChunkedVoxelObjectMesh`](crate::mesh::ChunkedVoxelObjectMesh).
    pub fn for_voxel_object(
        graphics_device: &GraphicsDevice,
        staging_belt: &mut wgpu::util::StagingBelt,
        command_encoder: &mut wgpu::CommandEncoder,
        voxel_object_id: VoxelObjectID,
        voxel_object: &MeshedChunkedVoxelObject,
        bind_group_layout_registry: &BindGroupLayoutRegistry,
    ) -> Self {
        let mesh = voxel_object.mesh();

        let position_buffer = Self::create_position_buffer(
            graphics_device,
            staging_belt,
            command_encoder,
            mesh.positions(),
            Cow::Owned(format!("{voxel_object_id} vertex position")),
        );

        let normal_vector_buffer = Self::create_normal_vector_buffer(
            graphics_device,
            staging_belt,
            command_encoder,
            mesh.normal_vectors(),
            Cow::Owned(format!("{voxel_object_id} normal vector")),
        );

        let index_material_buffer = Self::create_index_material_buffer(
            graphics_device,
            staging_belt,
            command_encoder,
            mesh.index_materials(),
            Cow::Owned(format!("{voxel_object_id} index material")),
        );

        let index_buffer = Self::create_index_buffer(
            graphics_device,
            staging_belt,
            command_encoder,
            mesh.indices(),
            Cow::Owned(format!("{voxel_object_id}")),
        );

        let chunk_submesh_buffer = Self::create_chunk_submesh_buffer(
            graphics_device,
            staging_belt,
            command_encoder,
            mesh.chunk_submeshes(),
            Cow::Owned(format!("{voxel_object_id} chunk info")),
        );

        let indirect_argument_buffer = Self::create_indirect_argument_buffer(
            graphics_device,
            staging_belt,
            command_encoder,
            mesh.n_chunks(),
            Cow::Owned(format!("{voxel_object_id} draw argument")),
        );

        let indexed_indirect_argument_buffer = Self::create_indexed_indirect_argument_buffer(
            graphics_device,
            staging_belt,
            command_encoder,
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
    pub fn chunk_extent(&self) -> f32 {
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
            Self::POSITION_AND_NORMAL_BUFFER_BIND_GROUP_LAYOUT_ID.hash(),
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
            Self::CHUNK_SUBMESH_AND_ARGUMENT_BUFFER_BIND_GROUP_LAYOUT_ID.hash(),
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
        staging_belt: &mut wgpu::util::StagingBelt,
        command_encoder: &mut wgpu::CommandEncoder,
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
            self.chunk_submesh_buffer.encode_update_of_valid_bytes(
                graphics_device,
                staging_belt,
                command_encoder,
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
                staging_belt,
                command_encoder,
                mesh.positions(),
                self.position_buffer.label().clone(),
            );
            self.normal_vector_buffer = Self::create_normal_vector_buffer(
                graphics_device,
                staging_belt,
                command_encoder,
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
                    staging_belt,
                    command_encoder,
                    &self.position_buffer,
                    mesh.positions(),
                    vertex_range.clone(),
                );
                Self::update_buffer_range(
                    graphics_device,
                    staging_belt,
                    command_encoder,
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
                staging_belt,
                command_encoder,
                mesh.index_materials(),
                self.index_material_buffer.label().clone(),
            );
            self.index_buffer = Self::create_index_buffer(
                graphics_device,
                staging_belt,
                command_encoder,
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
                    staging_belt,
                    command_encoder,
                    &self.index_material_buffer,
                    mesh.index_materials(),
                    index_range.clone(),
                );
                Self::update_buffer_range(
                    graphics_device,
                    staging_belt,
                    command_encoder,
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
                staging_belt,
                command_encoder,
                mesh.chunk_submeshes(),
                self.chunk_submesh_buffer.label().clone(),
            );
            self.indirect_argument_buffer = Self::create_indirect_argument_buffer(
                graphics_device,
                staging_belt,
                command_encoder,
                mesh.n_chunks(),
                self.indirect_argument_buffer.label().clone(),
            );
            self.indexed_indirect_argument_buffer = Self::create_indexed_indirect_argument_buffer(
                graphics_device,
                staging_belt,
                command_encoder,
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
            self.chunk_submesh_buffer.encode_update_of_valid_bytes(
                graphics_device,
                staging_belt,
                command_encoder,
                bytemuck::cast_slice(mesh.chunk_submeshes()),
            );
        }

        voxel_object.report_gpu_resources_synchronized();
    }

    fn create_position_buffer(
        graphics_device: &GraphicsDevice,
        staging_belt: &mut wgpu::util::StagingBelt,
        command_encoder: &mut wgpu::CommandEncoder,
        positions: &[VoxelMeshVertexPosition],
        label: Cow<'static, str>,
    ) -> GPUBuffer {
        let buffer_size = mem::size_of::<VoxelMeshVertexPosition>()
            .checked_mul(Self::add_spare_buffer_capacity(positions.len()))
            .unwrap();

        // The position buffer is bound as a vertex buffer for indexed drawing (when
        // updating shadow maps) and as a storage buffer for the geometry pass
        let usage = GPUBufferType::Vertex.usage() | wgpu::BufferUsages::STORAGE;

        GPUBuffer::new_with_spare_capacity_and_encoded_initialization(
            graphics_device,
            staging_belt,
            command_encoder,
            buffer_size,
            bytemuck::cast_slice(positions),
            usage,
            label,
        )
    }

    fn create_normal_vector_buffer(
        graphics_device: &GraphicsDevice,
        staging_belt: &mut wgpu::util::StagingBelt,
        command_encoder: &mut wgpu::CommandEncoder,
        normal_vectors: &[VoxelMeshVertexNormalVector],
        label: Cow<'static, str>,
    ) -> GPUBuffer {
        let total_capacity = Self::add_spare_buffer_capacity(normal_vectors.len());

        // The normal vector is bound as a storage buffer for the geometry pass
        GPUBuffer::new_storage_buffer_with_spare_capacity_and_encoded_initialization(
            graphics_device,
            staging_belt,
            command_encoder,
            total_capacity,
            normal_vectors,
            label,
        )
    }

    fn create_index_material_buffer(
        graphics_device: &GraphicsDevice,
        staging_belt: &mut wgpu::util::StagingBelt,
        command_encoder: &mut wgpu::CommandEncoder,
        index_materials: &[VoxelMeshIndexMaterials],
        label: Cow<'static, str>,
    ) -> GPUBuffer {
        let total_capacity = Self::add_spare_buffer_capacity(index_materials.len());

        // The the index material buffer is bound as a vertex buffer for the
        // geometry pass
        new_vertex_gpu_buffer_with_spare_capacity_and_encoded_initialization(
            graphics_device,
            staging_belt,
            command_encoder,
            total_capacity,
            index_materials,
            label,
        )
    }

    fn create_index_buffer(
        graphics_device: &GraphicsDevice,
        staging_belt: &mut wgpu::util::StagingBelt,
        command_encoder: &mut wgpu::CommandEncoder,
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

        GPUBuffer::new_with_spare_capacity_and_encoded_initialization(
            graphics_device,
            staging_belt,
            command_encoder,
            buffer_size,
            bytemuck::cast_slice(indices),
            usage,
            label,
        )
    }

    fn create_chunk_submesh_buffer(
        graphics_device: &GraphicsDevice,
        staging_belt: &mut wgpu::util::StagingBelt,
        command_encoder: &mut wgpu::CommandEncoder,
        chunk_submeshes: &[ChunkSubmesh],
        label: Cow<'static, str>,
    ) -> GPUBuffer {
        let total_capacity = Self::add_spare_buffer_capacity(chunk_submeshes.len());

        // The normal vector is bound as a storage buffer for the geometry pass
        GPUBuffer::new_storage_buffer_with_spare_capacity_and_encoded_initialization(
            graphics_device,
            staging_belt,
            command_encoder,
            total_capacity,
            chunk_submeshes,
            label,
        )
    }

    fn create_indirect_argument_buffer(
        graphics_device: &GraphicsDevice,
        staging_belt: &mut wgpu::util::StagingBelt,
        command_encoder: &mut wgpu::CommandEncoder,
        n_chunks: usize,
        label: Cow<'static, str>,
    ) -> GPUBuffer {
        let total_capacity = Self::add_spare_buffer_capacity(n_chunks);

        // Used for non-indexed indirect draw calls
        GPUBuffer::new_draw_indirect_buffer_with_spare_capacity_and_encoded_initialization(
            graphics_device,
            staging_belt,
            command_encoder,
            total_capacity,
            &vec![DrawIndirectArgs::default(); n_chunks],
            label,
        )
    }

    fn create_indexed_indirect_argument_buffer(
        graphics_device: &GraphicsDevice,
        staging_belt: &mut wgpu::util::StagingBelt,
        command_encoder: &mut wgpu::CommandEncoder,
        n_chunks: usize,
        label: Cow<'static, str>,
    ) -> GPUBuffer {
        let total_capacity = Self::add_spare_buffer_capacity(n_chunks);

        // Used for indexed indirect draw calls
        GPUBuffer::new_draw_indexed_indirect_buffer_with_spare_capacity_and_encoded_initialization(
            graphics_device,
            staging_belt,
            command_encoder,
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
        staging_belt: &mut wgpu::util::StagingBelt,
        command_encoder: &mut wgpu::CommandEncoder,
        buffer: &GPUBuffer,
        all_values: &[T],
        range: Range<usize>,
    ) {
        let byte_offset = mem::size_of::<T>().checked_mul(range.start).unwrap();

        buffer.encode_update_of_bytes_from_offset(
            graphics_device,
            staging_belt,
            command_encoder,
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
