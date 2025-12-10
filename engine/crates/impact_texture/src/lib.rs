//! Texture management.

pub mod gpu_resource;
pub mod import;
pub mod io;
pub mod lookup_table;
pub mod processing;

use anyhow::{Context, Result, anyhow, bail};
use gpu_resource::SamplingTexture;
use impact_alloc::{AVec, Allocator, arena::ArenaPool};
use impact_containers::DefaultHasher;
use impact_gpu::{
    device::GraphicsDevice,
    texture::{
        ColorSpace, DepthOrArrayLayers, SamplerConfig, TexelDescription, Texture, TextureConfig,
        mipmap::MipmapperGenerator,
    },
};
use impact_io::image::{Image, ImageMetadata, PixelFormat};
use impact_math::{hash64, stringhash64_newtype};
use impact_resource::{Resource, ResourceID, registry::ImmutableResourceRegistry};
use lookup_table::LookupTableTextureCreateInfo;
use processing::ImageProcessing;
use roc_integration::roc;
use std::{
    fmt,
    hash::{Hash, Hasher},
    num::NonZeroU32,
    path::PathBuf,
};

stringhash64_newtype!(
    /// Identifier for a texture.
    #[roc(parents = "Texture")]
    #[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
    [pub] TextureID
);

/// Identifier for a texture sampler, obtained by hashing the sampler
/// configuration parameters.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SamplerID(u64);

/// A registry of [`TextureCreateInfo`]s.
pub type TextureRegistry = ImmutableResourceRegistry<TextureCreateInfo>;

/// A registry of [`SamplerCreateInfo`]s.
pub type SamplerRegistry = ImmutableResourceRegistry<SamplerCreateInfo>;

/// Contains the information required to create a specific [`Texture`] and the
/// layout entry for bind groups containing the texture.
#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug)]
pub enum TextureCreateInfo {
    Image(ImageTextureCreateInfo),
    LookupTable(LookupTableTextureCreateInfo),
}

/// Contains the information required to create a specific image-based
/// [`Texture`] and the layout entry for bind groups containing the texture.
#[derive(Clone, Debug)]
pub struct ImageTextureCreateInfo {
    source: ImageTextureSource,
    metadata: ImageMetadata,
    texture_config: TextureConfig,
    sampler_config: Option<SamplerConfig>,
    processing: ImageProcessing,
}

/// Source for image-based texture data.
#[derive(Clone, Debug)]
pub enum ImageTextureSource {
    Single(ImageSource),
    Array {
        sources: Vec<ImageSource>,
        usage: TextureArrayUsage,
    },
}

/// Source for an image.
#[derive(Clone, Debug)]
pub enum ImageSource {
    File(PathBuf),
    Bytes(Image),
}

/// Intended usage for a texture array.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextureArrayUsage {
    Generic,
    Cubemap,
}

/// Contains the information required to create a specific
/// [`Sampler`](impact_gpu::texture::Sampler) and the layout entry for bind
/// groups containing the sampler.
#[derive(Clone, Debug)]
pub struct SamplerCreateInfo {
    pub config: SamplerConfig,
}

#[roc(dependencies = [impact_math::hash::Hash64])]
impl TextureID {
    #[roc(body = "Hashing.hash_str_64(name)")]
    /// Creates a texture ID hashed from the given name.
    pub fn from_name(name: &str) -> Self {
        Self(hash64!(name))
    }
}

impl ResourceID for TextureID {}

#[cfg(feature = "serde")]
impl serde::Serialize for TextureID {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for TextureID {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(TextureID::from_name(&s))
    }
}

impl From<&SamplerConfig> for SamplerID {
    fn from(config: &SamplerConfig) -> Self {
        let mut hasher = DefaultHasher::default();
        config.hash(&mut hasher);
        Self(hasher.finish())
    }
}

impl fmt::Display for SamplerID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SamplerID({})", self.0)
    }
}

impl ResourceID for SamplerID {}

impl TextureCreateInfo {
    pub fn sampler_id(&self) -> Option<SamplerID> {
        match self {
            Self::Image(image_texture_info) => {
                image_texture_info.sampler_config.as_ref().map(Into::into)
            }
            Self::LookupTable(table_texture_info) => {
                Some((&table_texture_info.sampler_config).into())
            }
        }
    }
}

