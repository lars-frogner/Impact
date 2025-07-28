//! Textures.

pub mod mipmap;

use crate::{buffer, device::GraphicsDevice};
use anyhow::{Result, bail};
use bytemuck::Pod;
use mipmap::MipmapperGenerator;
use ordered_float::OrderedFloat;
use std::{
    borrow::Cow,
    hash::{DefaultHasher, Hash, Hasher},
    num::NonZeroU32,
};

/// Identifier for specific texture samplers.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SamplerID(u64);

/// Represents a data type that can be copied directly to a [`Texture`].
pub trait TexelType: Pod {
    const DESCRIPTION: TexelDescription;
}

/// A description of the data type used for a texel.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TexelDescription {
    Rgba8(ColorSpace),
    Grayscale8,
    Float32,
}

/// A color space for pixel/texel values.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum ColorSpace {
    #[default]
    Linear,
    Srgb,
}

/// A texture holding multidimensional data.
#[derive(Debug)]
pub struct Texture {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    view_dimension: wgpu::TextureViewDimension,
    sampler_id: Option<SamplerID>,
}

/// A sampler for sampling [`Texture`]s.
#[derive(Debug)]
pub struct Sampler {
    sampler: wgpu::Sampler,
    binding_type: wgpu::SamplerBindingType,
}

/// Configuration parameters for [`Texture`]s.
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(default)
)]
#[derive(Clone, Debug, Default)]
pub struct TextureConfig {
    /// The color space that the texel data values should be assumed to be
    /// stored in.
    pub color_space: ColorSpace,
    /// The maximum number of mip levels that should be generated for the
    /// texture. If [`None`], a full mipmap chain will be generated.
    pub max_mip_level_count: Option<u32>,
}

/// Configuration parameters for [`Sampler`]s.
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(default)
)]
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SamplerConfig {
    /// How the sampler should address into textures.
    pub addressing: TextureAddressingConfig,
    /// How the sampler should filter textures.
    pub filtering: TextureFilteringConfig,
}

/// How a [`Sampler`] should address into [`Texture`]s.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum TextureAddressingConfig {
    /// Equivalent to [`DetailedTextureAddressingConfig::CLAMPED`].
    #[default]
    Clamped,
    /// Equivalent to [`DetailedTextureAddressingConfig::REPEATING`].
    Repeating,
    Detailed(DetailedTextureAddressingConfig),
}

/// Configuration parameters for how a [`Sampler`] should address into
/// [`Texture`]s.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct DetailedTextureAddressingConfig {
    /// How addressing outside the [0, 1] range for the U texture coordinate
    /// should be handled.
    pub address_mode_u: wgpu::AddressMode,
    /// How addressing outside the [0, 1] range for the V texture coordinate
    /// should be handled.
    pub address_mode_v: wgpu::AddressMode,
    /// How addressing outside the [0, 1] range for the W texture coordinate
    /// should be handled.
    pub address_mode_w: wgpu::AddressMode,
}

/// How a [`Sampler`] should filter [`Texture`]s.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, Default, PartialEq)]
pub enum TextureFilteringConfig {
    /// Equivalent to [`DetailedTextureFilteringConfig::NONE`].
    None,
    /// Equivalent to [`DetailedTextureFilteringConfig::BASIC`].
    #[default]
    Basic,
    /// Equivalent to [`DetailedTextureFilteringConfig::ANISOTROPIC_2X`].
    Anisotropic2x,
    /// Equivalent to [`DetailedTextureFilteringConfig::ANISOTROPIC_4X`].
    Anisotropic4x,
    /// Equivalent to [`DetailedTextureFilteringConfig::ANISOTROPIC_8X`].
    Anisotropic8x,
    /// Equivalent to [`DetailedTextureFilteringConfig::ANISOTROPIC_16X`].
    Anisotropic16x,
    Detailed(DetailedTextureFilteringConfig),
}

/// Configuration parameters for how a [`Sampler`] should filter [`Texture`]s.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct DetailedTextureFilteringConfig {
    /// Whether filtering will be enabled when sampling the texture.
    pub filtering_enabled: bool,
    /// How to filter the texture when it needs to be magnified.
    pub mag_filter: wgpu::FilterMode,
    /// How to filter the texture when it needs to be minified.
    pub min_filter: wgpu::FilterMode,
    /// How to filter between mipmap levels.
    pub mipmap_filter: wgpu::FilterMode,
    /// Minimum level of detail (i.e. mip level) to use.
    pub lod_min_clamp: f32,
    /// Maximum level of detail (i.e. mip level) to use.
    pub lod_max_clamp: f32,
    /// Maximum number of samples to use for anisotropic filtering.
    pub anisotropy_clamp: u16,
}

