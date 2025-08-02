//! Skybox.

pub mod resource;

use bytemuck::{Pod, Zeroable};
use impact_texture::TextureID;
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
