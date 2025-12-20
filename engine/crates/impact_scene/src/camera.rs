//! Cameras in a scene.

use crate::graph::CameraNodeID;
use impact_camera::{
    Camera,
    gpu_resource::{BufferableCamera, CameraGPUResource},
};
use impact_gpu::{bind_group_layout::BindGroupLayoutRegistry, device::GraphicsDevice, wgpu};
use impact_math::transform::Isometry3;
use nalgebra::Point3;

/// Manager for the cameras in a scene.
#[derive(Debug)]
pub struct CameraManager {
    active_camera: Option<SceneCamera>,
    context: CameraContext,
}

/// Camera-external context required for creating a camera.
#[derive(Clone, Debug)]
pub struct CameraContext {
    pub aspect_ratio: f32,
    pub jitter_enabled: bool,
}

/// Represents a [`Camera`] that has a camera node in a
/// [`SceneGraph`](crate::graph::SceneGraph).
#[derive(Debug)]
pub struct SceneCamera {
    camera: Box<dyn Camera>,
    view_transform: Isometry3,
    scene_graph_node_id: CameraNodeID,
    jitter_enabled: bool,
}

impl CameraManager {
    /// Creates a new camera manager with no active camera.
    pub fn new(context: CameraContext) -> Self {
        Self {
            active_camera: None,
            context,
        }
    }

    /// Whether there is an active camera.
    pub fn has_active_camera(&self) -> bool {
        self.active_camera.is_some()
    }

    pub fn active_camera(&self) -> Option<&SceneCamera> {
        self.active_camera.as_ref()
    }

    pub fn active_camera_mut(&mut self) -> Option<&mut SceneCamera> {
        self.active_camera.as_mut()
    }

    pub fn camera_context(&self) -> &CameraContext {
        &self.context
    }

    /// Returns the view transform of the active camera, or the identity
    /// transform if there is no active camera.
    pub fn active_view_transform(&self) -> Isometry3 {
        self.active_camera()
            .map(SceneCamera::view_transform)
            .copied()
            .unwrap_or_default()
    }

    /// Whether the active camera has the camera node with the give ID in the
    /// scene graph.
    pub fn active_camera_has_node(&self, scene_graph_node_id: CameraNodeID) -> bool {
        self.active_camera()
            .is_some_and(|camera| camera.scene_graph_node_id() == scene_graph_node_id)
    }

    /// Creates a [`SceneCamera`] for the given camera and camera node and sets
    /// it as the active camera.
    pub fn set_active_camera(&mut self, camera: impl Camera, scene_graph_node_id: CameraNodeID) {
        self.active_camera = Some(SceneCamera::new(
            camera,
            scene_graph_node_id,
            self.context.jitter_enabled,
        ));
    }

    /// Makes no camera active.
    pub fn clear_active_camera(&mut self) {
        self.active_camera.take();
    }

    /// Sets the ratio of width to height of the camera's view plane.
    ///
    /// # Panics
    /// If `aspect_ratio` is zero.
    pub fn set_aspect_ratio(&mut self, aspect_ratio: f32) {
        if let Some(camera) = &mut self.active_camera {
            camera.set_aspect_ratio(aspect_ratio);
        }
        self.context.aspect_ratio = aspect_ratio;
    }

    /// Sets whether jittering is enabled for the cameras.
    pub fn set_jitter_enabled(&mut self, jitter_enabled: bool) {
        if let Some(camera) = &mut self.active_camera {
            camera.set_jitter_enabled(jitter_enabled);
        }
        self.context.jitter_enabled = jitter_enabled;
    }

    /// Performs any required updates for keeping the camera GPU resources in
    /// sync with the camera manager.
    pub fn sync_gpu_resources(
        &self,
        graphics_device: &GraphicsDevice,
        staging_belt: &mut wgpu::util::StagingBelt,
        command_encoder: &mut wgpu::CommandEncoder,
        bind_group_layout_registry: &BindGroupLayoutRegistry,
        camera_gpu_resources: &mut Option<CameraGPUResource>,
    ) {
        if let Some(scene_camera) = self.active_camera() {
            if let Some(camera_gpu_resources) = camera_gpu_resources {
                camera_gpu_resources.sync_with_camera(
                    graphics_device,
                    staging_belt,
                    command_encoder,
                    scene_camera,
                );
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
}

impl SceneCamera {
    /// Creates a new [`SceneCamera`] representing the given [`Camera`] in the
    /// camera node with the given ID in the
    /// [`SceneGraph`](crate::graph::SceneGraph).
    pub fn new(
        camera: impl Camera,
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

    /// Returns a reference to the underlying [`Camera`].
    pub fn camera(&self) -> &dyn Camera {
        self.camera.as_ref()
    }

    /// Returns a reference to the camera's view transform.
    pub fn view_transform(&self) -> &Isometry3 {
        &self.view_transform
    }

    /// Returns whether jittering is enabled for the camera.
    pub fn jitter_enabled(&self) -> bool {
        self.jitter_enabled
    }

    /// Returns the ID of the [`CameraNode`](crate::graph::CameraNode)
    /// for the camera in the [`SceneGraph`](crate::graph::SceneGraph).
    pub fn scene_graph_node_id(&self) -> CameraNodeID {
        self.scene_graph_node_id
    }

    /// Computes the world-space position of the camera based on the current
    /// view transform.
    pub fn compute_world_space_position(&self) -> Point3<f32> {
        let camera_to_world = self.view_transform.inverted();
        Point3::from(*camera_to_world.translation())
    }

    /// Sets the transform from world space to camera space.
    pub fn set_view_transform(&mut self, view_transform: Isometry3) {
        self.view_transform = view_transform;
    }

    fn set_aspect_ratio(&mut self, aspect_ratio: f32) {
        self.camera.set_aspect_ratio(aspect_ratio);
    }

    fn set_jitter_enabled(&mut self, jitter_enabled: bool) {
        self.jitter_enabled = jitter_enabled;
    }
}

impl BufferableCamera for SceneCamera {
    fn camera(&self) -> &dyn Camera {
        self.camera()
    }

    fn view_transform(&self) -> &Isometry3 {
        self.view_transform()
    }

    fn jitter_enabled(&self) -> bool {
        self.jitter_enabled()
    }
}
