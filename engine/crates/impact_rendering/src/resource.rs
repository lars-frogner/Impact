//! Synchronization of GPU buffers with geometrical data.

use impact_camera::buffer::CameraGPUBufferManager;
use impact_containers::HashMap;
use impact_light::buffer::LightGPUBufferManager;
use impact_material::{
    MaterialRegistry, MaterialTemplateRegistry, MaterialTextureGroupRegistry,
    gpu_resource::{MaterialTemplateBindGroupLayoutMap, MaterialTextureBindGroupMap},
};
use impact_mesh::{
    LineSegmentMeshRegistry, TriangleMeshRegistry,
    gpu_resource::{LineSegmentMeshGPUResourceMap, TriangleMeshGPUResourceMap},
};
use impact_model::{InstanceFeature, buffer::InstanceFeatureGPUBufferManager};
use impact_scene::{model::ModelID, skybox::resource::SkyboxGPUResourceManager};
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
    /// Returns the GPU buffer manager for camera data, or [`None`] if it has
    /// not been created.
    fn get_camera_buffer_manager(&self) -> Option<&CameraGPUBufferManager>;

    /// Returns the GPU buffer manager for light data, or [`None`] if it has
    /// not been created.
    fn get_light_buffer_manager(&self) -> Option<&LightGPUBufferManager>;

    /// Returns the GPU resource manager for skybox data, or [`None`] if it has
    /// not been created.
    fn get_skybox_resource_manager(&self) -> Option<&SkyboxGPUResourceManager>;

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

    /// Returns the map of material template bind group layouts.
    fn material_template_bind_group_layout(&self) -> &MaterialTemplateBindGroupLayoutMap;

    /// Returns the map of material texture bind groups.
    fn material_texture_bind_group(&self) -> &MaterialTextureBindGroupMap;

    /// Returns a reference to the map of instance feature GPU buffer managers.
    fn instance_feature_buffer_managers(
        &self,
    ) -> &HashMap<ModelID, Vec<InstanceFeatureGPUBufferManager>>;

    /// Returns the instance feature GPU buffer managers for the given model
    /// identifier if the model exists, otherwise returns [`None`].
    fn get_instance_feature_buffer_managers(
        &self,
        model_id: &ModelID,
    ) -> Option<&[InstanceFeatureGPUBufferManager]> {
        self.instance_feature_buffer_managers()
            .get(model_id)
            .map(|managers| managers.as_slice())
    }

    /// Returns the instance feature GPU buffer manager for features of type
    /// `Fe` for the given model if it exists, otherwise returns [`None`].
    fn get_instance_feature_buffer_manager_for_feature_type<Fe: InstanceFeature>(
        &self,
        model_id: &ModelID,
    ) -> Option<&InstanceFeatureGPUBufferManager> {
        self.get_instance_feature_buffer_managers(model_id)
            .and_then(|buffers| {
                buffers
                    .iter()
                    .find(|buffer| buffer.is_for_feature_type::<Fe>())
            })
    }
}
