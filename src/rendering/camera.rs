//! Management of camera data for rendering.

use crate::geometry::Camera;
use crate::rendering::{
    buffer::{BufferableUniform, UniformRenderBuffer},
    fre, CoreRenderingSystem,
};
use nalgebra::Projective3;

/// Owner and manager of a render buffer for a camera
/// transformation.
#[derive(Debug)]
pub struct CameraRenderBufferManager {
    transform_buffer: UniformRenderBuffer,
}

impl CameraRenderBufferManager {
    /// Creates a new manager with a render buffer initialized
    /// from the view projection transform of the given camera.
    pub fn for_camera(
        core_system: &CoreRenderingSystem,
        camera: &impl Camera<fre>,
        label: &str,
    ) -> Self {
        let view_projection_transform = camera.compute_view_projection_transform();
        Self::new(core_system, view_projection_transform, label)
    }

    /// Ensures that the render buffer is in sync with the view
    /// projection transform of the given camera.
    pub fn sync_with_camera(
        &mut self,
        core_system: &CoreRenderingSystem,
        camera: &impl Camera<fre>,
    ) {
        if camera.view_projection_transform_changed() {
            let view_projection_transform = camera.compute_view_projection_transform();
            self.sync_render_buffer(core_system, view_projection_transform);
            camera.reset_view_projection_change_tracking();
        }
    }

    /// Returns the layout of the bind group to which the
    /// camera transform uniform bufffer is bound.
    ///
    /// The layout will remain valid even though the transform
    /// may change.
    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        self.transform_buffer.bind_group_layout()
    }

    /// Returns the bind group to which the camera transform
    /// uniform bufffer is bound.
    ///
    /// The bind group will remain valid even though the transform
    /// may change.
    pub fn bind_group(&self) -> &wgpu::BindGroup {
        self.transform_buffer.bind_group()
    }

    /// Creates a new manager with a render buffer initialized
    /// from the given view projection transform.
    fn new(
        core_system: &CoreRenderingSystem,
        view_projection_transform: Projective3<fre>,
        label: &str,
    ) -> Self {
        let transform_buffer =
            UniformRenderBuffer::new(core_system, &[view_projection_transform], label);
        Self { transform_buffer }
    }

    fn sync_render_buffer(
        &mut self,
        core_system: &CoreRenderingSystem,
        view_projection_transform: Projective3<fre>,
    ) {
        self.transform_buffer.queue_update_of_uniforms(
            core_system,
            0,
            &[view_projection_transform],
        );
    }
}

impl BufferableUniform for Projective3<fre> {
    const BIND_GROUP_LAYOUT_DESCRIPTOR: wgpu::BindGroupLayoutDescriptor<'static> =
        wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("Camera uniform bind group layout descriptor"),
        };
}