/// Dimensions and data for a lookup table to be loaded into a texture.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct TextureLookupTable<T: TexelType> {
    width: NonZeroU32,
    height: NonZeroU32,
    depth_or_array_layers: DepthOrArrayLayers,
    data: Vec<T>,
}

/// A number that either represents the number of depths in a 3D texture or the
/// number of layers in a 1D or 2D texture array.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DepthOrArrayLayers {
    Depth(NonZeroU32),
    ArrayLayers(NonZeroU32),
}

impl From<&SamplerConfig> for SamplerID {
    fn from(config: &SamplerConfig) -> Self {
        let addressing: DetailedTextureAddressingConfig = config.addressing.clone().into();
        let filtering: DetailedTextureFilteringConfig = config.filtering.clone().into();

        let mut hasher = DefaultHasher::new();
        addressing.hash(&mut hasher);
        filtering.filtering_enabled.hash(&mut hasher);
        filtering.mag_filter.hash(&mut hasher);
        filtering.min_filter.hash(&mut hasher);
        filtering.mipmap_filter.hash(&mut hasher);
        OrderedFloat(filtering.lod_min_clamp).hash(&mut hasher);
        OrderedFloat(filtering.lod_max_clamp).hash(&mut hasher);
        filtering.anisotropy_clamp.hash(&mut hasher);
        SamplerID(hasher.finish())
    }
}

impl TexelDescription {
    /// The number of bytes needed to represent a texel of this type.
    pub fn n_bytes(&self) -> u32 {
        match self {
            Self::Rgba8(_) | Self::Float32 => 4,
            Self::Grayscale8 => 1,
        }
    }

    /// Returns the texture format that will be used for this texel description.
    pub fn texture_format(&self) -> wgpu::TextureFormat {
        match self {
            Self::Rgba8(ColorSpace::Linear) => wgpu::TextureFormat::Rgba8Unorm,
            Self::Rgba8(ColorSpace::Srgb) => wgpu::TextureFormat::Rgba8UnormSrgb,
            Self::Grayscale8 => wgpu::TextureFormat::R8Unorm,
            Self::Float32 => wgpu::TextureFormat::R32Float,
        }
    }
}

impl TexelType for f32 {
    const DESCRIPTION: TexelDescription = TexelDescription::Float32;
}

impl TexelType for u8 {
    const DESCRIPTION: TexelDescription = TexelDescription::Grayscale8;
}

impl Texture {
    /// Creates a texture holding the given lookup table. The texture will be
    /// sampled with (bi/tri)linear interpolation, and lookups outside [0, 1]
    /// are clamped to the edge values.
    ///
    /// # Errors
    /// Returns an error if the row size (width times data value size) is not a
    /// multiple of 256 bytes (`wgpu` requires that rows are a multiple of 256
    /// bytes for copying data between buffers and textures).
    pub fn from_lookup_table<T: TexelType>(
        graphics_device: &GraphicsDevice,
        table: &TextureLookupTable<T>,
        label: &str,
        sampler_id: Option<SamplerID>,
    ) -> Result<Self> {
        let byte_buffer = bytemuck::cast_slice(&table.data);

        let texture_config = TextureConfig {
            color_space: ColorSpace::Linear,
            ..Default::default()
        };

        Self::create(
            graphics_device,
            None,
            byte_buffer,
            table.width,
            table.height,
            table.depth_or_array_layers,
            T::DESCRIPTION,
            false,
            texture_config,
            sampler_id,
            label,
        )
    }

