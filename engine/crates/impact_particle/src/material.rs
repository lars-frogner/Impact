//! Particle materials.

use crate::gpu_resource::ParticleMaterialGPUResources;
use anyhow::{Context, Result, anyhow, bail};
use bytemuck::{Pod, Zeroable};
use impact_alloc::arena::ArenaPool;
use impact_containers::{HashMap, NoHashMap};
use impact_gpu::{
    bind_group_layout::BindGroupLayoutRegistry,
    device::GraphicsDevice,
    texture::{
        ColorSpace, SamplerConfig, TextureAddressingConfig, TextureConfig, TextureFilteringConfig,
    },
};
use impact_io::image::PixelFormat;
use impact_math::hash::{Hash32, Hash64};
use impact_texture::{
    ImageSource, ImageTextureSource, SamplerRegistry, TextureArrayUsage, TextureID,
    TextureRegistry,
    gpu_resource::{SamplerMap, TextureMap},
    processing::{FormatConversion, ImageProcessing, NormalMapFormat},
};
use std::{
    borrow::Cow,
    path::{Path, PathBuf},
};

/// Identifier for a material that determines the visual properties of a
/// particle.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Zeroable, Pod)]
pub struct ParticleMaterialID(u32);

#[derive(Clone, Debug)]
pub struct ParticleMaterialSpecifications(pub Vec<ParticleMaterialSpecification>);

/// Specifies all relevant aspects of a particle material.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct ParticleMaterialSpecification {
    pub name: Cow<'static, str>,
    pub specular_reflectance: f32,
    pub roughness: f32,
    pub metalness: f32,
    pub emissive_luminance: f32,
    pub normal_and_alpha_texture: NormalAndAlphaTexture,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct NormalAndAlphaTexture {
    /// Path to the image containing a normal map in the RGB channels and an
    /// alpha map in the alpha channel.
    pub path: PathBuf,
    /// Format for the normal map.
    #[cfg_attr(feature = "serde", serde(default))]
    pub normal_map_format: NormalMapFormat,
}

/// Registry containing the names and properties of all particle materials and a
/// handle to the normal and alpha texture array used by the materials.
#[derive(Clone, Debug)]
pub struct ParticleMaterialRegistry {
    name_lookup_table: NoHashMap<u32, ParticleMaterialID>,
    names: Vec<Cow<'static, str>>,
    fixed_material_properties: Vec<FixedParticleMaterialProperties>,
    normal_and_alpha_texture_array_id: Option<TextureID>,
    noise_texture_array_id: Option<TextureID>,
    noise_texture_size: [u32; 3],
}

/// Specific properties of a particle material that do not change across the
/// particle.
///
/// This struct will be passed directly to the GPU in a uniform buffer.
#[repr(C)]
#[derive(Clone, Copy, Debug, Zeroable, Pod)]
pub struct FixedParticleMaterialProperties {
    specular_reflectance: f32,
    roughness: f32,
    metalness: f32,
    emissive_luminance: f32,
    normal_and_alpha_texture_idx: u32,
    _padding: [u8; 12],
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct ParticleMaterialConfig {
    /// Path to the RON file containing the specification of each particle
    /// material.
    pub particle_materials_path: PathBuf,
    /// Identifies a sequence of 8-bit grayscale noise texture images to use for
    /// stochastic transparency. An appropriate blue noise image sequence can be
    /// downloaded from <https://github.com/NVIDIA-RTX/STBN>.
    pub noise_texture_array: NoiseTextureArray,
}

/// Identifies a sequence of noise texture images to use for stochastic
/// transparency.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct NoiseTextureArray {
    /// Each image in the sequence must have a file name of the form
    /// `<base_name>_<idx>.<extension>`, where `idx` ranges from `0` to
    /// `count-1`. This parameter specifies the full path to one of the images
    /// except that the `_<idx>` part must be omitted.
    pub base_path: PathBuf,
    /// The number of images in the sequence.
    pub count: u32,
}

impl ParticleMaterialID {
    /// Converts the given index into a [`ParticleMaterialID`].
    pub fn from_idx(idx: usize) -> Self {
        Self(u32::try_from(idx).unwrap())
    }

