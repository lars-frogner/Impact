//! Buffering of camera data for rendering.

use crate::{
    assert_uniform_valid,
    camera::Camera,
    gpu::{
        buffer::GPUBuffer,
        rendering::fre,
        shader::CameraShaderInput,
        uniform::{self, UniformBufferable},
        GraphicsDevice,
    },
};
use bytemuck::{Pod, Zeroable};
use impact_utils::ConstStringHash64;
use nalgebra::{Projective3, Vector4};
use std::borrow::Cow;

/// Length of the sequence of jitter offsets to apply to the projection for
/// temporal anti-aliasing.
pub const JITTER_COUNT: usize = 8;

/// Bases for the Halton sequence used to generate the jitter offsets.
const JITTER_BASES: (u32, u32) = (2, 3);

/// Owner and manager of a GPU buffer for a camera projection transformation.
#[derive(Debug)]
pub struct CameraGPUBufferManager {
    projection_uniform_gpu_buffer: GPUBuffer,
    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
}

/// Uniform holding the projection transformation of a camera and the sequence
/// of jitter offsets to apply to the projection for temporal anti-aliasing.
///
/// The size of this struct has to be a multiple of 16 bytes as required for
/// uniforms.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
struct CameraProjectionUniform {
    transform: Projective3<fre>,
    jitter_offsets: [Vector4<fre>; JITTER_COUNT],
}

struct HaltonSequenceIter {
    base: u32,
    n: u32,
    d: u32,
}

impl CameraGPUBufferManager {
    const BINDING: u32 = 0;
    const VISIBILITY: wgpu::ShaderStages = wgpu::ShaderStages::VERTEX_FRAGMENT;
    const SHADER_INPUT: CameraShaderInput = CameraShaderInput {
        projection_binding: Self::BINDING,
    };

    /// Creates a new manager with a GPU buffer initialized from the projection
    /// transform of the given camera.
    pub fn for_camera(
        graphics_device: &GraphicsDevice,
        camera: &(impl Camera<fre> + ?Sized),
    ) -> Self {
        Self::new(
            graphics_device,
            *camera.projection_transform(),
            Cow::Borrowed("Camera"),
        )
    }

    /// Returns the layout of the bind group for the camera projection uniform.
    ///
    /// The layout will remain valid even though the projection may change.
    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.bind_group_layout
    }

    /// Returns the bind group for the camera projection uniform.
    ///
    /// The bind group will remain valid even though the projection may change.
    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }

    /// Returns the input required for accessing the camera projection uniform
    /// in a shader.
    pub const fn shader_input() -> &'static CameraShaderInput {
        &Self::SHADER_INPUT
    }

    /// Ensures that the GPU buffer is in sync with the projection transform of
    /// the given camera.
    pub fn sync_with_camera(
        &mut self,
        graphics_device: &GraphicsDevice,
        camera: &(impl Camera<fre> + ?Sized),
    ) {
        if camera.projection_transform_changed() {
            self.sync_gpu_buffer(graphics_device, camera.projection_transform());
            camera.reset_projection_change_tracking();
        }
    }

    /// Creates a new manager with a GPU buffer initialized from the given
    /// projection transform.
    fn new(
        graphics_device: &GraphicsDevice,
        projection_transform: Projective3<fre>,
        label: Cow<'static, str>,
    ) -> Self {
        let projection_uniform = CameraProjectionUniform::new(projection_transform);

        let projection_uniform_gpu_buffer = GPUBuffer::new_buffer_for_single_uniform(
            graphics_device,
            &projection_uniform,
            label.clone(),
        );

        let bind_group_layout =
            Self::create_bind_group_layout(graphics_device.device(), Self::VISIBILITY, &label);

        let bind_group = Self::create_bind_group(
            graphics_device.device(),
            &projection_uniform_gpu_buffer,
            &bind_group_layout,
            &label,
        );

        Self {
            projection_uniform_gpu_buffer,
            bind_group_layout,
            bind_group,
        }
    }

    /// Creates the bind group layout entry for the camera projection uniform,
    /// assigned to the given binding and with the given visibility.
    fn create_bind_group_layout_entry(
        binding: u32,
        visibility: wgpu::ShaderStages,
    ) -> wgpu::BindGroupLayoutEntry {
        CameraProjectionUniform::create_bind_group_layout_entry(binding, visibility)
    }

    fn create_bind_group_layout(
        device: &wgpu::Device,
        visibility: wgpu::ShaderStages,
        label: &str,
    ) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[Self::create_bind_group_layout_entry(
                Self::BINDING,
                visibility,
            )],
            label: Some(&format!("{} bind group layout", label)),
        })
    }

    fn create_bind_group(
        device: &wgpu::Device,
        projection_uniform_gpu_buffer: &GPUBuffer,
        layout: &wgpu::BindGroupLayout,
        label: &str,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout,
            entries: &[uniform::create_single_uniform_bind_group_entry(
                Self::BINDING,
                projection_uniform_gpu_buffer,
            )],
            label: Some(&format!("{} bind group", label)),
        })
    }

    fn sync_gpu_buffer(
        &mut self,
        graphics_device: &GraphicsDevice,
        projection_transform: &Projective3<fre>,
    ) {
        self.projection_uniform_gpu_buffer
            .update_first_bytes(graphics_device, bytemuck::bytes_of(projection_transform));
    }
}

