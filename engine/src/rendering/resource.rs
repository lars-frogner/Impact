//! GPU resource management.

use impact_camera::gpu_resource::CameraGPUResource;
use impact_light::gpu_resource::LightGPUResources;
use impact_material::gpu_resource::{
    MaterialTemplateBindGroupLayoutMap, MaterialTextureBindGroupMap,
};
use impact_mesh::gpu_resource::{LineSegmentMeshGPUResourceMap, TriangleMeshGPUResourceMap};
use impact_rendering::resource::BasicGPUResources;
use impact_scene::{model::ModelInstanceGPUBufferMap, skybox::gpu_resource::SkyboxGPUResource};
use impact_texture::gpu_resource::{LookupTableBindGroupMap, SamplerMap, TextureMap};
use impact_voxel::gpu_resource::{
    VoxelGPUResources, VoxelMaterialGPUResources, VoxelObjectGPUBufferMap,
};

#[derive(Debug)]
pub struct RenderResourceManager {
    pub camera: Option<CameraGPUResource>,
    pub skybox: Option<SkyboxGPUResource>,
    pub lights: Option<LightGPUResources>,
    pub triangle_meshes: TriangleMeshGPUResourceMap,
    pub line_segment_meshes: LineSegmentMeshGPUResourceMap,
    pub textures: TextureMap,
    pub samplers: SamplerMap,
    pub lookup_table_bind_groups: LookupTableBindGroupMap,
    pub material_template_bind_group_layouts: MaterialTemplateBindGroupLayoutMap,
    pub material_texture_bind_groups: MaterialTextureBindGroupMap,
    pub model_instance_buffers: ModelInstanceGPUBufferMap,
    pub voxel_materials: Option<VoxelMaterialGPUResources>,
    pub voxel_object_buffers: VoxelObjectGPUBufferMap,
}

impl RenderResourceManager {
    pub fn new() -> Self {
        Self {
            camera: None,
            skybox: None,
            lights: None,
            triangle_meshes: TriangleMeshGPUResourceMap::new(),
            line_segment_meshes: LineSegmentMeshGPUResourceMap::new(),
            textures: TextureMap::new(),
            samplers: SamplerMap::new(),
            lookup_table_bind_groups: LookupTableBindGroupMap::new(),
            material_template_bind_group_layouts: MaterialTemplateBindGroupLayoutMap::new(),
            material_texture_bind_groups: MaterialTextureBindGroupMap::new(),
            model_instance_buffers: ModelInstanceGPUBufferMap::new(),
            voxel_materials: None,
            voxel_object_buffers: VoxelObjectGPUBufferMap::new(),
        }
    }
}

impl Default for RenderResourceManager {
    fn default() -> Self {
        Self::new()
    }
}

impl BasicGPUResources for RenderResourceManager {
    fn camera(&self) -> Option<&CameraGPUResource> {
        self.camera.as_ref()
    }

    fn skybox(&self) -> Option<&SkyboxGPUResource> {
        self.skybox.as_ref()
    }

    fn light(&self) -> Option<&LightGPUResources> {
        self.lights.as_ref()
    }

    fn triangle_mesh(&self) -> &TriangleMeshGPUResourceMap {
        &self.triangle_meshes
    }

    fn line_segment_mesh(&self) -> &LineSegmentMeshGPUResourceMap {
        &self.line_segment_meshes
    }

    fn texture(&self) -> &TextureMap {
        &self.textures
    }

    fn sampler(&self) -> &SamplerMap {
        &self.samplers
    }

    fn lookup_table_bind_group(&self) -> &LookupTableBindGroupMap {
        &self.lookup_table_bind_groups
    }

    fn material_template_bind_group_layout(&self) -> &MaterialTemplateBindGroupLayoutMap {
        &self.material_template_bind_group_layouts
    }

    fn material_texture_bind_group(&self) -> &MaterialTextureBindGroupMap {
        &self.material_texture_bind_groups
    }

    fn model_instance_buffer(&self) -> &ModelInstanceGPUBufferMap {
        &self.model_instance_buffers
    }
}

impl VoxelGPUResources for RenderResourceManager {
    fn voxel_materials(&self) -> Option<&VoxelMaterialGPUResources> {
        self.voxel_materials.as_ref()
    }

    fn voxel_object_buffer(&self) -> &VoxelObjectGPUBufferMap {
        &self.voxel_object_buffers
    }
}