impl Resource for TextureCreateInfo {
    type ID = TextureID;
}

impl ImageTextureCreateInfo {
    /// Extracts image texture creation information from the given source,
    /// metadata, and configuration.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The image width or height is zero.
    /// - The file list for array textures is empty.
    /// - The pixel format is incompatible with the texture configuration.
    pub fn new(
        source: ImageTextureSource,
        metadata: ImageMetadata,
        texture_config: TextureConfig,
        sampler_config: Option<SamplerConfig>,
        processing: ImageProcessing,
    ) -> Result<Self> {
        if metadata.width == 0 || metadata.height == 0 {
            bail!("Got zero width or height for texture image");
        }

        if source.depth_or_array_layers().is_none() {
            bail!("Got empty source list for image texture array");
        }

        // Ensure the pixel format is compatible with the configuration
        determine_valid_texel_description(metadata.pixel_format, &texture_config)?;

        Ok(Self {
            source,
            metadata,
            texture_config,
            sampler_config,
            processing,
        })
    }

    /// Returns the texel description for this texture.
    pub fn texel_description(&self) -> TexelDescription {
        determine_valid_texel_description(self.metadata.pixel_format, &self.texture_config).unwrap()
    }

    /// Returns the texture width.
    pub fn width(&self) -> NonZeroU32 {
        NonZeroU32::new(self.metadata.width).unwrap()
    }

    /// Returns the texture height.
    pub fn height(&self) -> NonZeroU32 {
        NonZeroU32::new(self.metadata.height).unwrap()
    }

    /// Returns the depth or array layers for this texture.
    pub fn depth_or_array_layers(&self) -> DepthOrArrayLayers {
        self.source.depth_or_array_layers().unwrap()
    }

    /// Whether this texture represents a cubemap.
    pub fn is_cubemap(&self) -> bool {
        self.source.is_cubemap()
    }
}

impl Resource for SamplerCreateInfo {
    type ID = SamplerID;
}

impl ImageTextureSource {
    /// Returns the depth or array layers for this texture source, or [`None`]
    /// if it is an empty array.
    pub fn depth_or_array_layers(&self) -> Option<DepthOrArrayLayers> {
        match self {
            Self::Single(_) => Some(DepthOrArrayLayers::Depth(NonZeroU32::new(1).unwrap())),
            Self::Array { sources, .. } => (!sources.is_empty()).then(|| {
                DepthOrArrayLayers::ArrayLayers(
                    NonZeroU32::new(u32::try_from(sources.len()).unwrap()).unwrap(),
                )
            }),
        }
    }

    /// Whether this source represents a cubemap texture.
    pub fn is_cubemap(&self) -> bool {
        matches!(
            self,
            Self::Array {
                usage: TextureArrayUsage::Cubemap,
                ..
            }
        )
    }
}

/// Creates a texture for the image file represented by the given raw byte
/// buffer, using the given configuration parameters. Mipmaps will be
/// generated automatically.
///
/// # Errors
/// Returns an error if:
/// - The image bytes can not be interpreted.
/// - The image width or height is zero.
/// - The row size (width times texel size) is not a multiple of 256 bytes
///   (`wgpu` requires that rows are a multiple of 256 bytes for copying
///   data between buffers and textures).
/// - The image is grayscale and the color space in the configuration is not
///   linear.
pub fn create_texture_from_bytes(
    graphics_device: &GraphicsDevice,
    mipmapper_generator: &MipmapperGenerator,
    byte_buffer: &[u8],
    texture_config: TextureConfig,
    processing: &ImageProcessing,
    label: &str,
) -> Result<Texture> {
    // Estimate capacity for image decompression (4x byte buffer size)
    let arena = ArenaPool::get_arena_for_capacity(byte_buffer.len() * 4);
    let image = impact_io::image::load_image_from_bytes(&arena, byte_buffer)?;
    create_texture_from_image(
        graphics_device,
        mipmapper_generator,
        &image,
        texture_config,
        processing,
        label,
    )
}