    /// Returns the index of the particle material.
    pub const fn idx(&self) -> usize {
        self.0 as usize
    }

    /// Returns the index of the particle material as a GPU immediate value.
    pub const fn idx_immediate(&self) -> u32 {
        self.idx() as u32
    }
}

impl ParticleMaterialRegistry {
    /// Creates a new particle material registry based on the given
    /// configuration parameters.
    #[cfg(feature = "ron")]
    pub fn from_config(
        texture_registry: &mut TextureRegistry,
        sampler_registry: &mut SamplerRegistry,
        particle_material_config: Option<ParticleMaterialConfig>,
    ) -> Result<Self> {
        let Some(config) = particle_material_config else {
            return Ok(Self::empty());
        };
        let particle_materials =
            ParticleMaterialSpecifications::from_ron_file(&config.particle_materials_path)?;
        Self::create(
            texture_registry,
            sampler_registry,
            particle_materials,
            config.noise_texture_array,
        )
    }

    /// Creates a registry with no particle materials.
    pub fn empty() -> Self {
        Self {
            name_lookup_table: NoHashMap::default(),
            names: Vec::new(),
            fixed_material_properties: Vec::new(),
            normal_and_alpha_texture_array_id: None,
            noise_texture_array_id: None,
            noise_texture_size: [0; 3],
        }
    }

    /// Creates a new particle material registry for the specified particle
    /// materials.
    ///
    /// # Errors
    /// Returns an error if:
    /// - There are duplicate names.
    /// - The normal and alpha textures have different normal map formats.
    /// - The noise image base path has no file name.
    /// - The noise image pixel format is invalid.
    ///
    /// See also
    /// [`load_declared_image_texture`](impact_texture::import::load_declared_image_texture).
    pub fn create(
        texture_registry: &mut TextureRegistry,
        sampler_registry: &mut SamplerRegistry,
        particle_materials: ParticleMaterialSpecifications,
        noise_texture_array: NoiseTextureArray,
    ) -> Result<Self> {
        if particle_materials.0.is_empty() {
            return Ok(Self::empty());
        }

        let (names, fixed_material_properties, normal_and_alpha_texture_sources, normal_map_format) =
            particle_materials.resolve()?;

        let name_lookup_table: NoHashMap<_, _> = names
            .iter()
            .enumerate()
            .map(|(idx, name)| {
                (
                    Hash32::from_str(name).into(),
                    ParticleMaterialID::from_idx(idx),
                )
            })
            .collect();

        if name_lookup_table.len() != names.len() {
            bail!("Duplicate particle material names in registry");
        }

        let normal_and_alpha_texture_array_id =
            TextureID::from_name("particle_normal_and_alpha_texture_array");

        let normal_map_processing = ImageProcessing {
            format_conversions: vec![FormatConversion::NormalMap {
                from: normal_map_format,
            }],
        };

        impact_texture::import::load_image_texture(
            texture_registry,
            sampler_registry,
            normal_and_alpha_texture_array_id,
            ImageTextureSource::Array {
                sources: normal_and_alpha_texture_sources,
                usage: TextureArrayUsage::Generic,
            },
            TextureConfig {
                color_space: ColorSpace::Linear,
                max_mip_level_count: None,
            },
            Some(SamplerConfig {
                addressing: TextureAddressingConfig::Clamped,
                filtering: TextureFilteringConfig::Basic,
            }),
            normal_map_processing,
        )
        .context("Failed to load particle normal and alpha texture array")?;

        let noise_texture_array_id = TextureID::from_name("particle_noise_texture_array");
        let noise_texture_sources = noise_texture_array.create_image_sources()?;

        let noise_image_metadata = impact_texture::import::load_image_texture(
            texture_registry,
            sampler_registry,
            noise_texture_array_id,
            ImageTextureSource::Array {
                sources: noise_texture_sources,
                usage: TextureArrayUsage::Generic,
            },
            TextureConfig {
                color_space: ColorSpace::Linear,
                max_mip_level_count: Some(1),
            },
            None,
            ImageProcessing::none(),
        )
        .context("Failed to load noise texture array for particles")?;

        if noise_image_metadata.pixel_format != PixelFormat::Luma8 {
            bail!("Noise image pixel format must be 8-bit grayscale");
        }

        let noise_texture_size = [
            noise_image_metadata.width,
            noise_image_metadata.height,
            noise_texture_array.count,
        ];

        Ok(Self {
            name_lookup_table,
            names,
            fixed_material_properties,
            normal_and_alpha_texture_array_id: Some(normal_and_alpha_texture_array_id),
            noise_texture_array_id: Some(noise_texture_array_id),
            noise_texture_size,
        })
    }

