//! Management of rendering assets.

use crate::rendering::{ImageTexture, Shader};
use std::collections::HashMap;

stringhash_newtype!(
    /// Identifier for specific shaders.
    /// Wraps a [`StringHash`](crate::hash::StringHash).
    [pub] ShaderID
);

stringhash_newtype!(
    /// Identifier for specific textures.
    /// Wraps a [`StringHash`](crate::hash::StringHash).
    [pub] TextureID
);

/// Container for any rendering assets that never change.
#[derive(Debug, Default)]
pub struct Assets {
    /// Shader programs.
    pub shaders: HashMap<ShaderID, Shader>,
    /// Textures sourced from images.
    pub image_textures: HashMap<TextureID, ImageTexture>,
}

impl Assets {
    pub fn new() -> Self {
        Self {
            shaders: HashMap::new(),
            image_textures: HashMap::new(),
        }
    }
}