    /// Creates a texture for the data contained in the given byte buffer, with
    /// the given dimensions and texel description, using the given
    /// configuration parameters. Mipmaps will be generated automatically if a
    /// generator is provided.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The texture shape and texel size are inconsistent with the size of the
    ///   byte buffer.
    /// - The row size (width times texel size) is not a multiple of 256 bytes
    ///   (`wgpu` requires that rows are a multiple of 256 bytes for copying
    ///   data between buffers and textures).
    pub fn create(
        graphics_device: &GraphicsDevice,
        mipmapper_generator: Option<&MipmapperGenerator>,
        byte_buffer: &[u8],
        width: NonZeroU32,
        height: NonZeroU32,
        depth_or_array_layers: DepthOrArrayLayers,
        texel_description: TexelDescription,
        is_cubemap: bool,
        texture_config: TextureConfig,
        sampler_id: Option<SamplerID>,
        label: &str,
    ) -> Result<Self> {
        let texture_size = wgpu::Extent3d {
            width: u32::from(width),
            height: u32::from(height),
            depth_or_array_layers: u32::from(depth_or_array_layers.unwrap()),
        };

        if (texel_description.n_bytes()
            * texture_size.width
            * texture_size.height
            * texture_size.depth_or_array_layers) as usize
            != byte_buffer.len()
        {
            bail!(
                "Texture {} shape ({}, {}, {:?}) and texel size ({} bytes) not consistent with number bytes of data ({})",
                label,
                width,
                height,
                depth_or_array_layers,
                texel_description.n_bytes(),
                byte_buffer.len()
            )
        } else if (texel_description.n_bytes() * texture_size.width) % 256 != 0 {
            bail!(
                "Texture {} row size ({} bytes) is not a multiple of 256 bytes",
                label,
                texel_description.n_bytes() * texture_size.width
            )
        }

        let (dimension, view_dimension) =
            if let DepthOrArrayLayers::ArrayLayers(array_layers) = depth_or_array_layers {
                let view_dimension = if is_cubemap {
                    if array_layers.get() != 6 {
                        bail!("Tried to create cubemap with {} array layers", array_layers);
                    }
                    wgpu::TextureViewDimension::Cube
                } else {
                    wgpu::TextureViewDimension::D2Array
                };
                (wgpu::TextureDimension::D2, view_dimension)
            } else if texture_size.depth_or_array_layers == 1 {
                if texture_size.height == 1 {
                    (wgpu::TextureDimension::D1, wgpu::TextureViewDimension::D1)
                } else {
                    (wgpu::TextureDimension::D2, wgpu::TextureViewDimension::D2)
                }
            } else {
                (wgpu::TextureDimension::D3, wgpu::TextureViewDimension::D3)
            };

        let device = graphics_device.device();
        let queue = graphics_device.queue();

        let format = texel_description.texture_format();

        let full_mip_chain_level_count = texture_size.max_mips(dimension);

        let mip_level_count = if mipmapper_generator.is_some() {
            texture_config
                .max_mip_level_count
                .unwrap_or(full_mip_chain_level_count)
                .clamp(1, full_mip_chain_level_count)
        } else {
            1
        };

        let texture = Self::create_empty_texture(
            device,
            format,
            texture_size,
            dimension,
            mip_level_count,
            label,
        );

        Self::write_data_to_texture(
            queue,
            &texture,
            byte_buffer,
            texel_description,
            texture_size,
        );

        if let Some(mipmapper_generator) = mipmapper_generator {
            mipmapper_generator.update_texture_mipmaps(
                graphics_device,
                &texture,
                Cow::Owned(label.to_string()),
            );
        }

        let view = Self::create_view(&texture, view_dimension);

        Ok(Self::new(texture, view, view_dimension, sampler_id))
    }

    /// Creates a new [`Texture`] comprised of the given `wgpu` texture and
    /// sampler data.
    pub fn new(
        texture: wgpu::Texture,
        view: wgpu::TextureView,
        view_dimension: wgpu::TextureViewDimension,
        sampler_id: Option<SamplerID>,
    ) -> Self {
        Self {
            texture,
            view,
            view_dimension,
            sampler_id,
        }
    }

    /// Returns a reference to the underlying [`wgpu::Texture`].
    pub fn texture(&self) -> &wgpu::Texture {
        &self.texture
    }

    /// Returns a view into the texture.
    pub fn view(&self) -> &wgpu::TextureView {
        &self.view
    }

    /// Returns the ID of the specific sampler to use for this texture, or
    /// [`None`] if this texture has no specific sampler.
    pub fn sampler_id(&self) -> Option<SamplerID> {
        self.sampler_id
    }

