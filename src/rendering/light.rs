//! Management of light source data for rendering.

use crate::{
    rendering::{
        buffer::{self, UniformBufferable},
        uniform::{UniformRenderBufferManager, UniformTransferResult},
        CoreRenderingSystem, LightShaderInput,
    },
    scene::{DirectionalLight, LightStorage, PointLight},
};
use impact_utils::ConstStringHash64;

/// Manager of the set of uniform render buffers holding light source render
/// data. Also manages the bind group for these buffers.
#[derive(Debug)]
pub struct LightRenderBufferManager {
    point_light_render_buffer_manager: UniformRenderBufferManager,
    directional_light_render_buffer_manager: UniformRenderBufferManager,
    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
    shader_input: LightShaderInput,
}

impl LightRenderBufferManager {
    const POINT_LIGHT_BINDING: u32 = 0;
    const DIRECTIONAL_LIGHT_BINDING: u32 = 1;

    /// Creates a new manager with render buffers initialized from the given
    /// [`LightStorage`].
    pub fn for_light_storage(
        core_system: &CoreRenderingSystem,
        light_storage: &LightStorage,
    ) -> Self {
        let point_light_render_buffer_manager = UniformRenderBufferManager::for_uniform_buffer(
            core_system,
            light_storage.point_light_buffer(),
        );
        let directional_light_render_buffer_manager =
            UniformRenderBufferManager::for_uniform_buffer(
                core_system,
                light_storage.directional_light_buffer(),
            );

        let bind_group_layout = Self::create_bind_group_layout(core_system.device());

        let bind_group = Self::create_bind_group(
            core_system.device(),
            &point_light_render_buffer_manager,
            &directional_light_render_buffer_manager,
            &bind_group_layout,
        );

        let shader_input = Self::create_shader_input(
            &point_light_render_buffer_manager,
            &directional_light_render_buffer_manager,
        );

        Self {
            point_light_render_buffer_manager,
            directional_light_render_buffer_manager,
            bind_group_layout,
            bind_group,
            shader_input,
        }
    }

    /// Returns a reference to the bind group layout for the set of light
    /// uniform buffers.
    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.bind_group_layout
    }

    /// Returns a reference to the bind group for the set of light uniform
    /// buffers.
    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }

    /// Returns the input required for accessing the light data in a shader.
    pub fn shader_input(&self) -> &LightShaderInput {
        &self.shader_input
    }

    /// Ensures that the light uniform buffers are in sync with the light data
    /// in the given light storage. Will also recreate the bind group and update
    /// the shader input if any of the render buffers had to be reallocated.
    pub fn sync_with_light_storage(
        &mut self,
        core_system: &CoreRenderingSystem,
        light_storage: &LightStorage,
    ) {
        let point_light_transfer_result = self
            .point_light_render_buffer_manager
            .transfer_uniforms_to_render_buffer(core_system, light_storage.point_light_buffer());

        let directional_light_transfer_result = self
            .directional_light_render_buffer_manager
            .transfer_uniforms_to_render_buffer(
                core_system,
                light_storage.directional_light_buffer(),
            );

        if point_light_transfer_result == UniformTransferResult::CreatedNewBuffer
            || directional_light_transfer_result == UniformTransferResult::CreatedNewBuffer
        {
            // Recreate bind group and shader input
            self.bind_group = Self::create_bind_group(
                core_system.device(),
                &self.point_light_render_buffer_manager,
                &self.directional_light_render_buffer_manager,
                &self.bind_group_layout,
            );

            self.shader_input = Self::create_shader_input(
                &self.point_light_render_buffer_manager,
                &self.directional_light_render_buffer_manager,
            );
        }
    }

    fn create_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                PointLight::create_bind_group_layout_entry(Self::POINT_LIGHT_BINDING),
                DirectionalLight::create_bind_group_layout_entry(Self::DIRECTIONAL_LIGHT_BINDING),
            ],
            label: Some("Light bind group layout"),
        })
    }

    fn create_bind_group(
        device: &wgpu::Device,
        point_light_render_buffer_manager: &UniformRenderBufferManager,
        directional_light_render_buffer_manager: &UniformRenderBufferManager,
        layout: &wgpu::BindGroupLayout,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout,
            entries: &[
                point_light_render_buffer_manager
                    .create_bind_group_entry(Self::POINT_LIGHT_BINDING),
                directional_light_render_buffer_manager
                    .create_bind_group_entry(Self::DIRECTIONAL_LIGHT_BINDING),
            ],
            label: Some("Light bind group"),
        })
    }

    fn create_shader_input(
        point_light_render_buffer_manager: &UniformRenderBufferManager,
        directional_light_render_buffer_manager: &UniformRenderBufferManager,
    ) -> LightShaderInput {
        LightShaderInput {
            point_light_binding: Self::POINT_LIGHT_BINDING,
            directional_light_binding: Self::DIRECTIONAL_LIGHT_BINDING,
            max_point_light_count: point_light_render_buffer_manager.max_uniform_count() as u64,
            max_directional_light_count: directional_light_render_buffer_manager.max_uniform_count()
                as u64,
        }
    }
}

impl UniformBufferable for PointLight {
    const ID: ConstStringHash64 = ConstStringHash64::new("Point light");

    fn create_bind_group_layout_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
        buffer::create_uniform_buffer_bind_group_layout_entry(binding, wgpu::ShaderStages::FRAGMENT)
    }
}

impl UniformBufferable for DirectionalLight {
    const ID: ConstStringHash64 = ConstStringHash64::new("Directional light");

    fn create_bind_group_layout_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
        buffer::create_uniform_buffer_bind_group_layout_entry(binding, wgpu::ShaderStages::FRAGMENT)
    }
}
