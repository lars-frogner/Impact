//! Synchronization of GPU buffers with geometrical data.

use impact_camera::buffer::CameraGPUBufferManager;
use impact_containers::HashMap;
use impact_light::buffer::LightGPUBufferManager;
use impact_mesh::{MeshID, buffer::MeshGPUBufferManager};
use impact_model::{InstanceFeature, buffer::InstanceFeatureGPUBufferManager};
use impact_scene::{model::ModelID, skybox::resource::SkyboxGPUResourceManager};

pub trait BasicRenderResources {
    /// Returns the GPU buffer manager for camera data, or [`None`] if it has
    /// not been created.
    fn get_camera_buffer_manager(&self) -> Option<&CameraGPUBufferManager>;

    /// Returns the GPU buffer manager for light data, or [`None`] if it has
    /// not been created.
    fn get_light_buffer_manager(&self) -> Option<&LightGPUBufferManager>;

    /// Returns the GPU resource manager for skybox data, or [`None`] if it has
    /// not been created.
    fn get_skybox_resource_manager(&self) -> Option<&SkyboxGPUResourceManager>;

    /// Returns the GPU buffer manager for the given triangle mesh identifier if
    /// the triangle mesh exists, otherwise returns [`None`].
    fn get_triangle_mesh_buffer_manager(&self, mesh_id: MeshID) -> Option<&MeshGPUBufferManager>;

    /// Returns the GPU buffer manager for the given line segment mesh
    /// identifier if the line segment mesh exists, otherwise returns [`None`].
    fn get_line_segment_mesh_buffer_manager(
        &self,
        mesh_id: MeshID,
    ) -> Option<&MeshGPUBufferManager>;

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