/// Creates a texture for the given loaded image, using the given
/// configuration parameters. Mipmaps will be generated automatically.
///
/// # Errors
/// Returns an error if:
/// - The image width or height is zero.
/// - The row size (width times texel size) is not a multiple of 256 bytes
///   (`wgpu` requires that rows are a multiple of 256 bytes for copying
///   data between buffers and textures).
/// - The image is grayscale and the color space in the configuration is not
///   linear.
pub fn create_texture_from_image<A: Allocator>(
    graphics_device: &GraphicsDevice,
    mipmapper_generator: &MipmapperGenerator,
    image: &Image<A>,
    texture_config: TextureConfig,
    processing: &ImageProcessing,
    label: &str,
) -> Result<Texture> {
    let (width, height) = image.dimensions();
    let width = NonZeroU32::new(width).ok_or_else(|| anyhow!("Image width is zero"))?;
    let height = NonZeroU32::new(height).ok_or_else(|| anyhow!("Image height is zero"))?;
    let depth = NonZeroU32::new(1).unwrap();

    let texel_description =
        determine_valid_texel_description(image.meta.pixel_format, &texture_config)?;

    let arena = ArenaPool::get_arena_for_capacity(if processing.is_none_for(&image.meta) {
        0
    } else {
        image.data.len()
    });
    let processed = processing.execute(&arena, image);

    let data = processed.as_ref().map_or_else(
        || image.data.as_slice(),
        |processed| processed.data.as_slice(),
    );

    Texture::create(
        graphics_device,
        Some(mipmapper_generator),
        data,
        width,
        height,
        DepthOrArrayLayers::Depth(depth),
        texel_description,
        false,
        texture_config,
        label,
    )
}

/// Creates a texture array for the given image sources, using the given
/// configuration parameters. The `verify_metadata` will be called with the
/// metadata of each image.
///
/// # Errors
/// Returns an error if:
/// - The number of images is zero.
/// - The usage is `Cubemap` and the number of images is not six.
/// - Any of the images are wrapped in an [`Err`].
/// - The image width or height is zero.
/// - The image is grayscale and the color space in the configuration is not
///   linear.
/// - The image dimensions or pixel formats do not match.
/// - The row size (width times texel size) is not a multiple of 256 bytes
///   (`wgpu` requires that rows are a multiple of 256 bytes for copying
///   data between buffers and textures).
pub fn create_texture_array_from_image_sources<'a, I, V>(
    graphics_device: &GraphicsDevice,
    mipmapper_generator: Option<&MipmapperGenerator>,
    image_sources: I,
    verify_metadata: V,
    texture_config: TextureConfig,
    usage: TextureArrayUsage,
    processing: &ImageProcessing,
    label: &str,
) -> Result<Texture>
where
    I: ExactSizeIterator<Item = &'a ImageSource>,
    V: Fn(&ImageMetadata) -> Result<()>,
{
    let mut sources = image_sources.into_iter();
    let n_images = sources.len();

    let first_source = sources
        .next()
        .ok_or_else(|| anyhow!("No image sources for texture array"))?;

    if usage == TextureArrayUsage::Cubemap && n_images != 6 {
        bail!("Expected 6 images for texture cubemap, got {n_images}");
    }

    let array_layers = NonZeroU32::new(u32::try_from(n_images).unwrap()).unwrap();

    let image_size_from_meta = |meta: &ImageMetadata| -> Result<usize> {
        let texel_description =
            determine_valid_texel_description(meta.pixel_format, &texture_config)?;

        Ok(meta.width as usize * meta.height as usize * texel_description.n_bytes() as usize)
    };

    // Estimate capacity for texture array processing
    let estimated_bytes_per_image = match first_source {
        ImageSource::File(_) => 4 * 1024 * 1024, // 4MB per image estimate
        ImageSource::Bytes(bytes) => bytes.data.len() * 4, // 4x decompression
    };
    let capacity = n_images * estimated_bytes_per_image;
    let arena = ArenaPool::get_arena_for_capacity(capacity);

    let (mut byte_buffer, meta) = match first_source {
        ImageSource::File(path) => {
            let image =
                impact_io::image::load_image_from_path(&arena, path).with_context(|| {
                    format!("Failed to load array texture image from {}", path.display())
                })?;
            verify_metadata(&image.meta)?;

            let processed = processing.execute(&arena, &image);

            let mut byte_buffer = processed.map_or(image.data, |processed| processed.data);

            byte_buffer.reserve((n_images - 1) * image_size_from_meta(&image.meta)?);

            (byte_buffer, image.meta)
        }
        ImageSource::Bytes(image) => {
            verify_metadata(&image.meta)?;

            let mut byte_buffer =
                AVec::with_capacity_in(n_images * image_size_from_meta(&image.meta)?, &arena);

            byte_buffer.extend_from_slice(&image.data);

            processing.execute_in_place(&image.meta, &mut byte_buffer);

            (byte_buffer, image.meta.clone())
        }
    };

    let width = NonZeroU32::new(meta.width).ok_or_else(|| anyhow!("Image width is zero"))?;
    let height = NonZeroU32::new(meta.height).ok_or_else(|| anyhow!("Image height is zero"))?;
    let texel_description = determine_valid_texel_description(meta.pixel_format, &texture_config)?;

    for source in sources {
        let source_meta = match source {
            ImageSource::File(path) => {
                let image =
                    impact_io::image::load_image_from_path(&arena, path).with_context(|| {
                        format!("Failed to load array texture image from {}", path.display())
                    })?;

                verify_metadata(&image.meta)?;

                byte_buffer.extend_from_slice(&image.data);

                let image_start = byte_buffer.len() - image.data.len();
                processing.execute_in_place(&image.meta, &mut byte_buffer[image_start..]);

                image.meta
            }
            ImageSource::Bytes(image) => {
                verify_metadata(&image.meta)?;

                byte_buffer.extend_from_slice(&image.data);

                let image_start = byte_buffer.len() - image.data.len();
                processing.execute_in_place(&image.meta, &mut byte_buffer[image_start..]);

                image.meta.clone()
            }
        };

        if source_meta != meta {
            bail!("Inconsistent metadata for array images: {source_meta:?} != {meta:?}");
        }
    }

    Texture::create(
        graphics_device,
        mipmapper_generator,
        &byte_buffer,
        width,
        height,
        DepthOrArrayLayers::ArrayLayers(array_layers),
        texel_description,
        usage == TextureArrayUsage::Cubemap,
        texture_config,
        label,
    )
}

