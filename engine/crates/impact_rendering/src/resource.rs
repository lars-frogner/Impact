//! Synchronization of GPU buffers with geometrical data.

use impact_camera::gpu_resource::CameraGPUResource;
use impact_light::gpu_resource::LightGPUResources;
use impact_material::{
    MaterialRegistry, MaterialTemplateRegistry, MaterialTextureGroupRegistry,
    gpu_resource::{GPUMaterialMap, GPUMaterialTemplateMap, GPUMaterialTextureGroupMap},
};
use impact_mesh::{
    LineSegmentMeshRegistry, TriangleMeshRegistry,
    gpu_resource::{LineSegmentMeshGPUResourceMap, TriangleMeshGPUResourceMap},
};
use impact_scene::{model::ModelInstanceGPUBufferMap, skybox::gpu_resource::SkyboxGPUResource};
use impact_texture::{
    SamplerRegistry, TextureRegistry,
    gpu_resource::{LookupTableBindGroupMap, SamplerMap, TextureMap},
    lookup_table::LookupTableRegistry,
};

pub trait BasicResourceRegistries {
    fn triangle_mesh(&self) -> &TriangleMeshRegistry;

    fn line_segment_mesh(&self) -> &LineSegmentMeshRegistry;

    fn texture(&self) -> &TextureRegistry;

    fn sampler(&self) -> &SamplerRegistry;

    fn lookup_table(&self) -> &LookupTableRegistry;

    fn material(&self) -> &MaterialRegistry;

    fn material_template(&self) -> &MaterialTemplateRegistry;

    fn material_texture_group(&self) -> &MaterialTextureGroupRegistry;
}

pub trait BasicGPUResources {
    /// Returns the GPU resource for the camera, or [`None`] if it does not
    /// exist.
    fn camera(&self) -> Option<&CameraGPUResource>;

    /// Returns the GPU resource for the skybox, or [`None`] if it does not
    /// exist.
    fn skybox(&self) -> Option<&SkyboxGPUResource>;

    /// Returns the GPU resources for light data, or [`None`] if it has not been
    /// created.
    fn light(&self) -> Option<&LightGPUResources>;

    /// Returns the GPU resource map for triangle mesh data.
    fn triangle_mesh(&self) -> &TriangleMeshGPUResourceMap;

    /// Returns the GPU resource map for line segment mesh data.
    fn line_segment_mesh(&self) -> &LineSegmentMeshGPUResourceMap;

    /// Returns the map of textures.
    fn texture(&self) -> &TextureMap;

    /// Returns the map of texture samplers.
    fn sampler(&self) -> &SamplerMap;

    /// Returns the map of lookup table bind groups.
    fn lookup_table_bind_group(&self) -> &LookupTableBindGroupMap;

    /// Returns the map of materials.
    fn material(&self) -> &GPUMaterialMap;

    /// Returns the map of material templates.
    fn material_template(&self) -> &GPUMaterialTemplateMap;

    /// Returns the map of material texture groups.
    fn material_texture_group(&self) -> &GPUMaterialTextureGroupMap;

    /// Returns the map of model instance GPU buffers.
    fn model_instance_buffer(&self) -> &ModelInstanceGPUBufferMap;
}