    /// Creates the bind group layout entry for this texture, assigned to the
    /// given binding.
    pub fn create_bind_group_layout_entry(
        &self,
        binding: u32,
        visibility: wgpu::ShaderStages,
    ) -> wgpu::BindGroupLayoutEntry {
        create_texture_bind_group_layout_entry(
            binding,
            visibility,
            self.texture.format(),
            self.view_dimension,
        )
    }

    /// Creates the bind group entry for this texture, assigned to the given
    /// binding.
    pub fn create_bind_group_entry(&self, binding: u32) -> wgpu::BindGroupEntry<'_> {
        wgpu::BindGroupEntry {
            binding,
            resource: wgpu::BindingResource::TextureView(self.view()),
        }
    }

    /// Creates a new [`wgpu::Texture`] configured to hold 2D image data.
    fn create_empty_texture(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        texture_size: wgpu::Extent3d,
        dimension: wgpu::TextureDimension,
        mip_level_count: u32,
        label: &str,
    ) -> wgpu::Texture {
        device.create_texture(&wgpu::TextureDescriptor {
            size: texture_size,
            mip_level_count,
            sample_count: 1,
            dimension,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_DST,
            label: Some(label),
            view_formats: &[],
        })
    }

    fn write_data_to_texture(
        queue: &wgpu::Queue,
        texture: &wgpu::Texture,
        byte_buffer: &[u8],
        texel_description: TexelDescription,
        texture_size: wgpu::Extent3d,
    ) {
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            byte_buffer,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(texel_description.n_bytes() * texture_size.width),
                rows_per_image: Some(texture_size.height),
            },
            texture_size,
        );
    }

    fn create_view(
        texture: &wgpu::Texture,
        view_dimension: wgpu::TextureViewDimension,
    ) -> wgpu::TextureView {
        texture.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(view_dimension),
            ..Default::default()
        })
    }
}

impl Sampler {
    /// Creates a new sampler with the given configuration.
    pub fn create(graphics_device: &GraphicsDevice, config: SamplerConfig) -> Self {
        let addressing: DetailedTextureAddressingConfig = config.addressing.into();
        let filtering: DetailedTextureFilteringConfig = config.filtering.into();

        let sampler = graphics_device
            .device()
            .create_sampler(&wgpu::SamplerDescriptor {
                address_mode_u: addressing.address_mode_u,
                address_mode_v: addressing.address_mode_v,
                address_mode_w: addressing.address_mode_w,
                mag_filter: filtering.mag_filter,
                min_filter: filtering.min_filter,
                mipmap_filter: filtering.mipmap_filter,
                lod_min_clamp: filtering.lod_min_clamp,
                lod_max_clamp: filtering.lod_max_clamp,
                anisotropy_clamp: filtering.anisotropy_clamp,
                ..Default::default()
            });

        let binding_type = if filtering.filtering_enabled {
            wgpu::SamplerBindingType::Filtering
        } else {
            wgpu::SamplerBindingType::NonFiltering
        };

        Self {
            sampler,
            binding_type,
        }
    }

    /// Returns a reference to the underlying [`wgpu::Sampler`] sampler.
    pub fn sampler(&self) -> &wgpu::Sampler {
        &self.sampler
    }

    /// Creates the bind group layout entry for this sampler, assigned to the
    /// given binding.
    pub fn create_bind_group_layout_entry(
        &self,
        binding: u32,
        visibility: wgpu::ShaderStages,
    ) -> wgpu::BindGroupLayoutEntry {
        create_sampler_bind_group_layout_entry(binding, visibility, self.binding_type)
    }

    /// Creates the bind group entry for this sampler, assigned to the given
    /// binding.
    pub fn create_bind_group_entry(&self, binding: u32) -> wgpu::BindGroupEntry<'_> {
        wgpu::BindGroupEntry {
            binding,
            resource: wgpu::BindingResource::Sampler(self.sampler()),
        }
    }
}

impl DetailedTextureAddressingConfig {
    pub const CLAMPED: Self = Self {
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
    };

    pub const REPEATING: Self = Self {
        address_mode_u: wgpu::AddressMode::Repeat,
        address_mode_v: wgpu::AddressMode::Repeat,
        address_mode_w: wgpu::AddressMode::Repeat,
    };
}

impl Default for DetailedTextureAddressingConfig {
    fn default() -> Self {
        Self::CLAMPED
    }
}

