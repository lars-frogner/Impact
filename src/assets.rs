//! Management of rendering assets.

use crate::gpu::{
    rendering::brdf,
    texture::{
        mipmap::MipmapperGenerator, Sampler, SamplerConfig, SamplerID, TexelType, Texture,
        TextureAddressingConfig, TextureConfig, TextureFilteringConfig, TextureID,
        TextureLookupTable,
    },
    GraphicsDevice,
};
use anyhow::Result;
use impact_utils::hash32;
use lazy_static::lazy_static;
use serde::{de::DeserializeOwned, Serialize};
use std::{
    collections::{hash_map::Entry, HashMap},
    path::Path,
    sync::Arc,
};

/// Container for any rendering assets that never change.
#[derive(Debug)]
pub struct Assets {
    graphics_device: Arc<GraphicsDevice>,
    mipmapper_generator: Arc<MipmapperGenerator>,
    pub textures: HashMap<TextureID, Texture>,
    pub samplers: HashMap<SamplerID, Sampler>,
}

lazy_static! {
    static ref SPECULAR_GGX_REFLECTANCE_LOOKUP_TABLE_TEXTURE_ID: TextureID = TextureID(hash32!(
        Assets::SPECULAR_GGX_REFLECTANCE_LOOKUP_TABLE_TEXTURE_PATH
    ));
}

impl Assets {
    const SPECULAR_GGX_REFLECTANCE_LOOKUP_TABLE_TEXTURE_PATH: &'static str =
        "assets/specular_ggx_reflectance_lookup_table.mpk";

    pub fn specular_ggx_reflectance_lookup_table_texture_id() -> TextureID {
        *SPECULAR_GGX_REFLECTANCE_LOOKUP_TABLE_TEXTURE_ID
    }

    pub fn new(
        graphics_device: Arc<GraphicsDevice>,
        mipmapper_generator: Arc<MipmapperGenerator>,
    ) -> Self {
        Self {
            graphics_device,
            mipmapper_generator,
            textures: HashMap::new(),
            samplers: HashMap::new(),
        }
    }

    pub fn new_with_default_lookup_tables(
        graphics_device: Arc<GraphicsDevice>,
        mipmapper_generator: Arc<MipmapperGenerator>,
    ) -> Result<Self> {
        let mut assets = Self::new(graphics_device, mipmapper_generator);
        assets.load_default_lookup_table_textures()?;
        Ok(assets)
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
        image_path: impl AsRef<Path>,
        texture_config: TextureConfig,
        sampler_config: Option<SamplerConfig>,
    ) -> Result<TextureID> {
        let texture_id = TextureID(hash32!(image_path.as_ref().to_string_lossy()));
        if let Entry::Vacant(entry) = self.textures.entry(texture_id) {
            let sampler_id = sampler_config.as_ref().map(Into::into);

            let texture = entry.insert(Texture::from_path(
                &self.graphics_device,
                &self.mipmapper_generator,
                image_path,
                texture_config,
                sampler_id,
            )?);

            if let (Some(sampler_id), Some(sampler_config)) = (texture.sampler_id(), sampler_config)
            {
                self.samplers
                    .entry(sampler_id)
                    .or_insert_with(|| Sampler::create(&self.graphics_device, sampler_config));
            }
        }
        Ok(texture_id)
    }

