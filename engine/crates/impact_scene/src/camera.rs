//! Cameras in a scene.

use anyhow::{Result, anyhow};
use impact_camera::{
    Camera, CameraID,
    gpu_resource::{BufferableCamera, CameraGPUResource},
};
use impact_containers::HashMap;
use impact_gpu::{bind_group_layout::BindGroupLayoutRegistry, device::GraphicsDevice, wgpu};
use impact_math::{point::Point3, transform::Isometry3};

/// Manager for the cameras in a scene.
#[derive(Debug)]
pub struct CameraManager {
    inactive_cameras: HashMap<CameraID, SceneCamera>,
    active_camera: Option<SceneCamera>,
    context: CameraContext,
    active_camera_version: u64,
}

/// Camera-external context required for creating a camera.
#[derive(Clone, Debug)]
pub struct CameraContext {
    pub aspect_ratio: f32,
    pub jitter_enabled: bool,
}

/// Represents a [`Camera`] in a scene.
#[derive(Debug)]
pub struct SceneCamera {
    id: CameraID,
    camera: Box<dyn Camera>,
    view_transform: Isometry3,
    jitter_enabled: bool,
}

impl CameraManager {
    /// Creates a new camera manager with no active camera.
    pub fn new(context: CameraContext) -> Self {
        Self {
            inactive_cameras: HashMap::default(),
            active_camera: None,
            context,
            active_camera_version: 0,
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

    /// Whether the active camera has the given ID.
    pub fn active_camera_has_id(&self, camera_id: CameraID) -> bool {
        self.active_camera()
            .is_some_and(|camera| camera.id() == camera_id)
    }

    /// Adds the given camera to the manager and sets it as active.
    pub fn add_active_camera(&mut self, camera: impl Camera, camera_id: CameraID) {
        self.clear_active_camera();

        self.active_camera = Some(SceneCamera::new(
            camera_id,
            camera,
            self.context.jitter_enabled,
        ));
        self.active_camera_version = self.active_camera_version.wrapping_add(1);
    }

    /// Sets the given camera as active.
    ///
    /// # Errors
    /// Returns an error if the camera is not present.
    pub fn set_active_camera(&mut self, camera_id: CameraID) -> Result<()> {
        self.clear_active_camera();

        self.active_camera = Some(self.inactive_cameras.remove(&camera_id).ok_or_else(|| {
            anyhow!(
                "Tried to set missing camera with ID {:?} as active",
                camera_id
            )
        })?);
        self.active_camera_version = self.active_camera_version.wrapping_add(1);
        Ok(())
    }

    /// Makes no camera active.
    pub fn clear_active_camera(&mut self) {
        if let Some(camera) = self.active_camera.take() {
            self.inactive_cameras.insert(camera.id(), camera);
        }
    }

    /// Removes all cameras.
    pub fn remove_all_cameras(&mut self) {
        self.clear_active_camera();
        self.inactive_cameras.clear();
    }

    /// Sets the ratio of width to height of the camera's view plane.
    ///
    /// # Panics
    /// If `aspect_ratio` is zero.
    pub fn set_aspect_ratio(&mut self, aspect_ratio: f32) {
        if let Some(camera) = &mut self.active_camera {
            camera.set_aspect_ratio(aspect_ratio);
        }
        for camera in self.inactive_cameras.values_mut() {
            camera.set_aspect_ratio(aspect_ratio);
        }
        self.context.aspect_ratio = aspect_ratio;
    }

    /// Sets whether jittering is enabled for the cameras.
    pub fn set_jitter_enabled(&mut self, jitter_enabled: bool) {
        if let Some(camera) = &mut self.active_camera {
            camera.set_jitter_enabled(jitter_enabled);
        }
        for camera in self.inactive_cameras.values_mut() {
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
                camera_gpu_resources.sync_with_camera_manager(
                    graphics_device,
                    staging_belt,
                    command_encoder,
                    scene_camera,
                    self.active_camera_version,
                );
            } else {
                *camera_gpu_resources = Some(CameraGPUResource::for_camera(
                    graphics_device,
                    bind_group_layout_registry,
                    scene_camera,
                    self.active_camera_version,
                ));
            }
        } else {
            camera_gpu_resources.take();
        }
    }
}

impl SceneCamera {
    /// Creates a new [`SceneCamera`] with the given ID representing the given
    /// [`Camera`].
    pub fn new(id: CameraID, camera: impl Camera, jitter_enabled: bool) -> Self {
        Self {
            id,
            camera: Box::new(camera),
            view_transform: Isometry3::identity(),
            jitter_enabled,
        }
    }

    /// Returns the ID of the camera.
    pub fn id(&self) -> CameraID {
        self.id
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

    /// Computes the world-space position of the camera based on the current
    /// view transform.
    pub fn compute_world_space_position(&self) -> Point3 {
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