impl From<TextureAddressingConfig> for DetailedTextureAddressingConfig {
    fn from(config: TextureAddressingConfig) -> Self {
        match config {
            TextureAddressingConfig::Clamped => Self::CLAMPED,
            TextureAddressingConfig::Repeating => Self::REPEATING,
            TextureAddressingConfig::Detailed(config) => config,
        }
    }
}

impl DetailedTextureFilteringConfig {
    pub const BASIC: Self = Self {
        filtering_enabled: true,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::FilterMode::Linear,
        lod_min_clamp: 0.0,
        lod_max_clamp: f32::MAX,
        anisotropy_clamp: 1,
    };

    pub const ANISOTROPIC_2X: Self = Self {
        filtering_enabled: true,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::FilterMode::Linear,
        lod_min_clamp: 0.0,
        lod_max_clamp: f32::MAX,
        anisotropy_clamp: 2,
    };

    pub const ANISOTROPIC_4X: Self = Self {
        anisotropy_clamp: 4,
        ..Self::ANISOTROPIC_2X
    };

    pub const ANISOTROPIC_8X: Self = Self {
        anisotropy_clamp: 8,
        ..Self::ANISOTROPIC_2X
    };

    pub const ANISOTROPIC_16X: Self = Self {
        anisotropy_clamp: 16,
        ..Self::ANISOTROPIC_2X
    };

    pub const NONE: Self = Self {
        filtering_enabled: false,
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        mipmap_filter: wgpu::FilterMode::Nearest,
        lod_min_clamp: 0.0,
        lod_max_clamp: f32::MAX,
        anisotropy_clamp: 1,
    };
}

impl Default for DetailedTextureFilteringConfig {
    fn default() -> Self {
        Self::BASIC
    }
}

impl From<TextureFilteringConfig> for DetailedTextureFilteringConfig {
    fn from(config: TextureFilteringConfig) -> Self {
        match config {
            TextureFilteringConfig::None => Self::NONE,
            TextureFilteringConfig::Basic => Self::BASIC,
            TextureFilteringConfig::Anisotropic2x => Self::ANISOTROPIC_2X,
            TextureFilteringConfig::Anisotropic4x => Self::ANISOTROPIC_4X,
            TextureFilteringConfig::Anisotropic8x => Self::ANISOTROPIC_8X,
            TextureFilteringConfig::Anisotropic16x => Self::ANISOTROPIC_16X,
            TextureFilteringConfig::Detailed(config) => config,
        }
    }
}

impl<T: TexelType> TextureLookupTable<T> {
    /// Wraps the lookup table with the given dimensions and data. The table is
    /// considered an array of 2D subtables if `depth_or_array_layers` is
    /// `ArrayLayers`, otherwise it is considered 1D if `depth_or_array_layers`
    /// and `height` are 1, 2D if only `depth_or_array_layers` is 1 and 3D
    /// otherwise. The lookup values in the `data` vector are assumed to be laid
    /// out in row-major order, with adjacent values varying in width first,
    /// then height and finally depth.
    ///
    /// # Panics
    /// - If the `data` vector is empty.
    /// - If the table shape is inconsistent with the number of data values.
    pub fn new(
        width: usize,
        height: usize,
        depth_or_array_layers: DepthOrArrayLayers,
        data: Vec<T>,
    ) -> Self {
        assert!(!data.is_empty(), "No data for lookup table");

        assert_eq!(
            width * height * (u32::from(depth_or_array_layers.unwrap()) as usize),
            data.len(),
            "Lookup table shape ({}, {}, {:?}) inconsistent with number of data values ({})",
            width,
            height,
            depth_or_array_layers,
            data.len()
        );

        let width = NonZeroU32::new(u32::try_from(width).unwrap()).unwrap();
        let height = NonZeroU32::new(u32::try_from(height).unwrap()).unwrap();

        Self {
            width,
            height,
            depth_or_array_layers,
            data,
        }
    }
}

impl DepthOrArrayLayers {
    fn unwrap(&self) -> NonZeroU32 {
        match self {
            Self::Depth(depth) => *depth,
            Self::ArrayLayers(n_array_layers) => *n_array_layers,
        }
    }
}