    /// Returns the number of registered particle materials.
    pub fn n_particle_materials(&self) -> u32 {
        self.names.len() as u32
    }

    /// Returns the particle material with the given name, or [`None`] if no
    /// particle material with the given name has been registered.
    pub fn particle_material_for_name(&self, name: &str) -> Option<ParticleMaterialID> {
        self.particle_material_for_name_hash(Hash32::from_str(name))
    }

    /// Returns the particle material with the given name hash, or [`None`] if
    /// no particle material with the given name has been registered.
    pub fn particle_material_for_name_hash(&self, name_hash: Hash32) -> Option<ParticleMaterialID> {
        self.name_lookup_table.get(&u32::from(name_hash)).copied()
    }

    /// Returns the name of the given particle material.
    ///
    /// # Panics
    /// If the particle material is not present in the registry.
    pub fn name(&self, particle_material: ParticleMaterialID) -> &str {
        self.get_name(particle_material)
            .expect("Particle material not present in registry")
    }

    /// Returns the name of the given particle material, or [`None`] if the
    /// particle material is not present in the registry.
    pub fn get_name(&self, particle_material: ParticleMaterialID) -> Option<&str> {
        self.names
            .get(particle_material.idx())
            .map(|name| name.as_ref())
    }

    /// Returns the slice of fixed properties for all registered particle
    /// materials.
    pub fn fixed_material_properties(&self) -> &[FixedParticleMaterialProperties] {
        &self.fixed_material_properties
    }

    /// Returns the ID of the texture array with the normal and alpha textures
    /// for all registered particle materials, or [`None`] if there are no
    /// registered particle materials.
    pub fn normal_and_alpha_texture_array_id(&self) -> Option<TextureID> {
        self.normal_and_alpha_texture_array_id
    }

    /// Returns the ID of the texture array with the noise textures for
    /// stochastic transparency, or [`None`] if there are no registered particle
    /// materials.
    pub fn noise_texture_array_id(&self) -> Option<TextureID> {
        self.noise_texture_array_id
    }

    /// Returns the `[width, height, array_layers]` for the noise texture array.
    pub fn noise_texture_size(&self) -> [u32; 3] {
        self.noise_texture_size
    }

    /// Performs any required updates for keeping the given particle material
    /// GPU resources in sync with the particle material registry.
    pub fn sync_gpu_resources(
        &self,
        graphics_device: &GraphicsDevice,
        textures: &TextureMap,
        samplers: &SamplerMap,
        bind_group_layout_registry: &BindGroupLayoutRegistry,
        particle_material_resource_manager: &mut Option<ParticleMaterialGPUResources>,
    ) -> Result<()> {
        if particle_material_resource_manager.is_none() && self.n_particle_materials() > 0 {
            *particle_material_resource_manager = Some(
                ParticleMaterialGPUResources::for_particle_material_registry(
                    graphics_device,
                    textures,
                    samplers,
                    self,
                    bind_group_layout_registry,
                )?,
            );
        }
        Ok(())
    }
}

