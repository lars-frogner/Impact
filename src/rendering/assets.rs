//! Management of rendering assets.

use crate::rendering::ImageTexture;
use impact_utils::stringhash32_newtype;
use std::collections::HashMap;

stringhash32_newtype!(
    /// Identifier for specific textures.
    /// Wraps a [`StringHash32`](impact_utils::StringHash32).
    [pub] TextureID
);

/// Container for any rendering assets that never change.
#[derive(Debug, Default)]
pub struct Assets {
    /// Textures sourced from images.
    pub image_textures: HashMap<TextureID, ImageTexture>,
}

impl Assets {
    pub fn new() -> Self {
        Self {
            image_textures: HashMap::new(),
        }
    }
}
