//! Skybox.

pub mod resource;

use crate::gpu::{rendering::fre, texture::TextureID};

/// A skybox specified by a cubemap texture and a maximum luminance (the
/// luminance that a texel value of unity should be mapped to).
#[derive(Clone, Debug, PartialEq)]
pub struct Skybox {
    cubemap_texture_id: TextureID,
    max_luminance: fre,
}

impl Skybox {
    /// Creates a new skybox with the given cubemap texture and maximum
    /// luminance.
    pub fn new(cubemap_texture_id: TextureID, max_luminance: fre) -> Self {
        Self {
            cubemap_texture_id,
            max_luminance,
        }
    }
}