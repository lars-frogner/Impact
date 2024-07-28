//! Buffering of camera data for rendering.

use crate::{
    assert_uniform_valid,
    camera::SceneCamera,
    geometry::Frustum,
    gpu::{
        buffer::GPUBuffer,
        rendering::fre,
        shader::CameraShaderInput,
        uniform::{self, UniformBufferable},
        GraphicsDevice,
    },
};
use bytemuck::{Pod, Zeroable};
use impact_utils::{ConstStringHash64, HaltonSequence};
use nalgebra::{Projective3, Vector4};
use std::{borrow::Cow, sync::LazyLock};

/// Length of the sequence of jitter offsets to apply to the projection for
/// temporal anti-aliasing.
pub const JITTER_COUNT: usize = 8;

/// Bases for the Halton sequence used to generate the jitter offsets.
const JITTER_BASES: (u64, u64) = (2, 3);

static JITTER_OFFSETS: LazyLock<[Vector4<fre>; JITTER_COUNT]> =
    LazyLock::new(CameraProjectionUniform::generate_jitter_offsets);

/// Owner and manager of a GPU buffer for a camera projection transformation.
#[derive(Debug)]
pub struct CameraGPUBufferManager {
    projection_uniform_gpu_buffer: GPUBuffer,
    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
    jitter_enabled: bool,
}

/// Uniform holding the projection transformation of a camera, the corners of
/// the far plane of the view frustum in camera space, the inverse far-plane
/// z-coordinate and the sequence of jitter offsets to apply to the projection
/// for temporal anti-aliasing.
///
/// The size of this struct has to be a multiple of 16 bytes as required for
/// uniforms.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
struct CameraProjectionUniform {
    transform: Projective3<fre>,
    /// The corners are listed in the order consistent with
    /// [`TriangleMesh::create_screen_filling_quad`](`crate::mesh::TriangleMesh::create_screen_filling_quad`),
    /// which means it can be indexed into using the `vertex_index` built-in in
    /// the vertex shader when rendering a screen-filling quad to obtain the
    /// far-plane corner for the current screen corner. When passed to the
    /// fragment shader with interpolation, this will yield the camera-space
    /// position of the point on the far plane corresponding to the current
    /// fragment. By scaling this position by the fragment's normalized linear
    /// depth (the camera-space z-coordinate of the point on the object it
    /// covers divided by the far-plane z-coordinate), we can reconstruct the
    /// camera-space position of the fragment from the depth.
    frustum_far_plane_corners: [Vector4<fre>; 4],
    inverse_far_plane_z: Vector4<fre>,
    jitter_offsets: [Vector4<fre>; JITTER_COUNT],
}

impl CameraGPUBufferManager {
    const BINDING: u32 = 0;
    const VISIBILITY: wgpu::ShaderStages = wgpu::ShaderStages::VERTEX_FRAGMENT;
    const SHADER_INPUT: CameraShaderInput = CameraShaderInput {
        projection_binding: Self::BINDING,
    };

    /// Creates a new manager with a GPU buffer initialized from the projection
    /// transform of the given camera.
    pub fn for_camera(graphics_device: &GraphicsDevice, camera: &SceneCamera<fre>) -> Self {
        let label = Cow::Borrowed("Camera");

        let projection_uniform = CameraProjectionUniform::new(camera);

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
            jitter_enabled: camera.jitter_enabled(),
        }
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
        camera: &SceneCamera<fre>,
    ) {
        if camera.camera().projection_transform_changed()
            || camera.jitter_enabled() != self.jitter_enabled
        {
            self.sync_gpu_buffer(graphics_device, camera);
            camera.camera().reset_projection_change_tracking();
            self.jitter_enabled = camera.jitter_enabled();
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

    fn sync_gpu_buffer(&self, graphics_device: &GraphicsDevice, camera: &SceneCamera<fre>) {
        let projection_uniform = CameraProjectionUniform::new(camera);
        self.projection_uniform_gpu_buffer
            .update_valid_bytes(graphics_device, bytemuck::bytes_of(&projection_uniform));
    }
}

impl CameraProjectionUniform {
    fn new(camera: &SceneCamera<fre>) -> Self {
        let transform = *camera.camera().projection_transform();

        let frustum_far_plane_corners =
            Self::compute_far_plane_corners(camera.camera().view_frustum());

        // Important: Don't use camera.far_distance() for this, or reconstructed
        // positions will be off because the corners may not be exactly at the
        // far distance due to inaccuracies
        let inverse_far_plane_z =
            Vector4::new(frustum_far_plane_corners[0].z.recip(), 0.0, 0.0, 0.0);

        let jitter_offsets = if camera.jitter_enabled() {
            *JITTER_OFFSETS
        } else {
            [Vector4::zeros(); JITTER_COUNT]
        };

        Self {
            transform,
            frustum_far_plane_corners,
            inverse_far_plane_z,
            jitter_offsets,
        }
    }

    fn compute_far_plane_corners(view_frustum: &Frustum<fre>) -> [Vector4<fre>; 4] {
        let corners = view_frustum.compute_corners();
        [
            Vector4::new(corners[1].x, corners[1].y, corners[1].z, 0.0), // lower left
            Vector4::new(corners[5].x, corners[5].y, corners[5].z, 0.0), // lower right
            Vector4::new(corners[7].x, corners[7].y, corners[7].z, 0.0), // upper right
            Vector4::new(corners[3].x, corners[3].y, corners[3].z, 0.0), // upper left
        ]
    }

    fn generate_jitter_offsets() -> [Vector4<fre>; JITTER_COUNT] {
        let mut offsets = [Vector4::zeros(); JITTER_COUNT];
        let halton_x = HaltonSequence::<fre>::new(JITTER_BASES.0);
        let halton_y = HaltonSequence::<fre>::new(JITTER_BASES.1);
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