/// Creates a texture from the given [`TextureCreateInfo`].
///
/// Loads image or lookup table data from files specified in the info and
/// creates the appropriate texture type (standard, array (including cubemap),
/// or lookup table). Mipmaps will be generated automatically for image-based
/// textures configured for it.
///
/// # Returns
/// A [`SamplingTexture`] matching the type and configuration specified in the
/// info.
///
/// # Errors
/// Returns an error if:
/// - Image or lookup table files cannot be read from the specified paths.
/// - Image/table metadata from the info does not match the actual image/table
///   metadata.
///
/// See also
/// - [`create_texture_from_image`],
/// - [`create_texture_array_from_image_sources`]
/// - [`create_texture_from_lookup_table`](lookup_table::create_texture_from_lookup_table).
pub(crate) fn create_texture_from_info(
    graphics_device: &GraphicsDevice,
    mipmapper_generator: &MipmapperGenerator,
    texture_info: &TextureCreateInfo,
    label: &str,
) -> Result<SamplingTexture> {
    fn verify_image_metadata(from_info: &ImageMetadata, from_image: &ImageMetadata) -> Result<()> {
        if from_info != from_image {
            bail!(
                "Image metadata from texture info ({from_info:?}) \
                 does not match metadata from image file ({from_image:?})"
            );
        }
        Ok(())
    }

    #[cfg(feature = "bincode")]
    fn verify_table_metadata(
        from_info: &lookup_table::LookupTableMetadata,
        from_file: &lookup_table::LookupTableMetadata,
    ) -> Result<()> {
        if from_info != from_file {
            bail!(
                "Lookup table metadata from info ({from_info:?}) \
                 does not match metadata from table file ({from_file:?})"
            );
        }
        Ok(())
    }

    // Estimate capacity for texture creation
    let capacity = match texture_info {
        TextureCreateInfo::Image(_) => 4 * 1024 * 1024, // 4MB image estimate
        TextureCreateInfo::LookupTable(_) => 1024 * 1024, // 1MB lookup table
    };
    let arena = ArenaPool::get_arena_for_capacity(capacity);

    let (texture, sampler_id) = match texture_info {
        TextureCreateInfo::Image(image_texture_info) => {
            let image_metadata = &image_texture_info.metadata;
            let texture_config = image_texture_info.texture_config.clone();
            let processing = &image_texture_info.processing;
            let sampler_id = image_texture_info.sampler_config.as_ref().map(Into::into);

            let texture = match &image_texture_info.source {
                ImageTextureSource::Single(ImageSource::File(path)) => {
                    let image = impact_io::image::load_image_from_path(&arena, path).with_context(
                        || format!("Failed to load texture image from {}", path.display()),
                    )?;

                    verify_image_metadata(image_metadata, &image.meta)?;

                    create_texture_from_image(
                        graphics_device,
                        mipmapper_generator,
                        &image,
                        texture_config,
                        processing,
                        label,
                    )?
                }
                ImageTextureSource::Single(ImageSource::Bytes(image)) => {
                    verify_image_metadata(image_metadata, &image.meta)?;

                    create_texture_from_image(
                        graphics_device,
                        mipmapper_generator,
                        image,
                        texture_config,
                        processing,
                        label,
                    )?
                }
                ImageTextureSource::Array { sources, usage } => {
                    let mipmapper_generator = match usage {
                        TextureArrayUsage::Generic => Some(mipmapper_generator),
                        TextureArrayUsage::Cubemap => None,
                    };

                    create_texture_array_from_image_sources(
                        graphics_device,
                        mipmapper_generator,
                        sources.iter(),
                        |meta| verify_image_metadata(image_metadata, meta),
                        texture_config,
                        *usage,
                        processing,
                        label,
                    )?
                }
            };
            (texture, sampler_id)
        }
        TextureCreateInfo::LookupTable(LookupTableTextureCreateInfo {
            table_path,
            metadata,
            sampler_config,
        }) => {
            let texture = {
                #[cfg(feature = "bincode")]
                match metadata.value_type {
                    lookup_table::LookupTableValueType::Float32 => {
                        let table = io::read_lookup_table_from_file::<f32>(table_path)
                            .with_context(|| {
                                format!("Failed to load lookup table from {}", table_path.display())
                            })?;
                        verify_table_metadata(metadata, table.metadata())?;
                        lookup_table::create_texture_from_lookup_table(
                            graphics_device,
                            &table,
                            label,
                        )
                    }
                    lookup_table::LookupTableValueType::Unsigned8 => {
                        let table = io::read_lookup_table_from_file::<u8>(table_path)
                            .with_context(|| {
                                format!("Failed to load lookup table from {}", table_path.display())
                            })?;
                        verify_table_metadata(metadata, table.metadata())?;
                        lookup_table::create_texture_from_lookup_table(
                            graphics_device,
                            &table,
                            label,
                        )
                    }
                }
                #[cfg(not(feature = "bincode"))]
                Err(anyhow!(
                    "Enable the `bincode` feature to create lookup table textures"
                ))
            }?;
            (texture, Some(SamplerID::from(sampler_config)))
        }
    };

    Ok(SamplingTexture {
        texture,
        sampler_id,
    })
}

/// Determines the appropriate texel description for the given pixel format
/// and texture configuration.
///
/// # Errors
/// Returns an error if:
/// - The pixel format is grayscale and the color space in the configuration
///   is not linear.
pub(crate) fn determine_valid_texel_description(
    pixel_format: PixelFormat,
    texture_config: &TextureConfig,
) -> Result<TexelDescription> {
    Ok(match pixel_format {
        PixelFormat::Rgba8 => TexelDescription::Rgba8(texture_config.color_space),
        PixelFormat::Luma8 => {
            if texture_config.color_space != ColorSpace::Linear {
                bail!(
                    "Unsupported color space {:?} for grayscale image",
                    texture_config.color_space,
                );
            }
            TexelDescription::Grayscale8
        }
    })
}
