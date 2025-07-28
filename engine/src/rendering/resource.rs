//! GPU resource management.

pub mod legacy;

use impact_camera::buffer::CameraGPUBufferManager;
use impact_containers::HashMap;
use impact_light::buffer::LightGPUBufferManager;
use impact_mesh::gpu_resource::{LineSegmentMeshGPUResourceMap, TriangleMeshGPUResourceMap};
use impact_model::buffer::InstanceFeatureGPUBufferManager;
use impact_rendering::resource::BasicGPUResources;
use impact_scene::{model::ModelID, skybox::resource::SkyboxGPUResourceManager};
use impact_voxel::{
    VoxelObjectID,
    resource::{VoxelGPUResources, VoxelMaterialGPUResourceManager, VoxelObjectGPUBufferManager},
};

#[derive(Debug)]
pub struct RenderResourceManager {
    pub triangle_meshes: TriangleMeshGPUResourceMap,
    pub line_segment_meshes: LineSegmentMeshGPUResourceMap,
    pub legacy: legacy::RenderResourceManager,
}

impl RenderResourceManager {
    pub fn new() -> Self {
        Self {
            triangle_meshes: TriangleMeshGPUResourceMap::new(),
            line_segment_meshes: LineSegmentMeshGPUResourceMap::new(),
            legacy: legacy::RenderResourceManager::new(),
        }
    }
}

impl Default for RenderResourceManager {
    fn default() -> Self {
        Self::new()
    }
}

impl BasicGPUResources for RenderResourceManager {
    fn triangle_mesh(&self) -> &TriangleMeshGPUResourceMap {
        &self.triangle_meshes
    }

    fn line_segment_mesh(&self) -> &LineSegmentMeshGPUResourceMap {
        &self.line_segment_meshes
    }

    fn get_camera_buffer_manager(&self) -> Option<&CameraGPUBufferManager> {
        self.legacy.synchronized().get_camera_buffer_manager()
    }

    fn get_light_buffer_manager(&self) -> Option<&LightGPUBufferManager> {
        self.legacy.synchronized().get_light_buffer_manager()
    }

    fn get_skybox_resource_manager(&self) -> Option<&SkyboxGPUResourceManager> {
        self.legacy.synchronized().get_skybox_resource_manager()
    }

    fn instance_feature_buffer_managers(
        &self,
    ) -> &HashMap<ModelID, Vec<InstanceFeatureGPUBufferManager>> {
        self.legacy
            .synchronized()
            .instance_feature_buffer_managers()
    }
}

impl VoxelGPUResources for RenderResourceManager {
    fn get_voxel_material_resource_manager(&self) -> Option<&VoxelMaterialGPUResourceManager> {
        self.legacy
            .synchronized()
            .get_voxel_material_resource_manager()
    }

    fn voxel_object_buffer_managers(&self) -> &HashMap<VoxelObjectID, VoxelObjectGPUBufferManager> {
        self.legacy.synchronized().voxel_object_buffer_managers()
    }
}
