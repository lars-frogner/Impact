//! Management of rendering assets.

use crate::rendering::{ImageTexture, Shader};
use impact_utils::{stringhash32_newtype, stringhash64_newtype};
use std::{collections::HashMap, sync::Arc};

stringhash64_newtype!(
    /// Identifier for specific shaders.
    /// Wraps a [`StringHash64`](impact_utils::StringHash64).
    [pub] ShaderID
);

stringhash32_newtype!(
    /// Identifier for specific textures.
    /// Wraps a [`StringHash32`](impact_utils::StringHash32).
    [pub] TextureID
);

/// Container for any rendering assets that never change.
#[derive(Debug, Default)]
pub struct Assets {
    /// Shader programs.
    pub shaders: HashMap<ShaderID, Arc<Shader>>,
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
