//! Static assets for rendering.

mod shader;
mod texture;

use std::collections::HashMap;

pub use shader::Shader;
pub use texture::ImageTexture;

pub type AssetIdent = String;
pub type AssetMap<T> = HashMap<AssetIdent, T>;

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