impl CameraProjectionUniform {
    fn new(transform: Projective3<fre>) -> Self {
        Self {
            transform,
            jitter_offsets: Self::generate_jitter_offsets(),
        }
    }

    fn generate_jitter_offsets() -> [Vector4<fre>; JITTER_COUNT] {
        let mut offsets = [Vector4::zeros(); JITTER_COUNT];
        let halton_x = HaltonSequenceIter::new(JITTER_BASES.0);
        let halton_y = HaltonSequenceIter::new(JITTER_BASES.1);
        for ((offset, x), y) in offsets.iter_mut().zip(halton_x).zip(halton_y) {
            offset.x = 2.0 * x - 1.0;
            offset.y = 2.0 * y - 1.0;
        }
        offsets
    }
}

impl UniformBufferable for CameraProjectionUniform {
    const ID: ConstStringHash64 = ConstStringHash64::new("Camera projection");

    fn create_bind_group_layout_entry(
        binding: u32,
        visibility: wgpu::ShaderStages,
    ) -> wgpu::BindGroupLayoutEntry {
        uniform::create_uniform_buffer_bind_group_layout_entry(binding, visibility)
    }
}
assert_uniform_valid!(CameraProjectionUniform);

impl HaltonSequenceIter {
    fn new(base: u32) -> Self {
        Self { base, n: 0, d: 1 }
    }
}

impl Iterator for HaltonSequenceIter {
    type Item = fre;

    fn next(&mut self) -> Option<Self::Item> {
        let x = self.d - self.n;
        if x == 1 {
            self.n = 1;
            self.d *= self.base;
        } else {
            let mut y = self.d / self.base;
            while x <= y {
                y /= self.base;
            }
            self.n = (self.base + 1) * y - x;
        }
        Some(self.n as fre / self.d as fre)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn halton_sequence_for_base_2_is_correct() {
        let halton = HaltonSequenceIter::new(2);
        let expected = [
            1.0 / 2.0,
            1.0 / 4.0,
            3.0 / 4.0,
            1.0 / 8.0,
            5.0 / 8.0,
            3.0 / 8.0,
            7.0 / 8.0,
            1.0 / 16.0,
            9.0 / 16.0,
        ];
        for (i, x) in halton.take(9).enumerate() {
            assert_eq!(x, expected[i]);
        }
    }

    #[test]
    fn halton_sequence_for_base_3_is_correct() {
        let halton = HaltonSequenceIter::new(3);
        let expected = [
            1.0 / 3.0,
            2.0 / 3.0,
            1.0 / 9.0,
            4.0 / 9.0,
            7.0 / 9.0,
            2.0 / 9.0,
            5.0 / 9.0,
            8.0 / 9.0,
            1.0 / 27.0,
        ];
        for (i, x) in halton.take(9).enumerate() {
            assert_eq!(x, expected[i]);
        }
    }
}
