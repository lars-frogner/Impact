//! Particle materials.

use crate::gpu_resource::ParticleMaterialGPUResources;
use anyhow::{Context, Result, bail};
use bytemuck::{Pod, Zeroable};
use impact_alloc::AVec;
use impact_containers::NoHashMap;
use impact_gpu::{
    bind_group_layout::BindGroupLayoutRegistry,
    device::GraphicsDevice,
    texture::{
        ColorSpace, SamplerConfig, TextureAddressingConfig, TextureConfig, TextureFilteringConfig,
    },
};
use impact_io::image::{Image, ImageMetadata, PixelFormat};
use impact_math::hash::Hash32;
use impact_texture::{
    ImageSource, ImageTextureSource, SamplerRegistry, TextureID, TextureRegistry,
    gpu_resource::{SamplerMap, TextureMap},
    processing::{FormatConversion, ImageProcessing, NormalMapFormat},
};
use std::{
    borrow::Cow,
    num::NonZeroU32,
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
    pub atlas_texture_idx: u32,
}

/// Registry containing the names and properties of all particle materials and a
/// handle to the normal and alpha texture atlas used by the materials.
#[derive(Clone, Debug)]
pub struct ParticleMaterialRegistry {
    name_lookup_table: NoHashMap<u32, ParticleMaterialID>,
    names: Vec<Cow<'static, str>>,
    fixed_material_properties: Vec<FixedParticleMaterialProperties>,
    normal_and_alpha_atlas_id: Option<TextureID>,
    n_atlas_textures_per_axis: u32,
}

/// Specific properties of a particle material that do not change across the
/// particle.
///
/// This struct will be passed directly to the GPU in a uniform buffer.
#[repr(C)]
#[derive(Clone, Copy, Debug, Zeroable, Pod)]
pub struct FixedParticleMaterialProperties {
    atlas_texture_u: f32,
    atlas_texture_v: f32,
    specular_reflectance: f32,
    roughness: f32,
    metalness: f32,
    emissive_luminance: f32,
    _padding: [u8; 8],
}

#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(default)
)]
#[derive(Clone, Debug, Default)]
pub struct ParticleMaterialConfig {
    /// Path to the RON file containing the specification of each particle
    /// material. If [`None`], a single particle material called "Default" will
    /// be defined.
    pub particle_materials_path: Option<PathBuf>,
    /// Information about the texture atlas to use for normals and alpha. If
    /// [`None`], a flat and opaque single-texture atlas will be used.
    pub normal_and_alpha_atlas: Option<NormalAndAlphaAtlas>,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct NormalAndAlphaAtlas {
    /// Path to the image to use as the atlas.
    pub path: PathBuf,
    /// Number of textures along each axis of the atlas.
    pub textures_per_axis: NonZeroU32,
    /// Format for the normal map component.
    #[cfg_attr(feature = "serde", serde(default))]
    pub normal_map_format: NormalMapFormat,
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
        particle_material_config: ParticleMaterialConfig,
    ) -> Result<Self> {
        let particle_materials = match particle_material_config.particle_materials_path {
            Some(file_path) => ParticleMaterialSpecifications::from_ron_file(file_path)?,
            None => ParticleMaterialSpecifications::default(),
        };
        Self::create(
            texture_registry,
            sampler_registry,
            particle_materials,
            particle_material_config.normal_and_alpha_atlas,
        )
    }

    /// Creates a registry with no particle materials.
    pub fn empty() -> Self {
        Self {
            name_lookup_table: NoHashMap::default(),
            names: Vec::new(),
            fixed_material_properties: Vec::new(),
            normal_and_alpha_atlas_id: None,
            n_atlas_textures_per_axis: 0,
        }
    }

