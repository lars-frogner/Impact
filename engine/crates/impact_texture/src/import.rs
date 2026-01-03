//! Importing textures from declarations.

use crate::{
    ImageSource, ImageTextureCreateInfo, ImageTextureSource, SamplerCreateInfo, SamplerID,
    SamplerRegistry, TextureArrayUsage, TextureCreateInfo, TextureID, TextureRegistry,
    processing::ImageProcessing,
};
use anyhow::{Context, Result, bail};
use impact_gpu::texture::{SamplerConfig, TextureConfig};
use std::path::{Path, PathBuf};

/// Declaration of an image-based texture.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct ImageTextureDeclaration {
    /// The ID of the texture.
    pub id: TextureID,
    /// The source of the image data.
    pub source: DeclaredImageTextureSource,
    /// Configuration for the texture.
    #[cfg_attr(feature = "serde", serde(default))]
    pub texture_config: TextureConfig,
    /// Optional configuration for the texture sampler.
    #[cfg_attr(feature = "serde", serde(default))]
    pub sampler_config: Option<SamplerConfig>,
    /// Image processing to perform on the texture before use.
    #[cfg_attr(feature = "serde", serde(default))]
    pub processing: ImageProcessing,
}

/// Source for image-based texture data.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub enum DeclaredImageTextureSource {
    Image(PathBuf),
    ArrayImages(Vec<PathBuf>),
    CubemapImages {
        right: PathBuf,
        left: PathBuf,
        top: PathBuf,
        bottom: PathBuf,
        front: PathBuf,
        back: PathBuf,
    },
}

impl ImageTextureDeclaration {
    /// Resolves all paths in the declaration by prepending the given root path
    /// to all paths.
    pub fn resolve_paths(&mut self, root_path: &Path) {
        self.source.resolve_paths(root_path);
    }
}

impl DeclaredImageTextureSource {
    /// Resolves all paths in the source by prepending the given root
    /// path to all paths.
    fn resolve_paths(&mut self, root_path: &Path) {
        match self {
            Self::Image(image_path) => {
                *image_path = root_path.join(&image_path);
            }
            Self::ArrayImages(image_paths) => {
                for image_path in image_paths {
                    *image_path = root_path.join(&image_path);
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
                *right = root_path.join(&right);
                *left = root_path.join(&left);
                *top = root_path.join(&top);
                *bottom = root_path.join(&bottom);
                *front = root_path.join(&front);
                *back = root_path.join(&back);
            }
        }
    }
}

impl From<DeclaredImageTextureSource> for ImageTextureSource {
    fn from(source: DeclaredImageTextureSource) -> Self {
        match source {
            DeclaredImageTextureSource::Image(image_path) => {
                ImageTextureSource::Single(ImageSource::File(image_path))
            }
            DeclaredImageTextureSource::ArrayImages(image_paths) => ImageTextureSource::Array {
                sources: image_paths.into_iter().map(ImageSource::File).collect(),
                usage: TextureArrayUsage::Generic,
            },
            DeclaredImageTextureSource::CubemapImages {
                right,
                left,
                top,
                bottom,
                front,
                back,
            } => ImageTextureSource::Array {
                sources: [right, left, top, bottom, front, back]
                    .map(ImageSource::File)
                    .to_vec(),
                usage: TextureArrayUsage::Cubemap,
            },
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
            log::error!("Failed to load texture {}: {error:#}", declaration.id);
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
    load_image_texture(
        texture_registry,
        sampler_registry,
        declaration.id,
        declaration.source.into(),
        declaration.texture_config,
        declaration.sampler_config,
        declaration.processing,
    )?;
    Ok(declaration.id)
}

/// Stores the creation information for the texture with the given ID, source
/// and configuration in the texture registry. If a sampler configuration is
/// specified, the creation information for the sampler is stored in the sampler
/// registry.
///
/// # Errors
/// Returns an error if:
/// - Another texture with the same name is already loaded.
/// - The texture metadata can not be read from the image file(s).
/// - The texture metadata is invalid or incompatible with the configuration
///   options.
pub fn load_image_texture(
    texture_registry: &mut TextureRegistry,
    sampler_registry: &mut SamplerRegistry,
    texture_id: TextureID,
    source: ImageTextureSource,
    texture_config: TextureConfig,
    sampler_config: Option<SamplerConfig>,
    processing: ImageProcessing,
) -> Result<()> {
    if texture_registry.contains(texture_id) {
        bail!("Tried to load texture under already existing ID: {texture_id}");
    }

    let metadata = match &source {
        ImageTextureSource::Single(source) => match source {
            ImageSource::File(path) => {
                log::debug!(
                    "Reading metadata for image texture `{texture_id}` from {}",
                    path.display(),
                );
                impact_io::image::read_metadata_for_image_at_path(path).with_context(|| {
                    format!("Failed to read image metadata from {}", path.display())
                })?
            }
            ImageSource::Bytes(image) => image.meta.clone(),
        },
        ImageTextureSource::Array { sources, .. } => {
            if sources.is_empty() {
                bail!("Got empty list of sources for texture array");
            }
            // No need to check the metadata for all the images here, that will
            // be done when the texture is created
            let source = &sources[0];

            match source {
                ImageSource::File(path) => {
                    log::debug!(
                        "Reading metadata for image texture array `{texture_id}` from {}",
                        path.display()
                    );
                    impact_io::image::read_metadata_for_image_at_path(path).with_context(|| {
                        format!("Failed to read image metadata from {}", path.display())
                    })?
                }
                ImageSource::Bytes(image) => image.meta.clone(),
            }
        }
    };

    let image_texture_info = ImageTextureCreateInfo::new(
        source,
        metadata,
        texture_config,
        sampler_config.clone(),
        processing,
    )?;

    texture_registry.insert(texture_id, TextureCreateInfo::Image(image_texture_info));

    if let Some(sampler_config) = sampler_config {
        sampler_registry.insert_with_if_absent(SamplerID::from(&sampler_config), || {
            SamplerCreateInfo {
                config: sampler_config,
            }
        });
    }

    Ok(())
}
