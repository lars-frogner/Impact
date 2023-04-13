//! Management of rendering assets.

use crate::rendering::{
    CoreRenderingSystem, TexelType, Texture, TextureConfig, TextureLookupTable,
};
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
    pub textures: HashMap<TextureID, Texture>,
}

impl Assets {
    pub fn new() -> Self {
        Self {
            textures: HashMap::new(),
        }
    }

    /// Loads the image file at the given path as a [`Texture`], unless it
    /// already has been loaded.
    ///
    /// # Returns
    /// A [`Result`] with the [`TextureID`] assigned to the loaded texture.
    ///
    /// # Errors
    /// See [`Texture::from_path`].
    pub fn load_texture_from_path(
        &mut self,
        core_system: &CoreRenderingSystem,
        image_path: impl AsRef<Path>,
        config: TextureConfig,
    ) -> Result<TextureID> {
        let texture_id = TextureID(hash32!(image_path.as_ref().to_string_lossy()));
        if let Entry::Vacant(entry) = self.textures.entry(texture_id) {
            entry.insert(Texture::from_path(core_system, image_path, config)?);
        }
        Ok(texture_id)
    }

    /// Loads the given lookup table as a [`Texture`], unless it already has
    /// been loaded.
    ///
    /// # Returns
    /// A [`Result`] with the [`TextureID`] assigned to the loaded texture.
    ///
    /// # Errors
    /// See [`Texture::from_lookup_table`].
    pub fn load_texture_from_lookup_table<T: TexelType>(
        &mut self,
        core_system: &CoreRenderingSystem,
        table: &TextureLookupTable<T>,
        label: impl AsRef<str>,
    ) -> Result<TextureID> {
        let label = label.as_ref();
        let texture_id = TextureID(hash32!(label));
        if let Entry::Vacant(entry) = self.textures.entry(texture_id) {
            entry.insert(Texture::from_lookup_table(core_system, table, label)?);
        }
        Ok(texture_id)
    }
}