    /// Creates a new particle material registry for the specified particle
    /// materials.
    ///
    /// # Errors
    /// Returns an error if:
    /// - A particle material uses an out-of-bounds atlas texture index.
    /// - There are duplicate names.
    ///
    /// See also
    /// [`load_declared_image_texture`](impact_texture::import::load_declared_image_texture).
    pub fn create(
        texture_registry: &mut TextureRegistry,
        sampler_registry: &mut SamplerRegistry,
        particle_materials: ParticleMaterialSpecifications,
        normal_and_alpha_atlas: Option<NormalAndAlphaAtlas>,
    ) -> Result<Self> {
        if particle_materials.0.is_empty() {
            return Ok(Self::empty());
        }

        let (
            names,
            fixed_material_properties,
            (normal_and_alpha_atlas_source, n_atlas_textures_per_axis, normal_map_format),
        ) = particle_materials.resolve(normal_and_alpha_atlas)?;

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

        let normal_and_alpha_atlas_id =
            TextureID::from_name("particle_normal_and_alpha_texture_atlas_id");

        let normal_map_processing = ImageProcessing {
            format_conversions: vec![FormatConversion::NormalMap {
                from: normal_map_format,
            }],
        };

        impact_texture::import::load_image_texture(
            texture_registry,
            sampler_registry,
            normal_and_alpha_atlas_id,
            ImageTextureSource::Single(normal_and_alpha_atlas_source),
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
        .context("Failed to load particle normal and alpha texture atlas")?;

        Ok(Self {
            name_lookup_table,
            names,
            fixed_material_properties,
            normal_and_alpha_atlas_id: Some(normal_and_alpha_atlas_id),
            n_atlas_textures_per_axis: n_atlas_textures_per_axis.get(),
        })
    }

    /// Returns the number of registered particle materials.
    pub fn n_particle_materials(&self) -> u32 {
        self.names.len() as u32
    }

    /// Returns the number of textures along each axis of the normal and alpha
    /// texture atlas.
    pub fn n_atlas_textures_per_axis(&self) -> u32 {
        self.n_atlas_textures_per_axis
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

    /// Returns the ID of the texture with the combined normal and alpha texture
    /// atlas for all registered particle materials, or [`None`] if there are no
    /// registered particle materials.
    pub fn normal_and_alpha_atlas_id(&self) -> Option<TextureID> {
        self.normal_and_alpha_atlas_id
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
        normal_and_alpha_atlas: Option<NormalAndAlphaAtlas>,
    ) -> Result<(
        Vec<Cow<'static, str>>,
        Vec<FixedParticleMaterialProperties>,
        (ImageSource, NonZeroU32, NormalMapFormat),
    )> {
        const FALLBACK_ATLAS_RESOLUTION: NonZeroU32 = NonZeroU32::new(256).unwrap();

        let (normal_and_alpha_atlas_source, n_atlas_textures_per_axis, normal_map_format) =
            if let Some(NormalAndAlphaAtlas {
                path,
                textures_per_axis,
                normal_map_format,
            }) = normal_and_alpha_atlas
            {
                (
                    ImageSource::File(path),
                    textures_per_axis,
                    normal_map_format,
                )
            } else {
                (
                    ImageSource::Bytes(create_flat_opaque_normal_and_alpha_image(
                        FALLBACK_ATLAS_RESOLUTION,
                    )),
                    NonZeroU32::new(1).unwrap(),
                    NormalMapFormat::default(),
                )
            };

        let atlas_texture_count = n_atlas_textures_per_axis.get().pow(2);

        let mut names = Vec::with_capacity(self.0.len());
        let mut fixed_material_properties = Vec::with_capacity(self.0.len());

        for ParticleMaterialSpecification {
            name,
            specular_reflectance,
            roughness,
            metalness,
            emissive_luminance,
            atlas_texture_idx,
        } in self.0
        {
            if atlas_texture_idx >= atlas_texture_count {
                bail!(
                    "Particle material `{name}` has texture index outside the atlas \
                     (which fits {atlas_texture_count} textures)"
                );
            }

            names.push(name);

            let (atlas_texture_u, atlas_texture_v) =
                atlas_texture_uv_from_idx(n_atlas_textures_per_axis, atlas_texture_idx);

            fixed_material_properties.push(FixedParticleMaterialProperties::new(
                atlas_texture_u,
                atlas_texture_v,
                specular_reflectance,
                roughness,
                metalness,
                emissive_luminance,
            ));
        }

        Ok((
            names,
            fixed_material_properties,
            (
                normal_and_alpha_atlas_source,
                n_atlas_textures_per_axis,
                normal_map_format,
            ),
        ))
    }
}

impl FixedParticleMaterialProperties {
    fn new(
        atlas_texture_u: f32,
        atlas_texture_v: f32,
        specular_reflectance: f32,
        roughness: f32,
        metalness: f32,
        emissive_luminance: f32,
    ) -> Self {
        Self {
            atlas_texture_u,
            atlas_texture_v,
            specular_reflectance,
            roughness,
            metalness,
            emissive_luminance,
            _padding: [0; 8],
        }
    }
}

impl ParticleMaterialSpecifications {
    /// Parses the specifications from the RON file at the given path.
    #[cfg(feature = "ron")]
    pub fn from_ron_file(file_path: impl AsRef<Path>) -> Result<Self> {
        let file_path = file_path.as_ref();
        impact_io::parse_ron_file(file_path).map(Self)
    }
}

impl Default for ParticleMaterialSpecifications {
    fn default() -> Self {
        Self(vec![ParticleMaterialSpecification {
            name: Cow::Borrowed("Default"),
            specular_reflectance: 0.02,
            roughness: 0.5,
            metalness: 0.0,
            emissive_luminance: 0.0,
            atlas_texture_idx: 0,
        }])
    }
}

impl ParticleMaterialConfig {
    /// Resolves all paths in the configuration by prepending the given root
    /// path to all paths.
    pub fn resolve_paths(&mut self, root_path: &Path) {
        if let Some(particle_materials_path) = self.particle_materials_path.as_mut() {
            *particle_materials_path = root_path.join(&particle_materials_path);
        }
        if let Some(normal_and_alpha_atlas) = self.normal_and_alpha_atlas.as_mut() {
            normal_and_alpha_atlas.path = root_path.join(&normal_and_alpha_atlas.path);
        }
    }
}

fn atlas_texture_uv_from_idx(
    n_atlas_textures_per_axis: NonZeroU32,
    atlas_texture_idx: u32,
) -> (f32, f32) {
    let n = n_atlas_textures_per_axis.get();
    let i = atlas_texture_idx % n;
    let j = atlas_texture_idx / n;
    (i as f32 / n as f32, j as f32 / n as f32)
}

fn create_flat_opaque_normal_and_alpha_image(texture_resolution: NonZeroU32) -> Image {
    let pixel_count = texture_resolution.get().pow(2) as usize;

    let meta = ImageMetadata {
        width: texture_resolution.get(),
        height: texture_resolution.get(),
        pixel_format: PixelFormat::Rgba8,
    };

    let pixel = [128, 128, 255, 255]; // Corresponds to normal vector (0.0, 0.0, 1.0) and alpha 1.0

    let mut data = AVec::with_capacity(pixel.len() * pixel_count);
    for _ in 0..pixel_count {
        data.extend_from_slice(&pixel);
    }

    Image { meta, data }
}
