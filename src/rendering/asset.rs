//! Static assets for rendering.

mod shader;
mod texture;

pub use shader::{Shader, ShaderID};
pub use texture::{ImageTexture, TextureID};

use std::collections::HashMap;

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
