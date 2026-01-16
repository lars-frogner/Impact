//! Voxel types and their properties.

use crate::gpu_resource::VoxelMaterialGPUResources;
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
use impact_math::{
    hash::Hash32,
    vector::{Vector3C, Vector4C},
};
use impact_texture::{
    ImageSource, ImageTextureSource, SamplerRegistry, TextureArrayUsage, TextureID,
    TextureRegistry,
    gpu_resource::{SamplerMap, TextureMap},
    processing::{FormatConversion, ImageProcessing, NormalMapFormat},
};
use std::{
    borrow::Cow,
    num::NonZeroU32,
    path::{Path, PathBuf},
};

/// A type identifier that determines all the properties of a voxel.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Zeroable, Pod)]
pub struct VoxelType(u8);

#[derive(Clone, Debug)]
pub struct VoxelTypeSpecifications(pub Vec<VoxelTypeSpecification>);

/// Specifies all relevant aspects of a voxel type.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct VoxelTypeSpecification {
    pub name: Cow<'static, str>,
    pub mass_density: f32,
    pub color: VoxelColor,
    pub specular_reflectance: f32,
    pub roughness: VoxelRoughness,
    pub metalness: f32,
    pub emissive_luminance: f32,
    pub normal_map: Option<VoxelNormalMap>,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub enum VoxelColor {
    Uniform(RBGColor),
    Textured(PathBuf),
}

