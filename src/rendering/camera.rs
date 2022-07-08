//! Management of camera data for rendering.

use crate::geometry::Camera;
use crate::rendering::{
    buffer::{BufferableUniform, UniformBuffer},
    CoreRenderingSystem,
};
use bytemuck::{Pod, Zeroable};
use nalgebra::{Matrix4, Projective3};

/// Owner and manager of render data for cameras.
pub struct CameraRenderDataManager {
    transform_buffer: UniformBuffer,
    transform_bind_group_layout: wgpu::BindGroupLayout,
    transform_bind_group: wgpu::BindGroup,
    view_projection_transform_changed: bool,
}

/// Representation of a view projection transform
/// as a nested slice of matrix elements.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
pub struct RawViewProjectionTransform {
    view_projection_matrix: [[f32; 4]; 4],
}

impl CameraRenderDataManager {
    /// Creates a new manager with render data initialized
    /// from the view projection transform of the given camera.
    pub fn for_camera(
        core_system: &CoreRenderingSystem,
        camera: &impl Camera<f32>,
        label: &str,
    ) -> Self {
        let view_projection_transform = camera.compute_view_projection_transform();
        Self::new(core_system, &view_projection_transform, label)
    }

    /// Ensures that the render data is in sync with the view
    /// projection transform of the given camera.
    pub fn sync_with_camera(
        &mut self,
        core_system: &CoreRenderingSystem,
        camera: &mut impl Camera<f32>,
    ) {
        self.view_projection_transform_changed = camera.view_projection_transform_changed();
        if self.view_projection_transform_changed {
            let view_projection_transform = camera.compute_view_projection_transform();
            self.sync_render_data(core_system, &view_projection_transform);
            camera.reset_view_projection_change_tracking();
        }
    }

    /// Whether the view projection transform was updated
    /// at the latest sync.
    pub fn view_projection_transform_changed(&self) -> bool {
        self.view_projection_transform_changed
    }

    /// Returns the layout of the bind group to which the
    /// camera transform uniform bufffer is bound.
    ///
    /// The layout will remain valid even though the transform
    /// may change.
    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.transform_bind_group_layout
    }

    /// Returns the bind group to which the camera transform
    /// uniform bufffer is bound.
    ///
    /// The bind group will remain valid even though the transform
    /// may change.
    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.transform_bind_group
    }

    /// Creates a new manager with render data initialized
    /// from the given view projection transform.
    fn new(
        core_system: &CoreRenderingSystem,
        view_projection_transform: &Projective3<f32>,
        label: &str,
    ) -> Self {
        let raw_transform = RawViewProjectionTransform::from_transform(view_projection_transform);
        let transform_buffer = UniformBuffer::new(core_system, &[raw_transform], label);

        let (transform_bind_group, transform_bind_group_layout) =
            transform_buffer.create_bind_group_and_layout(core_system.device());

        Self {
            transform_buffer,
            transform_bind_group_layout,
            transform_bind_group,
            view_projection_transform_changed: false,
        }
    }

    fn sync_render_data(
        &mut self,
        core_system: &CoreRenderingSystem,
        view_projection_transform: &Projective3<f32>,
    ) {
        let raw_transform = RawViewProjectionTransform::from_transform(view_projection_transform);
        self.transform_buffer
            .queue_update_of_uniforms(core_system, 0, &[raw_transform]);
    }
}

impl RawViewProjectionTransform {
    /// Creates a new raw transform matrix from the given `Projective3`.
    fn from_transform(view_projection_transform: &Projective3<f32>) -> Self {
        Self::from_matrix(view_projection_transform.matrix())
    }

    /// Creates a new raw transform matrix from the given `Matrix4`.
    fn from_matrix(view_projection_matrix: &Matrix4<f32>) -> Self {
        Self::new(*view_projection_matrix.as_ref())
    }

    fn new(view_projection_matrix: [[f32; 4]; 4]) -> Self {
        Self {
            view_projection_matrix,
        }
    }
}

impl BufferableUniform for RawViewProjectionTransform {
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
