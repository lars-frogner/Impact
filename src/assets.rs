//! Management of rendering assets.

pub mod lookup_table;

use crate::gpu::{
    GraphicsDevice,
    texture::{
        Sampler, SamplerConfig, SamplerID, TexelType, Texture, TextureConfig, TextureID,
        TextureLookupTable, mipmap::MipmapperGenerator,
    },
};
use anyhow::Result;
use impact_utils::hash32;
use serde::{Serialize, de::DeserializeOwned};
use std::{
    collections::{HashMap, hash_map::Entry},
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

impl Assets {
    /// Creates a new empty asset container.
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
            right_image_path.as_ref().display(),
            left_image_path.as_ref().display(),
            top_image_path.as_ref().display(),
            bottom_image_path.as_ref().display(),
            front_image_path.as_ref().display(),
            back_image_path.as_ref().display()
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

    /// Loads the image files at the given paths as an arrayed [`Texture`],
    /// unless it already has been loaded.
    ///
    /// # Returns
    /// A [`Result`] with the [`TextureID`] assigned to the loaded texture.
    ///
    /// # Errors
    /// See [`Texture::array_from_image_paths`].
    pub fn load_texture_array_from_paths<I, P>(
        &mut self,
        image_paths: impl IntoIterator<IntoIter = I>,
        texture_config: TextureConfig,
        sampler_config: Option<SamplerConfig>,
    ) -> Result<TextureID>
    where
        I: ExactSizeIterator<Item = P> + Clone,
        P: AsRef<Path>,
    {
        let image_paths = image_paths.into_iter();

        let mut label = "Texture array {{".to_string();
        for (idx, path) in image_paths.clone().enumerate() {
            label.push_str(&path.as_ref().to_string_lossy());
            if idx < image_paths.len() - 1 {
                label.push_str(", ");
            }
        }
        label.push_str("}}");

        let texture_id = TextureID(hash32!(&label));

        if let Entry::Vacant(entry) = self.textures.entry(texture_id) {
            let sampler_id = sampler_config.as_ref().map(Into::into);

            let texture = entry.insert(Texture::array_from_image_paths(
                &self.graphics_device,
                &self.mipmapper_generator,
                image_paths,
                texture_config,
                sampler_id,
                &label,
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

    /// Unless a texture with the given label has already been loaded, this
    /// function loads the lookup table generated by the given function as a
    /// [`Texture`]. Also creates a sampler for the texture using the given
    /// sampler configuration.
    ///
    /// # Returns
    /// A [`Result`] with the [`TextureID`] assigned to the loaded texture.
    ///
    /// # Errors
    /// See [`Texture::from_lookup_table`].
    pub fn load_texture_from_generated_lookup_table<T: TexelType>(
        &mut self,
        generate_table: impl Fn() -> Result<TextureLookupTable<T>>,
        sampler_config: SamplerConfig,
        label: impl AsRef<str>,
    ) -> Result<TextureID> {
        let label = label.as_ref();
        let texture_id = TextureID(hash32!(label));
        if let Entry::Vacant(entry) = self.textures.entry(texture_id) {
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
    /// Also creates a sampler for the texture using the given
    /// sampler configuration.
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
        sampler_config: SamplerConfig,
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
            sampler_config,
            table_file_path.to_string_lossy(),
        )
    }
}
