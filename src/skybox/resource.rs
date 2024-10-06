//! Management of GPU resources for skyboxes.

use crate::skybox::Skybox;
use crate::{
    assert_uniform_valid,
    assets::Assets,
    gpu::{
        resource_group::GPUResourceGroup,
        uniform::{self, SingleUniformGPUBuffer, UniformBufferable},
        GraphicsDevice,
    },
};
use anyhow::{anyhow, Result};
use bytemuck::{Pod, Zeroable};
use impact_utils::ConstStringHash64;
use std::borrow::Cow;

/// Manager for GPU resources used for a skybox.
#[derive(Debug)]
pub struct SkyboxGPUResourceManager {
    skybox: Skybox,
    gpu_resource_group: GPUResourceGroup,
}

impl SkyboxGPUResourceManager {
    /// Returns the binding location of the uniform of skybox properties.
    pub const fn properties_uniform_binding() -> u32 {
        0
    }
    /// Returns the binding location of the skybox cubemap texture.
    pub const fn texture_binding() -> u32 {
        1
    }
    /// Returns the binding location of the skybox sampler.
    pub const fn sampler_binding() -> u32 {
        2
    }

    /// Creates a new GPU resource manager for the given skybox.
    ///
    /// # Errors
    /// Returns an error if the skybox cubemap texture or sampler is missing.
    pub fn for_skybox(
        graphics_device: &GraphicsDevice,
        assets: &Assets,
        skybox: Skybox,
    ) -> Result<Self> {
        let cubemap_texture = assets
            .textures
            .get(&skybox.cubemap_texture_id)
            .ok_or_else(|| anyhow!("Missing texture for skybox"))?;

        let sampler = cubemap_texture
            .sampler_id()
            .and_then(|sampler_id| assets.samplers.get(&sampler_id))
            .ok_or_else(|| anyhow!("Missing sampler for skybox"))?;

        let properties_uniform = SkyboxProperties::new(skybox.max_luminance);

        let properties_uniform_buffer = SingleUniformGPUBuffer::for_uniform(
            graphics_device,
            &properties_uniform,
            wgpu::ShaderStages::FRAGMENT,
            Cow::Borrowed("Skybox properties"),
        );

        let gpu_resource_group = GPUResourceGroup::new(
            graphics_device,
            vec![properties_uniform_buffer],
            &[],
            &[cubemap_texture],
            &[sampler],
            wgpu::ShaderStages::FRAGMENT,
            "Skybox properties",
        );

        Ok(Self {
            skybox,
            gpu_resource_group,
        })
    }

    /// Returns the skybox whose GPU resources are managed by this manager.
    pub fn skybox(&self) -> &Skybox {
        &self.skybox
    }

    /// Returns the bind group layout for the GPU resource group comprised of
    /// the properties uniform and the cubemap texture and sampler for the
    /// skybox.
    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        self.gpu_resource_group.bind_group_layout()
    }

    /// Returns the bind group for the GPU resource group comprised of the
    /// properties uniform and the cubemap texture and sampler for the skybox.
    pub fn bind_group(&self) -> &wgpu::BindGroup {
        self.gpu_resource_group.bind_group()
    }

    /// Synchronizes the skybox GPU resources with the given skybox.
    pub fn sync_with_skybox(
        &mut self,
        graphics_device: &GraphicsDevice,
        assets: &Assets,
        skybox: &Skybox,
    ) -> Result<()> {
        if skybox != &self.skybox {
            *self = Self::for_skybox(graphics_device, assets, skybox.clone())?;
        }
        Ok(())
    }
}

/// Uniform holding the maximum possible luminance from a skybox.
///
/// The size of this struct has to be a multiple of 16 bytes as required for
/// uniforms.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
struct SkyboxProperties {
    max_luminance: f32,
    _pad: [u8; 12],
}

impl SkyboxProperties {
    fn new(max_luminance: f32) -> Self {
        Self {
            max_luminance,
            _pad: [0; 12],
        }
    }
}

impl UniformBufferable for SkyboxProperties {
    const ID: ConstStringHash64 = ConstStringHash64::new("Skybox properties");

    fn create_bind_group_layout_entry(
        binding: u32,
        visibility: wgpu::ShaderStages,
    ) -> wgpu::BindGroupLayoutEntry {
        uniform::create_uniform_buffer_bind_group_layout_entry(binding, visibility)
    }
}
assert_uniform_valid!(SkyboxProperties);
