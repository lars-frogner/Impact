//! The active camera for a scene.

use crate::graph::CameraNodeID;
use impact_camera::{
    Camera,
    gpu_resource::{BufferableCamera, CameraGPUResource},
};
use impact_gpu::{bind_group_layout::BindGroupLayoutRegistry, device::GraphicsDevice};
use nalgebra::{Isometry3, Point3};

/// Represents a [`Camera`] that has a camera node in a [`SceneGraph`](crate::graph::SceneGraph).
#[derive(Debug)]
pub struct SceneCamera {
    camera: Box<dyn Camera<f32>>,
    view_transform: Isometry3<f32>,
    scene_graph_node_id: CameraNodeID,
    jitter_enabled: bool,
}

impl SceneCamera {
    /// Creates a new [`SceneCamera`] representing the given [`Camera`] in the
    /// camera node with the given ID in the [`SceneGraph`](crate::graph::SceneGraph).
    pub fn new(
        camera: impl Camera<f32>,
        scene_graph_node_id: CameraNodeID,
        jitter_enabled: bool,
    ) -> Self {
        Self {
            camera: Box::new(camera),
            view_transform: Isometry3::identity(),
            scene_graph_node_id,
            jitter_enabled,
        }
    }

    /// Returns the ID of the [`CameraNode`](crate::graph::CameraNode)
    /// for the camera in the [`SceneGraph`](crate::graph::SceneGraph).
    pub fn scene_graph_node_id(&self) -> CameraNodeID {
        self.scene_graph_node_id
    }

    /// Computes the world-space position of the camera based on the current
    /// view transform.
    pub fn compute_world_space_position(&self) -> Point3<f32> {
        let camera_to_world = self.view_transform.inverse();
        camera_to_world.translation.vector.into()
    }

    /// Sets the transform from world space to camera space.
    pub fn set_view_transform(&mut self, view_transform: Isometry3<f32>) {
        self.view_transform = view_transform;
    }

    /// Sets the ratio of width to height of the camera's view plane.
    ///
    /// # Panics
    /// If `aspect_ratio` is zero.
    pub fn set_aspect_ratio(&mut self, aspect_ratio: f32) {
        self.camera.set_aspect_ratio(aspect_ratio);
    }

    /// Sets whether jittering is enabled for the camera.
    pub fn set_jitter_enabled(&mut self, jitter_enabled: bool) {
        self.jitter_enabled = jitter_enabled;
    }
}

impl BufferableCamera for SceneCamera {
    fn camera(&self) -> &dyn Camera<f32> {
        self.camera.as_ref()
    }

    fn view_transform(&self) -> &Isometry3<f32> {
        &self.view_transform
    }

    fn jitter_enabled(&self) -> bool {
        self.jitter_enabled
    }
}

/// Performs any required updates for keeping the camera GPU resources in sync
/// with the given scene camera.
pub fn sync_gpu_resources_for_scene_camera(
    scene_camera: Option<&SceneCamera>,
    graphics_device: &GraphicsDevice,
    bind_group_layout_registry: &BindGroupLayoutRegistry,
    camera_gpu_resources: &mut Option<CameraGPUResource>,
) {
    if let Some(scene_camera) = scene_camera {
        if let Some(camera_gpu_resources) = camera_gpu_resources {
            camera_gpu_resources.sync_with_camera(graphics_device, scene_camera);
        } else {
            *camera_gpu_resources = Some(CameraGPUResource::for_camera(
                graphics_device,
                bind_group_layout_registry,
                scene_camera,
            ));
        }
    } else {
        camera_gpu_resources.take();
    }
}
