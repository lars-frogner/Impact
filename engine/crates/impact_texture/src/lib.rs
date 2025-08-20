//! Texture management.

pub mod gpu_resource;
pub mod import;
pub mod io;
pub mod lookup_table;

use anyhow::{Context, Result, anyhow, bail};
use gpu_resource::SamplingTexture;
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
use roc_integration::roc;
use std::{
    borrow::{Borrow, Cow},
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
}

/// Source for image-based texture data.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub enum ImageTextureSource {
    Image(ImageSource),
    ArrayImages(Vec<ImageSource>),
    CubemapImages {
        right: ImageSource,
        left: ImageSource,
        top: ImageSource,
        bottom: ImageSource,
        front: ImageSource,
        back: ImageSource,
    },
}

/// Source for an image.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub enum ImageSource {
    File(PathBuf),
    Bytes(Image),
}

/// Contains the information required to create a specific
/// [`Sampler`](impact_gpu::texture::Sampler) and the layout entry for bind
/// groups containing the sampler.
#[derive(Clone, Debug)]
pub struct SamplerCreateInfo {
    pub config: SamplerConfig,
}

#[roc(dependencies = [impact_math::Hash64])]
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
    ) -> Result<Self> {
        if metadata.width == 0 || metadata.height == 0 {
            bail!("Got zero width or height for texture image");
        }

        if let ImageTextureSource::ArrayImages(sources) = &source
            && sources.is_empty()
        {
            bail!("Got empty source list for image texture array");
        }

        // Ensure the pixel format is compatible with the configuration
        determine_valid_texel_description(metadata.pixel_format, &texture_config)?;

        Ok(Self {
            source,
            metadata,
            texture_config,
            sampler_config,
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
        match &self.source {
            ImageTextureSource::Image(_) => DepthOrArrayLayers::Depth(NonZeroU32::new(1).unwrap()),
            ImageTextureSource::ArrayImages(sources) => DepthOrArrayLayers::ArrayLayers(
                NonZeroU32::new(u32::try_from(sources.len()).unwrap()).unwrap(),
            ),
            ImageTextureSource::CubemapImages { .. } => {
                DepthOrArrayLayers::ArrayLayers(NonZeroU32::new(6).unwrap())
            }
        }
    }

    /// Whether this texture represents a cubemap.
    pub fn is_cubemap(&self) -> bool {
        matches!(&self.source, ImageTextureSource::CubemapImages { .. })
    }
}

impl Resource for SamplerCreateInfo {
    type ID = SamplerID;
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
    label: &str,
) -> Result<Texture> {
    let image = impact_io::image::load_image_from_bytes(byte_buffer)?;
    create_texture_from_image(
        graphics_device,
        mipmapper_generator,
        &image,
        texture_config,
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
pub fn create_texture_from_image(
    graphics_device: &GraphicsDevice,
    mipmapper_generator: &MipmapperGenerator,
    image: &Image,
    texture_config: TextureConfig,
    label: &str,
) -> Result<Texture> {
    let (width, height) = image.dimensions();
    let width = NonZeroU32::new(width).ok_or_else(|| anyhow!("Image width is zero"))?;
    let height = NonZeroU32::new(height).ok_or_else(|| anyhow!("Image height is zero"))?;
    let depth = NonZeroU32::new(1).unwrap();

    let texel_description =
        determine_valid_texel_description(image.meta.pixel_format, &texture_config)?;

    Texture::create(
        graphics_device,
        Some(mipmapper_generator),
        &image.data,
        width,
        height,
        DepthOrArrayLayers::Depth(depth),
        texel_description,
        false,
        texture_config,
        label,
    )
}

/// Creates a cubemap texture for the given loaded images representing
/// cubemap faces, using the given configuration parameters.
///
/// # Errors
/// Returns an error if:
/// - The image dimensions or pixel formats do not match.
/// - The image width or height is zero.
/// - The row size (width times texel size) is not a multiple of 256 bytes
///   (`wgpu` requires that rows are a multiple of 256 bytes for copying
///   data between buffers and textures).
/// - The image is grayscale and the color space in the configuration is not
///   linear.
pub fn create_cubemap_texture_from_images(
    graphics_device: &GraphicsDevice,
    right_image: &Image,
    left_image: &Image,
    top_image: &Image,
    bottom_image: &Image,
    front_image: &Image,
    back_image: &Image,
    texture_config: TextureConfig,
    label: &str,
) -> Result<Texture> {
    let dimensions = right_image.dimensions();
    if left_image.dimensions() != dimensions
        || top_image.dimensions() != dimensions
        || bottom_image.dimensions() != dimensions
        || front_image.dimensions() != dimensions
        || back_image.dimensions() != dimensions
    {
        bail!("Inconsistent dimensions for cubemap texture images")
    }

    let pixel_format = right_image.meta.pixel_format;
    if left_image.meta.pixel_format != pixel_format
        || top_image.meta.pixel_format != pixel_format
        || bottom_image.meta.pixel_format != pixel_format
        || front_image.meta.pixel_format != pixel_format
        || back_image.meta.pixel_format != pixel_format
    {
        bail!("Inconsistent pixel formats for cubemap texture images")
    }

    let (width, height) = right_image.dimensions();
    let width = NonZeroU32::new(width).ok_or_else(|| anyhow!("Image width is zero"))?;
    let height = NonZeroU32::new(height).ok_or_else(|| anyhow!("Image height is zero"))?;
    let array_layers = NonZeroU32::new(6).unwrap();

    let texel_description = determine_valid_texel_description(pixel_format, &texture_config)?;

    let mut byte_buffer = Vec::with_capacity(
        (6 * dimensions.0 * dimensions.1 * texel_description.n_bytes()) as usize,
    );

    byte_buffer.extend_from_slice(&right_image.data);
    byte_buffer.extend_from_slice(&left_image.data);
    byte_buffer.extend_from_slice(&top_image.data);
    byte_buffer.extend_from_slice(&bottom_image.data);
    byte_buffer.extend_from_slice(&front_image.data);
    byte_buffer.extend_from_slice(&back_image.data);

    Texture::create(
        graphics_device,
        None,
        &byte_buffer,
        width,
        height,
        DepthOrArrayLayers::ArrayLayers(array_layers),
        texel_description,
        true,
        texture_config,
        label,
    )
}

/// Creates a texture array for the given loaded images, using the given
/// configuration parameters.
///
/// # Errors
/// Returns an error if:
/// - The number of images is zero.
/// - Any of the images are wrapped in an [`Err`].
/// - The image width or height is zero.
/// - The image is grayscale and the color space in the configuration is not
///   linear.
/// - The image dimensions or pixel formats do not match.
/// - The row size (width times texel size) is not a multiple of 256 bytes
///   (`wgpu` requires that rows are a multiple of 256 bytes for copying
///   data between buffers and textures).
pub fn create_texture_array_from_images<I, Im>(
    graphics_device: &GraphicsDevice,
    mipmapper_generator: &MipmapperGenerator,
    images: impl IntoIterator<IntoIter = I>,
    texture_config: TextureConfig,
    label: &str,
) -> Result<Texture>
where
    I: ExactSizeIterator<Item = Result<Im>>,
    Im: Borrow<Image>,
{
    let mut images = images.into_iter();
    let n_images = images.len();

    let first_image = images
        .next()
        .ok_or_else(|| anyhow!("No images for texture array"))??;

    let dimensions = first_image.borrow().dimensions();
    let width = NonZeroU32::new(dimensions.0).ok_or_else(|| anyhow!("Image width is zero"))?;
    let height = NonZeroU32::new(dimensions.1).ok_or_else(|| anyhow!("Image height is zero"))?;
    let array_layers = NonZeroU32::new(u32::try_from(n_images).unwrap()).unwrap();

    let pixel_format = first_image.borrow().meta.pixel_format;
    let texel_description = determine_valid_texel_description(pixel_format, &texture_config)?;

    let mut byte_buffer = Vec::with_capacity(
        n_images * (width.get() * height.get() * texel_description.n_bytes()) as usize,
    );

    byte_buffer.extend_from_slice(&first_image.borrow().data);

    for image in images {
        let image = image?;

        if image.borrow().dimensions() != dimensions {
            bail!("Inconsistent dimensions for texture array images")
        }

        if image.borrow().meta.pixel_format != pixel_format {
            bail!("Inconsistent pixel formats for texture array images")
        }

        byte_buffer.extend_from_slice(&image.borrow().data);
    }

    Texture::create(
        graphics_device,
        Some(mipmapper_generator),
        &byte_buffer,
        width,
        height,
        DepthOrArrayLayers::ArrayLayers(array_layers),
        texel_description,
        false,
        texture_config,
        label,
    )
}

/// Creates a texture from the given [`TextureCreateInfo`].
///
/// Loads image or lookup table data from files specified in the info and
/// creates the appropriate texture type (standard, array, cubemap, or lookup
/// table). Mipmaps will be generated automatically for image-based textures
/// configured for it.
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
/// - [`create_texture_array_from_images`]
/// - [`create_cubemap_texture_from_images`]
/// - [`create_texture_from_lookup_table`](lookup_table::create_texture_from_lookup_table).
#[allow(unused_variables)]
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

    let (texture, sampler_id) = match texture_info {
        TextureCreateInfo::Image(image_texture_info) => {
            let image_metadata = &image_texture_info.metadata;
            let texture_config = image_texture_info.texture_config.clone();
            let sampler_id = image_texture_info.sampler_config.as_ref().map(Into::into);

            let texture = match &image_texture_info.source {
                ImageTextureSource::Image(source) => {
                    let image = match source {
                        ImageSource::File(image_path) => Cow::Owned(
                            impact_io::image::load_image_from_path(image_path).with_context(
                                || {
                                    format!(
                                        "Failed to load texture image from {}",
                                        image_path.display()
                                    )
                                },
                            )?,
                        ),
                        ImageSource::Bytes(image) => Cow::Borrowed(image),
                    };

                    verify_image_metadata(image_metadata, &image.meta)?;

                    create_texture_from_image(
                        graphics_device,
                        mipmapper_generator,
                        &image,
                        texture_config,
                        label,
                    )?
                }
                ImageTextureSource::ArrayImages(sources) => {
                    let images = sources.iter().map(|source| {
                        let image = match source {
                            ImageSource::File(image_path) => Cow::Owned(
                                impact_io::image::load_image_from_path(image_path).with_context(
                                    || {
                                        format!(
                                            "Failed to load array texture image from {}",
                                            image_path.display()
                                        )
                                    },
                                )?,
                            ),
                            ImageSource::Bytes(image) => Cow::Borrowed(image),
                        };

                        verify_image_metadata(image_metadata, &image.meta)?;

                        Ok(image)
                    });

                    create_texture_array_from_images(
                        graphics_device,
                        mipmapper_generator,
                        images,
                        texture_config,
                        label,
                    )?
                }
                ImageTextureSource::CubemapImages {
                    right,
                    left,
                    top,
                    bottom,
                    front,
                    back,
                } => {
                    let right_image = match right {
                        ImageSource::File(image_path) => Cow::Owned(
                            impact_io::image::load_image_from_path(image_path).with_context(
                                || {
                                    format!(
                                        "Failed to load right cubemap texture image from {}",
                                        image_path.display()
                                    )
                                },
                            )?,
                        ),
                        ImageSource::Bytes(image) => Cow::Borrowed(image),
                    };
                    let left_image = match left {
                        ImageSource::File(image_path) => Cow::Owned(
                            impact_io::image::load_image_from_path(image_path).with_context(
                                || {
                                    format!(
                                        "Failed to load left cubemap texture image from {}",
                                        image_path.display()
                                    )
                                },
                            )?,
                        ),
                        ImageSource::Bytes(image) => Cow::Borrowed(image),
                    };
                    let top_image = match top {
                        ImageSource::File(image_path) => Cow::Owned(
                            impact_io::image::load_image_from_path(image_path).with_context(
                                || {
                                    format!(
                                        "Failed to load top cubemap texture image from {}",
                                        image_path.display()
                                    )
                                },
                            )?,
                        ),
                        ImageSource::Bytes(image) => Cow::Borrowed(image),
                    };
                    let bottom_image = match bottom {
                        ImageSource::File(image_path) => Cow::Owned(
                            impact_io::image::load_image_from_path(image_path).with_context(
                                || {
                                    format!(
                                        "Failed to load bottom cubemap texture image from {}",
                                        image_path.display()
                                    )
                                },
                            )?,
                        ),
                        ImageSource::Bytes(image) => Cow::Borrowed(image),
                    };
                    let front_image = match front {
                        ImageSource::File(image_path) => Cow::Owned(
                            impact_io::image::load_image_from_path(image_path).with_context(
                                || {
                                    format!(
                                        "Failed to load front cubemap texture image from {}",
                                        image_path.display()
                                    )
                                },
                            )?,
                        ),
                        ImageSource::Bytes(image) => Cow::Borrowed(image),
                    };
                    let back_image = match back {
                        ImageSource::File(image_path) => Cow::Owned(
                            impact_io::image::load_image_from_path(image_path).with_context(
                                || {
                                    format!(
                                        "Failed to load back cubemap texture image from {}",
                                        image_path.display()
                                    )
                                },
                            )?,
                        ),
                        ImageSource::Bytes(image) => Cow::Borrowed(image),
                    };

                    verify_image_metadata(image_metadata, &right_image.meta)?;
                    verify_image_metadata(image_metadata, &left_image.meta)?;
                    verify_image_metadata(image_metadata, &top_image.meta)?;
                    verify_image_metadata(image_metadata, &bottom_image.meta)?;
                    verify_image_metadata(image_metadata, &front_image.meta)?;
                    verify_image_metadata(image_metadata, &back_image.meta)?;

                    create_cubemap_texture_from_images(
                        graphics_device,
                        &right_image,
                        &left_image,
                        &top_image,
                        &bottom_image,
                        &front_image,
                        &back_image,
                        texture_config,
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
