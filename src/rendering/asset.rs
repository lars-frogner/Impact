//! Static assets for rendering.

mod shader;
mod texture;

pub use shader::Shader;
pub use texture::ImageTexture;

use crate::hash::StringHash;
use std::collections::HashMap;

pub type AssetID = StringHash;
pub type AssetMap<T> = HashMap<AssetID, T>;

/// Container for any rendering assets that never change.
#[derive(Debug)]
pub struct Assets {
    /// Shader programs.
    pub shaders: AssetMap<Shader>,
    /// Textures sourced from images.
    pub image_textures: AssetMap<ImageTexture>,
}

impl Assets {
    pub fn new() -> Self {
        Self {
            shaders: HashMap::new(),
            image_textures: HashMap::new(),
        }
    }
}

impl Default for Assets {
    fn default() -> Self {
        Self::new()
    }
}