    /// Loads the cubemap face image files at the given paths as a cubemap
    /// [`Texture`], unless it already has been loaded.
    ///
    /// # Returns
    /// A [`Result`] with the [`TextureID`] assigned to the loaded texture.
    ///
    /// # Errors
    /// See [`Texture::from_cubemap_image_paths`].
    pub fn load_cubemap_texture_from_paths<P: AsRef<Path>>(
        &mut self,
        right_image_path: P,
        left_image_path: P,
        top_image_path: P,
        bottom_image_path: P,
        front_image_path: P,
        back_image_path: P,
        texture_config: TextureConfig,
        sampler_config: Option<SamplerConfig>,
    ) -> Result<TextureID> {
        let texture_id = TextureID(hash32!(format!(
            "Cubemap {{{}, {}, {}, {}, {}, {}}}",
            right_image_path.as_ref().to_string_lossy(),
            left_image_path.as_ref().to_string_lossy(),
            top_image_path.as_ref().to_string_lossy(),
            bottom_image_path.as_ref().to_string_lossy(),
            front_image_path.as_ref().to_string_lossy(),
            back_image_path.as_ref().to_string_lossy()
        )));

        if let Entry::Vacant(entry) = self.textures.entry(texture_id) {
            let sampler_id = sampler_config.as_ref().map(Into::into);

            let texture = entry.insert(Texture::from_cubemap_image_paths(
                &self.graphics_device,
                right_image_path,
                left_image_path,
                top_image_path,
                bottom_image_path,
                front_image_path,
                back_image_path,
                texture_config,
                sampler_id,
            )?);

            if let (Some(sampler_id), Some(sampler_config)) = (texture.sampler_id(), sampler_config)
            {
                self.samplers
                    .entry(sampler_id)
                    .or_insert_with(|| Sampler::create(&self.graphics_device, sampler_config));
            }
        }

        Ok(texture_id)
    }

    /// Loads all default lookup tables as textures. The tables are read from
    /// file or computed.
    ///
    /// # Errors
    /// Returns an error if a computed table can not be saved to file.
    /// Additionally, see [`Texture::from_lookup_table`].
    pub fn load_default_lookup_table_textures(&mut self) -> Result<()> {
        self.load_texture_from_stored_or_computed_lookup_table(
            Self::SPECULAR_GGX_REFLECTANCE_LOOKUP_TABLE_TEXTURE_PATH,
            || brdf::create_specular_ggx_reflectance_lookup_tables(1024, 512),
        )?;

        Ok(())
    }

    /// Unless a texture with the given label has already been loaded, this
    /// function loads the lookup table generated by the given function as a
    /// [`Texture`].
    ///
    /// # Returns
    /// A [`Result`] with the [`TextureID`] assigned to the loaded texture.
    ///
    /// # Errors
    /// See [`Texture::from_lookup_table`].
    pub fn load_texture_from_generated_lookup_table<T: TexelType>(
        &mut self,
        generate_table: impl Fn() -> Result<TextureLookupTable<T>>,
        label: impl AsRef<str>,
    ) -> Result<TextureID> {
        let label = label.as_ref();
        let texture_id = TextureID(hash32!(label));
        if let Entry::Vacant(entry) = self.textures.entry(texture_id) {
            let sampler_config = SamplerConfig {
                addressing: TextureAddressingConfig::CLAMPED,
                filtering: TextureFilteringConfig::NONE,
            };
            let sampler_id = (&sampler_config).into();

            entry.insert(Texture::from_lookup_table(
                &self.graphics_device,
                &generate_table()?,
                label,
                Some(sampler_id),
            )?);

            self.samplers
                .entry(sampler_id)
                .or_insert_with(|| Sampler::create(&self.graphics_device, sampler_config));
        }
        Ok(texture_id)
    }

    /// Unless a texture with ID corresponding to the given path has already
    /// been loaded, this function loads as a [`Texture`] the lookup table that
    /// is either taken from the file at the given path if it exists, otherwise
    /// it is computed with the given function and saved at the given path.
    ///
    /// # Returns
    /// A [`Result`] with the [`TextureID`] assigned to the loaded texture.
    ///
    /// # Errors
    /// Returns an error if the computed table can not be saved to file.
    /// Additionally, see [`Texture::from_lookup_table`].
    pub fn load_texture_from_stored_or_computed_lookup_table<T>(
        &mut self,
        table_file_path: impl AsRef<Path>,
        compute_table: impl Fn() -> TextureLookupTable<T>,
    ) -> Result<TextureID>
    where
        T: TexelType + Serialize + DeserializeOwned,
    {
        let table_file_path = table_file_path.as_ref();

        self.load_texture_from_generated_lookup_table(
            || {
                TextureLookupTable::<T>::read_from_file(table_file_path).or_else(|_| {
                    let table = compute_table();
                    table.save_to_file(table_file_path)?;
                    Ok(table)
                })
            },
            table_file_path.to_string_lossy(),
        )
    }
}