pub type RBGColor = Vector3C;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub enum VoxelRoughness {
    Uniform(f32),
    Textured { path: PathBuf, scale_factor: f32 },
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct VoxelNormalMap {
    path: PathBuf,
    #[cfg_attr(feature = "serde", serde(default))]
    format: NormalMapFormat,
}

/// Registry containing the names and properties of all voxel types.
#[derive(Clone, Debug)]
pub struct VoxelTypeRegistry {
    name_lookup_table: NoHashMap<u32, VoxelType>,
    names: Vec<Cow<'static, str>>,
    mass_densities: Vec<f32>,
    fixed_material_properties: Vec<FixedVoxelMaterialProperties>,
    color_texture_array_id: Option<TextureID>,
    roughness_texture_array_id: Option<TextureID>,
    normal_texture_array_id: Option<TextureID>,
}

/// Specific properties of a voxel material that do not change with position.
#[repr(C)]
#[derive(Clone, Copy, Debug, Zeroable, Pod)]
pub struct FixedVoxelMaterialProperties {
    properties: Vector4C,
}

impl VoxelType {
    /// Creates a dummy voxel type that can not be present in any registry.
    pub const fn dummy() -> Self {
        Self(255)
    }

    /// Converts the given `u8` index into a [`VoxelType`].
    pub const fn from_idx_u8(idx: u8) -> Self {
        Self(idx)
    }

    /// Converts the given index into a [`VoxelType`].
    ///
    /// # Panics
    /// If `idx >= 256`.
    pub fn from_idx(idx: usize) -> Self {
        Self::from_idx_u8(u8::try_from(idx).unwrap())
    }

    /// Returns the index of the voxel type as a `u8`.
    pub const fn idx_u8(&self) -> u8 {
        self.0
    }

    /// Returns the index of the voxel type.
    pub const fn idx(&self) -> usize {
        self.0 as usize
    }
}

impl VoxelTypeRegistry {
    /// The maximum number of voxel types that can be registered.
    pub const fn max_n_voxel_types() -> usize {
        255
    }

    /// Creates a new voxel type registry based on the given configuration
    /// parameters.
    #[cfg(feature = "ron")]
    pub fn from_config(
        texture_registry: &mut TextureRegistry,
        sampler_registry: &mut SamplerRegistry,
        voxel_config: crate::VoxelConfig,
    ) -> anyhow::Result<Self> {
        let voxel_types = match voxel_config.voxel_types_path {
            Some(file_path) => VoxelTypeSpecifications::from_ron_file(file_path)?,
            None => VoxelTypeSpecifications::default(),
        };
        Self::create(
            texture_registry,
            sampler_registry,
            voxel_config.texture_resolution,
            voxel_types,
        )
    }

    /// Reads the RON (Rusty Object Notation) file at the given path and
    /// deserializes it into a [`VoxelTypeSpecifications`] object that is used
    /// to create a new voxel type registry.
    #[cfg(feature = "ron")]
    pub fn from_voxel_type_ron_file(
        texture_registry: &mut TextureRegistry,
        sampler_registry: &mut SamplerRegistry,
        texture_resolution: NonZeroU32,
        file_path: impl AsRef<Path>,
    ) -> Result<Self> {
        let voxel_types = VoxelTypeSpecifications::from_ron_file(file_path)?;
        Self::create(
            texture_registry,
            sampler_registry,
            texture_resolution,
            voxel_types,
        )
    }

    pub fn empty() -> Self {
        Self {
            name_lookup_table: NoHashMap::default(),
            names: Vec::new(),
            mass_densities: Vec::new(),
            fixed_material_properties: Vec::new(),
            color_texture_array_id: None,
            roughness_texture_array_id: None,
            normal_texture_array_id: None,
        }
    }

    /// Creates a new voxel type registry for the specified voxel types.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The number of voxel types is not smaller than
    ///   [`Self::max_n_voxel_types`].
    /// - There are duplicate names.
    ///
    /// See also
    /// [`load_declared_image_texture`](impact_texture::import::load_declared_image_texture).
    pub fn create(
        texture_registry: &mut TextureRegistry,
        sampler_registry: &mut SamplerRegistry,
        texture_resolution: NonZeroU32,
        voxel_types: VoxelTypeSpecifications,
    ) -> Result<Self> {
        if voxel_types.0.is_empty() {
            return Ok(Self::empty());
        }

        voxel_types.validate()?;

        let (
            names,
            mass_densities,
            fixed_material_properties,
            color_texture_sources,
            roughness_texture_sources,
            (normal_texture_sources, normal_map_format),
        ) = voxel_types.resolve(texture_resolution)?;

        let name_lookup_table: NoHashMap<_, _> = names
            .iter()
            .enumerate()
            .map(|(idx, name)| (Hash32::from_str(name).into(), VoxelType::from_idx(idx)))
            .collect();

        if name_lookup_table.len() != names.len() {
            bail!("Duplicate voxel type names in registry");
        }

        let color_texture_array_id = TextureID::from_name("voxel_color_texture_array");
        let roughness_texture_array_id = TextureID::from_name("voxel_roughness_texture_array");
        let normal_texture_array_id = TextureID::from_name("voxel_normal_texture_array");

        impact_texture::import::load_image_texture(
            texture_registry,
            sampler_registry,
            color_texture_array_id,
            ImageTextureSource::Array {
                sources: color_texture_sources,
                usage: TextureArrayUsage::Generic,
            },
            TextureConfig {
                color_space: ColorSpace::Srgb,
                max_mip_level_count: None,
            },
            Some(SamplerConfig {
                addressing: TextureAddressingConfig::Repeating,
                filtering: TextureFilteringConfig::Basic,
            }),
            ImageProcessing::none(),
        )
        .context("Failed to load voxel color texture array")?;

        impact_texture::import::load_image_texture(
            texture_registry,
            sampler_registry,
            roughness_texture_array_id,
            ImageTextureSource::Array {
                sources: roughness_texture_sources,
                usage: TextureArrayUsage::Generic,
            },
            TextureConfig {
                color_space: ColorSpace::Linear,
                max_mip_level_count: None,
            },
            None,
            ImageProcessing::none(),
        )
        .context("Failed to load voxel roughness texture array")?;

        let normal_map_processing =
            normal_map_format.map_or_else(ImageProcessing::none, |format| ImageProcessing {
                format_conversions: vec![FormatConversion::NormalMap { from: format }],
            });

        impact_texture::import::load_image_texture(
            texture_registry,
            sampler_registry,
            normal_texture_array_id,
            ImageTextureSource::Array {
                sources: normal_texture_sources,
                usage: TextureArrayUsage::Generic,
            },
            TextureConfig {
                color_space: ColorSpace::Linear,
                max_mip_level_count: None,
            },
            None,
            normal_map_processing,
        )
        .context("Failed to load voxel normal texture array")?;

        Ok(Self {
            name_lookup_table,
            names,
            mass_densities,
            fixed_material_properties,
            color_texture_array_id: Some(color_texture_array_id),
            roughness_texture_array_id: Some(roughness_texture_array_id),
            normal_texture_array_id: Some(normal_texture_array_id),
        })
    }

    /// Returns the number of registered voxel types.
    pub fn n_voxel_types(&self) -> usize {
        self.names.len()
    }

    /// Returns the voxel type with the given name, or [`None`] if no voxel type
    /// with the given name has been registered.
    pub fn voxel_type_for_name(&self, name: &str) -> Option<VoxelType> {
        self.voxel_type_for_name_hash(Hash32::from_str(name))
    }

    /// Returns the voxel type with the given name hash, or [`None`] if no voxel
    /// type with the given name has been registered.
    pub fn voxel_type_for_name_hash(&self, name_hash: Hash32) -> Option<VoxelType> {
        self.name_lookup_table.get(&u32::from(name_hash)).copied()
    }

    /// Returns the name of the given voxel type.
    ///
    /// # Panics
    /// If the voxel type is not present in the registry.
    pub fn name(&self, voxel_type: VoxelType) -> &str {
        self.get_name(voxel_type)
            .expect("Voxel type not present in registry")
    }

    /// Returns the name of the given voxel type, or [`None`] if the voxel type
    /// is not present in the registry.
    pub fn get_name(&self, voxel_type: VoxelType) -> Option<&str> {
        self.names.get(voxel_type.idx()).map(|name| name.as_ref())
    }

    /// Returns the slice of mass densities for all registered voxel types.
    pub fn mass_densities(&self) -> &[f32] {
        &self.mass_densities
    }

    /// Returns the slice of fixed material properties for all registered voxel
    /// types.
    pub fn fixed_material_properties(&self) -> &[FixedVoxelMaterialProperties] {
        &self.fixed_material_properties
    }

    /// Returns the ID of the texture array with the color textures of all
    /// registered voxel types, or [`None`] if there are no registered voxel
    /// types.
    pub fn color_texture_array_id(&self) -> Option<TextureID> {
        self.color_texture_array_id
    }

    /// Returns the ID of the texture array with the roughness textures of all
    /// registered voxel types, or [`None`] if there are no registered voxel
    /// types.
    pub fn roughness_texture_array_id(&self) -> Option<TextureID> {
        self.roughness_texture_array_id
    }

    /// Returns the ID of the texture array with the normal textures of all
    /// registered voxel types, or [`None`] if there are no registered voxel
    /// types.
    pub fn normal_texture_array_id(&self) -> Option<TextureID> {
        self.normal_texture_array_id
    }

    /// Performs any required updates for keeping the given voxel material GPU
    /// resources in sync with the voxel type registry.
    pub fn sync_material_gpu_resources(
        &self,
        graphics_device: &GraphicsDevice,
        textures: &TextureMap,
        samplers: &SamplerMap,
        bind_group_layout_registry: &BindGroupLayoutRegistry,
        voxel_material_resource_manager: &mut Option<VoxelMaterialGPUResources>,
    ) -> Result<()> {
        if voxel_material_resource_manager.is_none() && self.n_voxel_types() > 0 {
            *voxel_material_resource_manager =
                Some(VoxelMaterialGPUResources::for_voxel_type_registry(
                    graphics_device,
                    textures,
                    samplers,
                    self,
                    bind_group_layout_registry,
                )?);
        }
        Ok(())
    }
}

impl VoxelTypeSpecifications {
    fn validate(&self) -> Result<()> {
        if self.0.len() >= VoxelTypeRegistry::max_n_voxel_types() {
            bail!("Too many voxel types for registry");
        }
        Ok(())
    }

    #[allow(clippy::type_complexity)]
    fn resolve(
        self,
        texture_resolution: NonZeroU32,
    ) -> Result<(
        Vec<Cow<'static, str>>,
        Vec<f32>,
        Vec<FixedVoxelMaterialProperties>,
        Vec<ImageSource>,
        Vec<ImageSource>,
        (Vec<ImageSource>, Option<NormalMapFormat>),
    )> {
        let mut names = Vec::with_capacity(self.0.len());
        let mut mass_densities = Vec::with_capacity(self.0.len());
        let mut fixed_material_properties = Vec::with_capacity(self.0.len());
        let mut color_texture_sources = Vec::with_capacity(self.0.len());
        let mut roughness_texture_sources = Vec::with_capacity(self.0.len());
        let mut normal_texture_sources = Vec::with_capacity(self.0.len());
        let mut normal_map_format = None;

        for VoxelTypeSpecification {
            name,
            mass_density,
            color,
            specular_reflectance,
            roughness,
            metalness,
            emissive_luminance,
            normal_map,
        } in self.0
        {
            let color_texture_source = match color {
                VoxelColor::Textured(path) => ImageSource::File(path),
                VoxelColor::Uniform(color) => {
                    ImageSource::Bytes(create_uniform_color_image(texture_resolution, color))
                }
            };

            let (roughness_texture_source, roughness_scale) = match roughness {
                VoxelRoughness::Textured { path, scale_factor } => {
                    (ImageSource::File(path), scale_factor)
                }
                VoxelRoughness::Uniform(roughness) => (
                    ImageSource::Bytes(create_uniform_roughness_image(
                        texture_resolution,
                        roughness,
                    )),
                    1.0,
                ),
            };

            let normal_texture_source = match normal_map {
                Some(VoxelNormalMap { path, format }) => {
                    if let Some(existing_format) = normal_map_format
                        && format != existing_format
                    {
                        bail!("Mixed normal map formats for voxel types is not supported");
                    } else {
                        normal_map_format = Some(format);
                    }
                    ImageSource::File(path)
                }
                None => ImageSource::Bytes(create_identity_normal_image(texture_resolution)),
            };

            names.push(name);
            mass_densities.push(mass_density);
            fixed_material_properties.push(FixedVoxelMaterialProperties::new(
                specular_reflectance,
                roughness_scale,
                metalness,
                emissive_luminance,
            ));
            color_texture_sources.push(color_texture_source);
            roughness_texture_sources.push(roughness_texture_source);
            normal_texture_sources.push(normal_texture_source);
        }

        Ok((
            names,
            mass_densities,
            fixed_material_properties,
            color_texture_sources,
            roughness_texture_sources,
            (normal_texture_sources, normal_map_format),
        ))
    }
}

impl FixedVoxelMaterialProperties {
    /// Combines the given fixed properties for a voxel material.
    pub fn new(
        specular_reflectance: f32,
        roughness_scale: f32,
        metalness: f32,
        emissive_luminance: f32,
    ) -> Self {
        Self {
            properties: Vector4C::new(
                specular_reflectance,
                roughness_scale,
                metalness,
                emissive_luminance,
            ),
        }
    }
}

impl Default for FixedVoxelMaterialProperties {
    fn default() -> Self {
        Self::new(0.5, 0.5, 0.0, 0.0)
    }
}

impl VoxelTypeSpecifications {
    /// Parses the specifications from the RON file at the given path and
    /// resolves any specified paths.
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

impl Default for VoxelTypeSpecifications {
    fn default() -> Self {
        Self(vec![VoxelTypeSpecification {
            name: Cow::Borrowed("Default"),
            mass_density: 1.0,
            color: VoxelColor::Uniform(Vector3C::new(0.9, 0.9, 0.9)),
            specular_reflectance: 0.02,
            roughness: VoxelRoughness::Uniform(0.5),
            metalness: 0.0,
            emissive_luminance: 0.0,
            normal_map: None,
        }])
    }
}

impl VoxelTypeSpecification {
    /// Resolves all paths in the specification by prepending the given root
    /// path to all paths.
    pub fn resolve_paths(&mut self, root_path: &Path) {
        if let VoxelColor::Textured(path) = &mut self.color {
            *path = root_path.join(&path);
        }
        if let VoxelRoughness::Textured { path, .. } = &mut self.roughness {
            *path = root_path.join(&path);
        }
        if let Some(VoxelNormalMap { path, .. }) = &mut self.normal_map {
            *path = root_path.join(&path);
        }
    }
}

fn create_uniform_color_image(texture_resolution: NonZeroU32, color: RBGColor) -> Image {
    let pixel_count = texture_resolution.get().pow(2) as usize;

    let meta = ImageMetadata {
        width: texture_resolution.get(),
        height: texture_resolution.get(),
        pixel_format: PixelFormat::Rgba8,
    };

    let pixel = [
        float_to_u8(color.x()),
        float_to_u8(color.y()),
        float_to_u8(color.z()),
        255,
    ];

    let mut data = AVec::with_capacity(pixel.len() * pixel_count);
    for _ in 0..pixel_count {
        data.extend_from_slice(&pixel);
    }

    Image { meta, data }
}

fn create_uniform_roughness_image(texture_resolution: NonZeroU32, roughness: f32) -> Image {
    let pixel_count = texture_resolution.get().pow(2) as usize;

    let meta = ImageMetadata {
        width: texture_resolution.get(),
        height: texture_resolution.get(),
        pixel_format: PixelFormat::Luma8,
    };

    let mut data = AVec::new();
    data.resize(pixel_count, float_to_u8(roughness));

    Image { meta, data }
}

fn create_identity_normal_image(texture_resolution: NonZeroU32) -> Image {
    let pixel_count = texture_resolution.get().pow(2) as usize;

    let meta = ImageMetadata {
        width: texture_resolution.get(),
        height: texture_resolution.get(),
        pixel_format: PixelFormat::Rgba8,
    };

    let pixel = [128, 128, 255, 255]; // Corresponds to normal vector (0.0, 0.0, 1.0)

    let mut data = AVec::with_capacity(pixel.len() * pixel_count);

    for _ in 0..pixel_count {
        data.extend_from_slice(&pixel);
    }

    Image { meta, data }
}

fn float_to_u8(x: f32) -> u8 {
    ((x * 255.0) as u8).clamp(0, 255)
}