impl ParticleMaterialSpecifications {
    #[allow(clippy::type_complexity)]
    fn resolve(
        self,
    ) -> Result<(
        Vec<Cow<'static, str>>,
        Vec<FixedParticleMaterialProperties>,
        Vec<ImageSource>,
        NormalMapFormat,
    )> {
        let mut names = Vec::with_capacity(self.0.len());
        let mut fixed_material_properties = Vec::with_capacity(self.0.len());
        let mut normal_and_alpha_texture_sources = Vec::with_capacity(self.0.len());
        let mut normal_map_format = None;

        let arena = ArenaPool::get_arena();
        let mut normal_and_alpha_texture_source_indices =
            HashMap::with_capacity_and_hasher_in(self.0.len(), Default::default(), &arena);

        for ParticleMaterialSpecification {
            name,
            specular_reflectance,
            roughness,
            metalness,
            emissive_luminance,
            normal_and_alpha_texture,
        } in self.0
        {
            if let Some(existing_format) = normal_map_format
                && normal_and_alpha_texture.normal_map_format != existing_format
            {
                bail!("Mixed normal map formats for particle materials is not supported");
            } else {
                normal_map_format = Some(normal_and_alpha_texture.normal_map_format);
            }

            let normal_and_alpha_texture_idx = *normal_and_alpha_texture_source_indices
                .entry(Hash64::from_str(
                    &normal_and_alpha_texture.path.to_string_lossy(),
                ))
                .or_insert_with(|| {
                    let idx = normal_and_alpha_texture_sources.len();
                    normal_and_alpha_texture_sources
                        .push(ImageSource::File(normal_and_alpha_texture.path));
                    idx
                });

            names.push(name);

            fixed_material_properties.push(FixedParticleMaterialProperties::new(
                specular_reflectance,
                roughness,
                metalness,
                emissive_luminance,
                normal_and_alpha_texture_idx as u32,
            ));
        }

        Ok((
            names,
            fixed_material_properties,
            normal_and_alpha_texture_sources,
            normal_map_format.unwrap(),
        ))
    }
}

impl FixedParticleMaterialProperties {
    fn new(
        specular_reflectance: f32,
        roughness: f32,
        metalness: f32,
        emissive_luminance: f32,
        normal_and_alpha_texture_idx: u32,
    ) -> Self {
        Self {
            specular_reflectance,
            roughness,
            metalness,
            emissive_luminance,
            normal_and_alpha_texture_idx,
            _padding: [0; 12],
        }
    }
}

impl ParticleMaterialSpecifications {
    /// Parses the specifications from the RON file at the given path.
    #[cfg(feature = "ron")]
    pub fn from_ron_file(file_path: impl AsRef<Path>) -> Result<Self> {
        let file_path = file_path.as_ref();
        let mut specs = Self(impact_io::parse_ron_file(file_path)?);
        if let Some(root_path) = file_path.parent() {
            specs.resolve_paths(root_path);
        }
        Ok(specs)
    }

    /// Resolves all paths in the specifications by prepending the given root
    /// path to all paths.
    #[cfg(feature = "ron")]
    fn resolve_paths(&mut self, root_path: &Path) {
        for specification in &mut self.0 {
            specification.resolve_paths(root_path);
        }
    }
}

impl ParticleMaterialSpecification {
    /// Resolves all paths in the specification by prepending the given root
    /// path to all paths.
    pub fn resolve_paths(&mut self, root_path: &Path) {
        self.normal_and_alpha_texture.path = root_path.join(&self.normal_and_alpha_texture.path);
    }
}

impl ParticleMaterialConfig {
    /// Resolves all paths in the configuration by prepending the given root
    /// path to all paths.
    pub fn resolve_paths(&mut self, root_path: &Path) {
        self.particle_materials_path = root_path.join(&self.particle_materials_path);
        self.noise_texture_array.base_path = root_path.join(&self.noise_texture_array.base_path);
    }
}

impl NoiseTextureArray {
    fn create_image_sources(&self) -> Result<Vec<ImageSource>> {
        let stem = self
            .base_path
            .file_stem()
            .ok_or_else(|| anyhow!("Missing file name for noise texture array base path"))?
            .display();

        let extension = self.base_path.extension();
        let dot = if extension.is_some() { "." } else { "" };
        let extension = extension.unwrap_or_default().display();

        let get_image_path = |idx: u32| {
            self.base_path
                .with_file_name(format!("{stem}_{idx}{dot}{extension}"))
        };

        let sources = (0..self.count)
            .map(|idx| ImageSource::File(get_image_path(idx)))
            .collect();

        Ok(sources)
    }
}
