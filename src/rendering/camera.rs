//! Management of camera data for rendering.

use crate::assert_uniform_valid;
use crate::geometry::Camera;
use crate::rendering::{
    buffer::{self, RenderBuffer, UniformBufferable},
    fre, CameraShaderInput, CoreRenderingSystem,
};
use impact_utils::ConstStringHash64;
use nalgebra::Projective3;
use std::borrow::Cow;

/// Owner and manager of a render buffer for a camera projection transformation.
#[derive(Debug)]
pub struct CameraRenderBufferManager {
    transform_render_buffer: RenderBuffer,
    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
}

impl CameraRenderBufferManager {
    const BINDING: u32 = 0;
    const SHADER_INPUT: CameraShaderInput = CameraShaderInput {
        projection_matrix_binding: Self::BINDING,
    };

    /// Creates a new manager with a render buffer initialized from the
    /// projection transform of the given camera.
    pub fn for_camera(
        core_system: &CoreRenderingSystem,
        camera: &(impl Camera<fre> + ?Sized),
    ) -> Self {
        Self::new(
            core_system,
            *camera.projection_transform(),
            Cow::Borrowed("Camera"),
        )
    }

    /// Returns the layout of the bind group to which the projection transform
    /// uniform bufffer is bound.
    ///
    /// The layout will remain valid even though the transform may change.
    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.bind_group_layout
    }

    /// Returns the bind group to which the projection transform uniform bufffer
    /// is bound.
    ///
    /// The bind group will remain valid even though the transform may change.
    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }

    /// Returns the input required for accessing the projection transform in a
    /// shader.
    pub const fn shader_input() -> &'static CameraShaderInput {
        &Self::SHADER_INPUT
    }

    /// Ensures that the render buffer is in sync with the projection transform
    /// of the given camera.
    pub fn sync_with_camera(
        &mut self,
        core_system: &CoreRenderingSystem,
        camera: &(impl Camera<fre> + ?Sized),
    ) {
        if camera.projection_transform_changed() {
            self.sync_render_buffer(core_system, camera.projection_transform());
            camera.reset_projection_change_tracking();
        }
    }

    /// Creates a new manager with a render buffer initialized
    /// from the given projection transform.
    fn new(
        core_system: &CoreRenderingSystem,
        projection_transform: Projective3<fre>,
        label: Cow<'static, str>,
    ) -> Self {
        let transform_render_buffer = RenderBuffer::new_buffer_for_single_uniform(
            core_system,
            &projection_transform,
            label.clone(),
        );

        let bind_group_layout = Self::create_bind_group_layout(core_system.device(), &label);

        let bind_group = Self::create_bind_group(
            core_system.device(),
            &transform_render_buffer,
            &bind_group_layout,
            &label,
        );

        Self {
            transform_render_buffer,
            bind_group_layout,
            bind_group,
        }
    }

    /// Creates the bind group layout entry for the camera transform
    /// uniform, assigned to the given binding.
    fn create_bind_group_layout_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
        Projective3::create_bind_group_layout_entry(binding)
    }

    fn create_bind_group_layout(device: &wgpu::Device, label: &str) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[Self::create_bind_group_layout_entry(Self::BINDING)],
            label: Some(&format!("{} bind group layout", label)),
        })
    }

    fn create_bind_group(
        device: &wgpu::Device,
        transform_render_buffer: &RenderBuffer,
        layout: &wgpu::BindGroupLayout,
        label: &str,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout,
            entries: &[buffer::create_single_uniform_bind_group_entry(
                Self::BINDING,
                transform_render_buffer,
            )],
            label: Some(&format!("{} bind group", label)),
        })
    }

    fn sync_render_buffer(
        &mut self,
        core_system: &CoreRenderingSystem,
        projection_transform: &Projective3<fre>,
    ) {
        self.transform_render_buffer
            .update_all_bytes(core_system, bytemuck::bytes_of(projection_transform));
    }
}

impl UniformBufferable for Projective3<fre> {
    const ID: ConstStringHash64 = ConstStringHash64::new("Camera projection");

    fn create_bind_group_layout_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
        buffer::create_uniform_buffer_bind_group_layout_entry(
            binding,
            wgpu::ShaderStages::VERTEX_FRAGMENT,
        )
    }
}
assert_uniform_valid!(Projective3<fre>);
