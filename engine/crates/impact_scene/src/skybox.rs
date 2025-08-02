//! Skybox.

pub mod gpu_resource;

use anyhow::Result;
use bytemuck::{Pod, Zeroable};
use gpu_resource::SkyboxGPUResource;
use impact_gpu::device::GraphicsDevice;
use impact_texture::{
    TextureID,
    gpu_resource::{SamplerMap, TextureMap},
};
use roc_integration::roc;

/// A skybox specified by a cubemap texture and a maximum luminance (the
/// luminance that a texel value of unity should be mapped to).
#[roc]
#[repr(C)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Clone, Copy, Debug, Zeroable, Pod)]
pub struct Skybox {
    cubemap_texture_id: TextureID,
    max_luminance: f64,
}

#[roc]
impl Skybox {
    /// Creates a new skybox with the given cubemap texture and maximum
    /// luminance.
    #[roc(body = "{ cubemap_texture_id, max_luminance }")]
    pub fn new(cubemap_texture_id: TextureID, max_luminance: f64) -> Self {
        Self {
            cubemap_texture_id,
            max_luminance,
        }
    }
}

impl PartialEq for Skybox {
    fn eq(&self, other: &Self) -> bool {
        self.cubemap_texture_id == other.cubemap_texture_id
            && self.max_luminance.to_bits() == other.max_luminance.to_bits()
    }
}

impl Eq for Skybox {}

/// Performs any required updates for keeping the skybox GPU resources in sync
/// with the given scene skybox.
///
/// # Errors
/// Returns an error if the skybox cubemap texture or sampler is missing.
pub fn sync_gpu_resources_for_skybox(
    skybox: Option<&Skybox>,
    graphics_device: &GraphicsDevice,
    textures: &TextureMap,
    samplers: &SamplerMap,
    skybox_gpu_resources: &mut Option<SkyboxGPUResource>,
) -> Result<()> {
    if let Some(&skybox) = skybox {
        if let Some(skybox_gpu_resources) = skybox_gpu_resources {
            skybox_gpu_resources.sync_with_skybox(graphics_device, textures, samplers, skybox)?;
        } else {
            *skybox_gpu_resources = Some(SkyboxGPUResource::for_skybox(
                graphics_device,
                textures,
                samplers,
                skybox,
            )?);
        }
    } else {
        skybox_gpu_resources.take();
    }
    Ok(())
}
