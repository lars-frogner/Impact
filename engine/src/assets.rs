//! Management of rendering assets.

pub mod lookup_tables;

use anyhow::{Result, bail};
use impact_containers::HashMap;
use impact_gpu::{
    device::GraphicsDevice,
    texture::{
        self, Sampler, SamplerConfig, SamplerID, TexelType, Texture, TextureConfig, TextureID,
        TextureLookupTable, mipmap::MipmapperGenerator,
    },
};
use impact_material::MaterialTextureProvider;
use impact_math::hash32;
use impact_mesh::TriangleMeshSpecification;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::{
    collections::hash_map::Entry,
    path::{Path, PathBuf},
    sync::Arc,
};

/// Container for any rendering assets that never change.
#[derive(Debug)]
pub struct Assets {
    config: AssetConfig,
    graphics_device: Arc<GraphicsDevice>,
    mipmapper_generator: Arc<MipmapperGenerator>,
    pub textures: HashMap<TextureID, Texture>,
    pub samplers: HashMap<SamplerID, Sampler>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct AssetConfig {
    /// Path to the folder where automatically computed lookup tables should be
    /// stored.
    pub lookup_table_dir: PathBuf,
    /// Path to a file containing an [`AssetSpecifications`] object serialized
    /// as RON (Rusty Object Notation). The assets specified in the file will
    /// be automatically loaded on startup.
    pub asset_file_path: Option<PathBuf>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AssetSpecifications {
    pub textures: Vec<TextureSpecification>,
    pub triangle_meshes: Vec<TriangleMeshSpecification>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TextureSpecification {
    Texture {
        name: String,
        image_path: PathBuf,
        texture_config: TextureConfig,
        sampler_config: Option<SamplerConfig>,
    },
    Cubemap {
        name: String,
        right_image_path: PathBuf,
        left_image_path: PathBuf,
        top_image_path: PathBuf,
        bottom_image_path: PathBuf,
        front_image_path: PathBuf,
        back_image_path: PathBuf,
        texture_config: TextureConfig,
        sampler_config: Option<SamplerConfig>,
    },
    TextureArray {
        name: String,
        image_paths: Vec<PathBuf>,
        texture_config: TextureConfig,
        sampler_config: Option<SamplerConfig>,
    },
}

impl Assets {
    /// Creates a new empty asset container.
    pub fn new(
        config: AssetConfig,
        graphics_device: Arc<GraphicsDevice>,
        mipmapper_generator: Arc<MipmapperGenerator>,
    ) -> Self {
        Self {
            config,
            graphics_device,
            mipmapper_generator,
            textures: HashMap::default(),
            samplers: HashMap::default(),
        }
    }

    /// Returns the path to the folder where automatically computed lookup
    /// tables are be stored.
    pub fn lookup_table_dir(&self) -> &Path {
        &self.config.lookup_table_dir
    }

    /// Parses the asset file pointed to in the [`AssetConfig`] and loads all
    /// assets specified in the file.
    ///
    /// # Returns
    /// The parsed [`AssetSpecifications`].
    ///
    /// # Errors
    /// Returns an error if the asset file does not exist or is invalid.
    /// See also [`Self::load_specified_assets`].
    pub fn load_assets_specified_in_config(&mut self) -> Result<AssetSpecifications> {
        let Some(asset_file_path) = self.config.asset_file_path.as_ref() else {
            return Ok(AssetSpecifications::default());
        };
        let specifications = AssetSpecifications::from_ron_file(asset_file_path)?;
        self.load_specified_assets(&specifications)?;
        Ok(specifications)
    }

    /// Loads all assets in the given specifications.
    ///
    /// # Errors
    /// See [`Self::load_texture_from_path`],
    /// [`Self::load_cubemap_texture_from_paths`] and
    /// [`Self::load_texture_array_from_paths`].
    pub fn load_specified_assets(&mut self, specifications: &AssetSpecifications) -> Result<()> {
        for texture_specifcation in &specifications.textures {
            match texture_specifcation {
                TextureSpecification::Texture {
                    name,
                    image_path,
                    texture_config,
                    sampler_config,
                } => {
                    self.load_texture_from_path(
                        name,
                        image_path,
                        texture_config.clone(),
                        sampler_config.clone(),
                    )?;
                }
                TextureSpecification::Cubemap {
                    name,
                    right_image_path,
                    left_image_path,
                    top_image_path,
                    bottom_image_path,
                    front_image_path,
                    back_image_path,
                    texture_config,
                    sampler_config,
                } => {
                    self.load_cubemap_texture_from_paths(
                        name,
                        right_image_path,
                        left_image_path,
                        top_image_path,
                        bottom_image_path,
                        front_image_path,
                        back_image_path,
                        texture_config.clone(),
                        sampler_config.clone(),
                    )?;
                }
                TextureSpecification::TextureArray {
                    name,
                    image_paths,
                    texture_config,
                    sampler_config,
                } => {
                    self.load_texture_array_from_paths(
                        name,
                        image_paths,
                        texture_config.clone(),
                        sampler_config.clone(),
                    )?;
                }
            }
        }
        Ok(())
    }

    /// Loads the image file at the given path as a [`Texture`] and stores it
    /// under the given name.
    ///
    /// # Returns
    /// A [`Result`] with the [`TextureID`] (hashed from the name) assigned to
    /// the loaded texture.
    ///
    /// # Errors
    /// - Returns an error if a texture with the same name already has been
    ///   loaded.
    /// - See also [`Texture::from_path`](crate::gpu::texture::Texture::from_path).
    pub fn load_texture_from_path(
        &mut self,
        texture_name: impl ToString,
        image_path: impl AsRef<Path>,
        texture_config: TextureConfig,
        sampler_config: Option<SamplerConfig>,
    ) -> Result<TextureID> {
        let texture_name = texture_name.to_string();
        let image_path = image_path.as_ref();

        impact_log::debug!(
            "Loading texture `{}` from {}",
            &texture_name,
            image_path.display()
        );

        let texture_id = TextureID(hash32!(texture_name));

        match self.textures.entry(texture_id) {
            Entry::Vacant(entry) => {
                let sampler_id = sampler_config.as_ref().map(Into::into);

                let texture = entry.insert(Texture::from_path(
                    &self.graphics_device,
                    &self.mipmapper_generator,
                    image_path,
                    texture_config,
                    sampler_id,
                )?);

                if let (Some(sampler_id), Some(sampler_config)) =
                    (texture.sampler_id(), sampler_config)
                {
                    self.samplers
                        .entry(sampler_id)
                        .or_insert_with(|| Sampler::create(&self.graphics_device, sampler_config));
                }
                Ok(texture_id)
            }
            Entry::Occupied(_) => {
                bail!("A texture named `{}` is already loaded", texture_id);
            }
        }
    }

    /// Loads the cubemap face image files at the given paths as a cubemap
    /// [`Texture`] and stores it under the given name.
    ///
    /// # Returns
    /// A [`Result`] with the [`TextureID`] (hashed from the name) assigned to
    /// the loaded texture.
    ///
    /// # Errors
    /// - Returns an error if a texture with the same name already has been
    ///   loaded.
    /// - See also [`Texture::from_cubemap_image_paths`](crate::gpu::texture::Texture::from_cubemap_image_paths).
    pub fn load_cubemap_texture_from_paths<P: AsRef<Path>>(
        &mut self,
        texture_name: impl ToString,
        right_image_path: P,
        left_image_path: P,
        top_image_path: P,
        bottom_image_path: P,
        front_image_path: P,
        back_image_path: P,
        texture_config: TextureConfig,
        sampler_config: Option<SamplerConfig>,
    ) -> Result<TextureID> {
        let texture_name = texture_name.to_string();

        impact_log::debug!("Loading cubemap texture `{texture_name}`");

        let texture_id = TextureID(hash32!(texture_name));

        match self.textures.entry(texture_id) {
            Entry::Vacant(entry) => {
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

                if let (Some(sampler_id), Some(sampler_config)) =
                    (texture.sampler_id(), sampler_config)
                {
                    self.samplers
                        .entry(sampler_id)
                        .or_insert_with(|| Sampler::create(&self.graphics_device, sampler_config));
                }
                Ok(texture_id)
            }
            Entry::Occupied(_) => {
                bail!("A texture named `{}` is already loaded", texture_id);
            }
        }
    }

    /// Loads the image files at the given paths as an arrayed [`Texture`]
    /// and stores it under the given name.
    ///
    /// # Returns
    /// A [`Result`] with the [`TextureID`] (hashed from the name) assigned to
    /// the loaded texture.
    ///
    /// # Errors
    /// - Returns an error if a texture with the same name already has been
    ///   loaded.
    /// - See also [`Texture::array_from_image_paths`](crate::gpu::texture::Texture::array_from_image_paths).
    pub fn load_texture_array_from_paths<I, P>(
        &mut self,
        texture_name: impl AsRef<str>,
        image_paths: impl IntoIterator<IntoIter = I>,
        texture_config: TextureConfig,
        sampler_config: Option<SamplerConfig>,
    ) -> Result<TextureID>
    where
        I: ExactSizeIterator<Item = P> + Clone,
        P: AsRef<Path>,
    {
        let texture_name = texture_name.as_ref();

        impact_log::debug!("Loading texture array `{texture_name}`");

        let texture_id = TextureID(hash32!(texture_name));

        match self.textures.entry(texture_id) {
            Entry::Vacant(entry) => {
                let sampler_id = sampler_config.as_ref().map(Into::into);

                let texture = entry.insert(Texture::array_from_image_paths(
                    &self.graphics_device,
                    &self.mipmapper_generator,
                    image_paths,
                    texture_config,
                    sampler_id,
                    texture_name,
                )?);

                if let (Some(sampler_id), Some(sampler_config)) =
                    (texture.sampler_id(), sampler_config)
                {
                    self.samplers
                        .entry(sampler_id)
                        .or_insert_with(|| Sampler::create(&self.graphics_device, sampler_config));
                }
                Ok(texture_id)
            }
            Entry::Occupied(_) => {
                bail!("A texture named `{}` is already loaded", texture_id);
            }
        }
    }

    /// Loads the lookup table generated by the given function as a [`Texture`]
    /// and stores it under the given name. Also creates a sampler for the
    /// texture using the given sampler configuration.
    ///
    /// # Returns
    /// A [`Result`] with the [`TextureID`] (hashed from the name) assigned to
    /// the loaded texture.
    ///
    /// # Errors
    /// - Returns an error if a texture with the same name already has been
    ///   loaded.
    /// - See also [`Texture::from_lookup_table`](crate::gpu::texture::Texture::from_lookup_table).
    pub fn load_texture_from_generated_lookup_table<T: TexelType>(
        &mut self,
        texture_name: impl AsRef<str>,
        generate_table: impl Fn() -> Result<TextureLookupTable<T>>,
        sampler_config: SamplerConfig,
    ) -> Result<TextureID> {
        let texture_name = texture_name.as_ref();
        let texture_id = TextureID(hash32!(texture_name));

        match self.textures.entry(texture_id) {
            Entry::Vacant(entry) => {
                let sampler_id = (&sampler_config).into();

                entry.insert(Texture::from_lookup_table(
                    &self.graphics_device,
                    &generate_table()?,
                    texture_name,
                    Some(sampler_id),
                )?);

                self.samplers
                    .entry(sampler_id)
                    .or_insert_with(|| Sampler::create(&self.graphics_device, sampler_config));

                Ok(texture_id)
            }
            Entry::Occupied(_) => {
                bail!("A texture named `{}` is already loaded", texture_id);
            }
        }
    }

    /// Loads as a [`Texture`] the lookup table that is either taken from the
    /// file at the given path if it exists, otherwise it is computed with the
    /// given function and saved at the given path. The loaded texture is
    /// stored under the given name.  Also creates a sampler for the texture
    /// using the given sampler configuration.
    ///
    /// # Returns
    /// A [`Result`] with the [`TextureID`] (hashed from the name) assigned to
    /// the loaded texture.
    ///
    /// # Errors
    /// Returns an error if:
    /// - A texture with the same name already has been loaded.
    /// - The computed table can not be saved to file.
    /// - See also [`Texture::from_lookup_table`](crate::gpu::texture::Texture::from_lookup_table).
    pub fn load_texture_from_stored_or_computed_lookup_table<T>(
        &mut self,
        texture_name: impl AsRef<str>,
        table_file_path: impl AsRef<Path>,
        compute_table: impl Fn() -> TextureLookupTable<T>,
        sampler_config: SamplerConfig,
    ) -> Result<TextureID>
    where
        T: TexelType + Serialize + DeserializeOwned,
    {
        let table_file_path = table_file_path.as_ref();

        self.load_texture_from_generated_lookup_table(
            texture_name,
            || {
                texture::read_lookup_table_from_file(table_file_path).or_else(|_| {
                    let table = compute_table();
                    texture::save_lookup_table_to_file(&table, table_file_path)?;
                    Ok(table)
                })
            },
            sampler_config,
        )
    }
}

impl MaterialTextureProvider for Assets {
    fn get_texture(&self, texture_id: &TextureID) -> Option<&Texture> {
        self.textures.get(texture_id)
    }

    fn get_sampler(&self, sampler_id: &SamplerID) -> Option<&Sampler> {
        self.samplers.get(sampler_id)
    }
}

impl AssetConfig {
    /// Resolves all paths in the configuration by prepending the given root
    /// path to all paths.
    pub fn resolve_paths(&mut self, root_path: &Path) {
        self.lookup_table_dir = root_path.join(&self.lookup_table_dir);

        if let Some(asset_file_path) = self.asset_file_path.as_mut() {
            *asset_file_path = root_path.join(&asset_file_path);
        }
    }
}

impl Default for AssetConfig {
    fn default() -> Self {
        Self {
            lookup_table_dir: PathBuf::from("assets/lookup_tables"),
            asset_file_path: None,
        }
    }
}

impl AssetSpecifications {
    /// Parses the specifications from the RON file at the given path and
    /// resolves any specified paths.
    pub fn from_ron_file(file_path: impl AsRef<Path>) -> Result<Self> {
        let file_path = file_path.as_ref();
        let mut specs: Self = impact_io::parse_ron_file(file_path)?;
        if let Some(root_path) = file_path.parent() {
            specs.resolve_paths(root_path);
        }
        Ok(specs)
    }

    /// Resolves all paths in the specifications by prepending the given root
    /// path to all paths.
    fn resolve_paths(&mut self, root_path: &Path) {
        for specification in &mut self.textures {
            specification.resolve_paths(root_path);
        }
        for specification in &mut self.triangle_meshes {
            specification.resolve_paths(root_path);
        }
    }
}

impl TextureSpecification {
    /// Resolves all paths in the specification by prepending the given root
    /// path to all paths.
    pub fn resolve_paths(&mut self, root_path: &Path) {
        match self {
            Self::Texture { image_path, .. } => {
                *image_path = root_path.join(&image_path);
            }
            Self::Cubemap {
                right_image_path,
                left_image_path,
                top_image_path,
                bottom_image_path,
                front_image_path,
                back_image_path,
                ..
            } => {
                *right_image_path = root_path.join(&right_image_path);
                *left_image_path = root_path.join(&left_image_path);
                *top_image_path = root_path.join(&top_image_path);
                *bottom_image_path = root_path.join(&bottom_image_path);
                *front_image_path = root_path.join(&front_image_path);
                *back_image_path = root_path.join(&back_image_path);
            }
            Self::TextureArray { image_paths, .. } => {
                for image_path in image_paths {
                    *image_path = root_path.join(&image_path);
                }
            }
        }
    }
}
