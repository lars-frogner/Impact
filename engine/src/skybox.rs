//! Skybox.

pub mod resource;

use crate::gpu::texture::TextureID;
use bytemuck::{Pod, Zeroable};
use roc_codegen::roc;

/// A skybox specified by a cubemap texture and a maximum luminance (the
/// luminance that a texel value of unity should be mapped to).
#[roc(parents = "Skybox")]
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Zeroable, Pod)]
pub struct Skybox {
    cubemap_texture_id: TextureID,
    max_luminance: f32,
}

#[roc]
impl Skybox {
    /// Creates a new skybox with the given cubemap texture and maximum
    /// luminance.
    #[roc(body = "{ cubemap_texture_id, max_luminance }")]
    pub fn new(cubemap_texture_id: TextureID, max_luminance: f32) -> Self {
        Self {
            cubemap_texture_id,
            max_luminance,
        }
    }
}
