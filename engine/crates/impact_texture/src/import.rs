//! Importing textures from declarations.

use crate::{
    ImageSource, ImageTextureCreateInfo, ImageTextureSource, SamplerCreateInfo, SamplerID,
    SamplerRegistry, TextureCreateInfo, TextureID, TextureRegistry,
};
use anyhow::{Context, Result, bail};
use impact_gpu::texture::{SamplerConfig, TextureConfig};
use std::path::Path;

/// Declaration of an image-based texture.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct ImageTextureDeclaration {
    /// The ID of the texture.
    pub id: TextureID,
    /// The source of the image data.
    pub source: ImageTextureSource,
    /// Configuration for the texture.
    #[cfg_attr(feature = "serde", serde(default))]
    pub texture_config: TextureConfig,
    /// Optional configuration for the texture sampler.
    #[cfg_attr(feature = "serde", serde(default))]
    pub sampler_config: Option<SamplerConfig>,
}

impl ImageTextureDeclaration {
    /// Resolves all paths in the declaration by prepending the given root path
    /// to all paths.
    pub fn resolve_paths(&mut self, root_path: &Path) {
        self.source.resolve_paths(root_path);
    }
}

impl ImageTextureSource {
    /// Resolves all paths in the source by prepending the given root
    /// path to all paths.
    fn resolve_paths(&mut self, root_path: &Path) {
        match self {
            Self::Image(source) => {
                if let ImageSource::File(image_path) = source {
                    *image_path = root_path.join(&image_path);
                }
            }
            Self::ArrayImages(sources) => {
                for source in sources {
                    if let ImageSource::File(image_path) = source {
                        *image_path = root_path.join(&image_path);
                    }
                }
            }
            Self::CubemapImages {
                right,
                left,
                top,
                bottom,
                front,
                back,
                ..
            } => {
                if let ImageSource::File(image_path) = right {
                    *image_path = root_path.join(&image_path);
                };
                if let ImageSource::File(image_path) = left {
                    *image_path = root_path.join(&image_path);
                };
                if let ImageSource::File(image_path) = top {
                    *image_path = root_path.join(&image_path);
                };
                if let ImageSource::File(image_path) = bottom {
                    *image_path = root_path.join(&image_path);
                };
                if let ImageSource::File(image_path) = front {
                    *image_path = root_path.join(&image_path);
                };
                if let ImageSource::File(image_path) = back {
                    *image_path = root_path.join(&image_path);
                };
            }
        }
    }
}

/// Stores creation information for all textures and samplers in the given
/// declarations in the registries.
///
/// # Errors
/// See [`load_declared_image_texture`].
pub fn load_declared_image_textures(
    texture_registry: &mut TextureRegistry,
    sampler_registry: &mut SamplerRegistry,
    texture_declarations: &[ImageTextureDeclaration],
) -> Result<()> {
    for declaration in texture_declarations {
        if let Err(error) =
            load_declared_image_texture(texture_registry, sampler_registry, declaration.clone())
        {
            // Failing to load a texture is not fatal, since we might not need it
            impact_log::error!("Failed to load texture {}: {error:#}", declaration.id);
        }
    }
    Ok(())
}

/// Stores the creation information for the texture in the given declaration in
/// the texture registry. If a sampler configuration is specified, the creation
/// information for the sampler is stored in the sampler registry.
///
/// # Errors
/// Returns an error if:
/// - Another texture with the same name is already loaded.
/// - The texture metadata can not be read from the image file(s).
/// - The texture metadata is invalid or incompatible with the configuration
///   options.
pub fn load_declared_image_texture(
    texture_registry: &mut TextureRegistry,
    sampler_registry: &mut SamplerRegistry,
    declaration: ImageTextureDeclaration,
) -> Result<TextureID> {
    let texture_id = declaration.id;

    if texture_registry.contains(texture_id) {
        bail!(
            "Tried to load texture under already existing ID: {}",
            declaration.id
        );
    }

    let metadata = match &declaration.source {
        ImageTextureSource::Image(source) => match source {
            ImageSource::File(image_path) => {
                impact_log::debug!(
                    "Reading metadata for image texture `{texture_id}` from {}",
                    image_path.display(),
                );
                impact_io::image::read_metadata_for_image_at_path(image_path).with_context(|| {
                    format!(
                        "Failed to read image metadata from {}",
                        image_path.display()
                    )
                })
            }
            ImageSource::Bytes(image) => Ok(image.meta.clone()),
        },
        ImageTextureSource::ArrayImages(sources) => {
            if sources.is_empty() {
                bail!("Got empty list of sources for texture array");
            }
            // No need to check the metadata for all the images here, that will
            // be done when the texture is created
            match &sources[0] {
                ImageSource::File(image_path) => {
                    impact_log::debug!(
                        "Reading metadata for image texture array `{texture_id}` from {}",
                        image_path.display()
                    );
                    impact_io::image::read_metadata_for_image_at_path(image_path).with_context(
                        || {
                            format!(
                                "Failed to read image metadata from {}",
                                image_path.display()
                            )
                        },
                    )
                }
                ImageSource::Bytes(image) => Ok(image.meta.clone()),
            }
        }
        ImageTextureSource::CubemapImages { right, .. } => match right {
            ImageSource::File(image_path) => {
                impact_log::debug!(
                    "Reading metadata for cubemap texture `{texture_id}` from {}",
                    image_path.display()
                );
                impact_io::image::read_metadata_for_image_at_path(image_path).with_context(|| {
                    format!(
                        "Failed to read image metadata from {}",
                        image_path.display()
                    )
                })
            }
            ImageSource::Bytes(image) => Ok(image.meta.clone()),
        },
    }?;

    let image_texture_info = ImageTextureCreateInfo::new(
        declaration.source,
        metadata,
        declaration.texture_config,
        declaration.sampler_config.clone(),
    )?;

    texture_registry.insert(texture_id, TextureCreateInfo::Image(image_texture_info));

    if let Some(sampler_config) = declaration.sampler_config {
        sampler_registry.insert_with_if_absent(SamplerID::from(&sampler_config), || {
            SamplerCreateInfo {
                config: sampler_config,
            }
        });
    }

    Ok(texture_id)
}
