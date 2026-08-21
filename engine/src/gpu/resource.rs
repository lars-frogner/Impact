//! GPU resource management.

use impact_camera::gpu_resource::CameraGPUResource;
use impact_gpu::{device::GraphicsDevice, wgpu};
use impact_light::gpu_resource::LightGPUResources;
use impact_material::gpu_resource::{
    GPUMaterialMap, GPUMaterialTemplateMap, GPUMaterialTextureGroupMap,
};
use impact_mesh::gpu_resource::{LineSegmentMeshGPUResourceMap, TriangleMeshGPUResourceMap};
use impact_particle::gpu_resource::{
    ParticleGPUResources, ParticleMaterialGPUResources, ParticleSimulationGPUResources,
};
use impact_rendering::resource::BasicGPUResources;
use impact_scene::{model::ModelInstanceGPUBufferMap, skybox::gpu_resource::SkyboxGPUResource};
use impact_texture::gpu_resource::{LookupTableBindGroupMap, SamplerMap, TextureMap};
use impact_voxel::gpu_resource::{
    VoxelGPUResources, VoxelMaterialGPUResources, VoxelObjectGPUResources,
};

#[derive(Debug)]
pub struct GPUResourceManager {
    pub camera: Option<CameraGPUResource>,
    pub skybox: Option<SkyboxGPUResource>,
    pub lights: Option<LightGPUResources>,
    pub triangle_meshes: TriangleMeshGPUResourceMap,
    pub line_segment_meshes: LineSegmentMeshGPUResourceMap,
    pub textures: TextureMap,
    pub samplers: SamplerMap,
    pub lookup_table_bind_groups: LookupTableBindGroupMap,
    pub materials: GPUMaterialMap,
    pub material_templates: GPUMaterialTemplateMap,
    pub material_texture_groups: GPUMaterialTextureGroupMap,
    pub model_instance_buffers: ModelInstanceGPUBufferMap,
    pub particle_materials: Option<ParticleMaterialGPUResources>,
    pub particle_simulations: ParticleSimulationGPUResources,
    pub voxel_materials: Option<VoxelMaterialGPUResources>,
    pub voxel_objects: VoxelObjectGPUResources,
    pub staging_belt: wgpu::util::StagingBelt,
}

impl GPUResourceManager {
    pub fn new(graphics_device: &GraphicsDevice) -> Self {
        let staging_belt =
            wgpu::util::StagingBelt::new(graphics_device.device().clone(), 1024 * 1024);

        Self {
            camera: None,
            skybox: None,
            lights: None,
            triangle_meshes: TriangleMeshGPUResourceMap::new(),
            line_segment_meshes: LineSegmentMeshGPUResourceMap::new(),
            textures: TextureMap::new(),
            samplers: SamplerMap::new(),
            lookup_table_bind_groups: LookupTableBindGroupMap::new(),
            materials: GPUMaterialMap::new(),
            material_templates: GPUMaterialTemplateMap::new(),
            material_texture_groups: GPUMaterialTextureGroupMap::new(),
            model_instance_buffers: ModelInstanceGPUBufferMap::new(),
            particle_materials: None,
            particle_simulations: ParticleSimulationGPUResources::new(),
            voxel_materials: None,
            voxel_objects: VoxelObjectGPUResources::new(),
            staging_belt,
        }
    }
}

impl BasicGPUResources for GPUResourceManager {
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

    fn material(&self) -> &GPUMaterialMap {
        &self.materials
    }

    fn material_template(&self) -> &GPUMaterialTemplateMap {
        &self.material_templates
    }

    fn material_texture_group(&self) -> &GPUMaterialTextureGroupMap {
        &self.material_texture_groups
    }

    fn model_instance_buffer(&self) -> &ModelInstanceGPUBufferMap {
        &self.model_instance_buffers
    }
}

impl ParticleGPUResources for GPUResourceManager {
    fn particle_materials(&self) -> Option<&ParticleMaterialGPUResources> {
        self.particle_materials.as_ref()
    }

    fn particle_simulations(&self) -> &ParticleSimulationGPUResources {
        &self.particle_simulations
    }
}

impl VoxelGPUResources for GPUResourceManager {
    fn voxel_materials(&self) -> Option<&VoxelMaterialGPUResources> {
        self.voxel_materials.as_ref()
    }

    fn voxel_objects(&self) -> &VoxelObjectGPUResources {
        &self.voxel_objects
    }
}
