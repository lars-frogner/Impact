//! Management of rendering assets.

use crate::rendering::{ColorSpace, CoreRenderingSystem, ImageTexture};
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
    pub image_textures: HashMap<TextureID, ImageTexture>,
}

impl Assets {
    pub fn new() -> Self {
        Self {
            image_textures: HashMap::new(),
        }
    }

    /// Loads the image file at the given path as an [`ImageTexture`],
    /// unless it already has been loaded.
    ///
    /// # Returns
    /// A [`Result`] with the [`TextureID`] assigned to the loaded texture.
    ///
    /// # Errors
    /// See [`ImageTexture::from_path`].
    pub fn load_image_texture_from_path(
        &mut self,
        core_system: &CoreRenderingSystem,
        image_path: impl AsRef<Path>,
        color_space: ColorSpace,
    ) -> Result<TextureID> {
        let texture_id = TextureID(hash32!(image_path.as_ref().to_string_lossy()));
        if let Entry::Vacant(entry) = self.image_textures.entry(texture_id) {
            entry.insert(ImageTexture::from_path(
                core_system,
                image_path,
                color_space,
            )?);
        }
        Ok(texture_id)
    }
}