/// Creates a [`wgpu::BindGroupLayoutEntry`] for a [`Texture`] with the given
/// binding, visibility, format and view dimension.
pub fn create_texture_bind_group_layout_entry(
    binding: u32,
    visibility: wgpu::ShaderStages,
    format: wgpu::TextureFormat,
    view_dimension: wgpu::TextureViewDimension,
) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility,
        ty: wgpu::BindingType::Texture {
            sample_type: format
                .sample_type(
                    Some(wgpu::TextureAspect::DepthOnly),
                    Some(wgpu::Features::FLOAT32_FILTERABLE),
                )
                .unwrap(),
            view_dimension,
            multisampled: false,
        },
        count: None,
    }
}

/// Creates a [`wgpu::BindGroupLayoutEntry`] for a [`Sampler`] with the given
/// binding, visibility and binding type.
pub fn create_sampler_bind_group_layout_entry(
    binding: u32,
    visibility: wgpu::ShaderStages,
    binding_type: wgpu::SamplerBindingType,
) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility,
        ty: wgpu::BindingType::Sampler(binding_type),
        count: None,
    }
}

pub fn extract_texture_data_and_convert<IN: Pod, OUT: From<IN>>(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    mip_level: u32,
    texture_array_idx: u32,
) -> Result<Vec<OUT>> {
    let data = extract_texture_data::<IN>(device, queue, texture, mip_level, texture_array_idx)?;
    Ok(data.into_iter().map(|value| OUT::from(value)).collect())
}

pub fn extract_texture_data<T: Pod>(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    mip_level: u32,
    texture_array_idx: u32,
) -> Result<Vec<T>> {
    let data = extract_texture_bytes(device, queue, texture, mip_level, texture_array_idx)?;
    Ok(bytemuck::cast_slice(&data).to_vec())
}

pub fn extract_texture_bytes(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    mip_level: u32,
    texture_array_idx: u32,
) -> Result<Vec<u8>> {
    assert!(mip_level <= texture.mip_level_count());

    assert!(texture_array_idx < texture.depth_or_array_layers());
    let texture_array_idx = texture_array_idx as usize;

    let size = texture
        .size()
        .mip_level_size(mip_level, texture.dimension());
    let width = u64::from(size.width);
    let height = u64::from(size.height);

    let format = texture.format();

    let aspect = if format.has_depth_aspect() {
        wgpu::TextureAspect::DepthOnly
    } else {
        wgpu::TextureAspect::All
    };

    let block_size = u64::from(
        texture
            .format()
            .block_copy_size(Some(aspect))
            .expect("Texel block size unavailable"),
    );

    let block_dim = u64::from(format.block_dimensions().0);
    let blocks_per_row = width.div_ceil(block_dim);
    let bytes_per_row = block_size * blocks_per_row;

    const ALIGNMENT: u64 = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as u64;

    let padded_bytes_per_row = bytes_per_row.div_ceil(ALIGNMENT) * ALIGNMENT;
    assert!(u32::try_from(padded_bytes_per_row).is_ok());

    let padded_layer_size = padded_bytes_per_row * height;
    let padded_buffer_size = padded_layer_size * u64::from(texture.depth_or_array_layers());

    let mut command_encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Texture copy encoder"),
    });

    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        size: padded_buffer_size,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        label: Some("Texture buffer"),
        mapped_at_creation: false,
    });

    command_encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level,
            origin: wgpu::Origin3d::ZERO,
            aspect,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded_bytes_per_row as u32),
                rows_per_image: Some(height as u32),
            },
        },
        size,
    );

    queue.submit(std::iter::once(command_encoder.finish()));

    let buffer_view = buffer::map_buffer_slice_to_cpu(device, buffer.slice(..))?;

    let texture_image_size = (bytes_per_row * height) as usize;

    // Extract only the data of the texture with the given texture array index
    let layer_start = texture_array_idx * padded_layer_size as usize;

    // Return the buffer data directly if there is no padding
    if bytes_per_row == padded_bytes_per_row {
        return Ok(buffer_view[layer_start..layer_start + texture_image_size].to_vec());
    }

    let mut image_buffer = Vec::with_capacity(texture_image_size);

    // Only copy over non-padding data
    for row_idx in 0..height as usize {
        let start = layer_start + row_idx * padded_bytes_per_row as usize;
        let end = start + bytes_per_row as usize;
        image_buffer.extend_from_slice(&buffer_view[start..end]);
    }
    assert_eq!(image_buffer.len(), texture_image_size);

    Ok(image_buffer)
}
