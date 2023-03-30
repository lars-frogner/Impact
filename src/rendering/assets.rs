//! Management of rendering assets.

use crate::rendering::{ColorImageTexture, CoreRenderingSystem};
use anyhow::Result;
use impact_utils::{hash32, stringhash32_newtype};
use std::{
    collections::{hash_map::Entry, HashMap},
    path::Path,
};

stringhash32_newtype!(
    /// Identifier for specific textures.
    /// Wraps a [`StringHash32`](impact_utils::StringHash32).
    [pub] TextureID
);

/// Container for any rendering assets that never change.
#[derive(Debug, Default)]
pub struct Assets {
    /// Textures sourced from images.
    pub color_image_textures: HashMap<TextureID, ColorImageTexture>,
}

impl Assets {
    pub fn new() -> Self {
        Self {
            color_image_textures: HashMap::new(),
        }
    }

    /// Loads the image file at the given path as an [`ColorImageTexture`],
    /// unless it already has been loaded.
    ///
    /// # Returns
    /// A [`Result`] with the [`TextureID`] assigned to the loaded texture.
    ///
    /// # Errors
    /// See [`ColorImageTexture::from_path`].
    pub fn load_color_image_texture_from_path(
        &mut self,
        core_system: &CoreRenderingSystem,
        image_path: impl AsRef<Path>,
    ) -> Result<TextureID> {
        let texture_id = TextureID(hash32!(image_path.as_ref().to_string_lossy()));
        if let Entry::Vacant(entry) = self.color_image_textures.entry(texture_id) {
            entry.insert(ColorImageTexture::from_path(core_system, image_path)?);
        }
        Ok(texture_id)
    }
}
