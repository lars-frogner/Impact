//! Lookup tables that can be loaded into textures.

use crate::{SamplerID, TextureID};
use anyhow::{Result, bail};
use impact_gpu::{
    device::GraphicsDevice,
    texture::{
        ColorSpace, DepthOrArrayLayers, SamplerConfig, TexelDescription, TexelType, Texture,
        TextureConfig,
    },
};
use impact_math::stringhash64_newtype;
use impact_resource::{Resource, ResourceID, registry::ImmutableResourceRegistry};
use std::{
    num::{NonZeroU32, NonZeroU64},
    path::PathBuf,
};

stringhash64_newtype!(
    /// Identifier for a lookup table.
    [pub] LookupTableID
);

/// A registry of [`LookupTableBindingInfo`]s.
pub type LookupTableRegistry = ImmutableResourceRegistry<LookupTableBindingInfo>;

/// Declaration of a lookup table.
#[derive(Clone, Debug)]
pub struct LookupTableDeclaration {
    pub id: LookupTableID,
    pub table_path: PathBuf,
    pub sampler_config: SamplerConfig,
}

/// Contains the information required to create a bind group for a lookup
/// table's texture and sampler.
#[derive(Clone, Debug)]
pub struct LookupTableBindingInfo {
    id: LookupTableID,
    metadata: LookupTableMetadata,
    sampler_config: SamplerConfig,
}

/// Contains the information required to create a specific lookup table-based
/// [`Texture`] and the layout entry for bind groups containing the texture.
#[derive(Clone, Debug)]
pub struct LookupTableTextureCreateInfo {
    pub table_path: PathBuf,
    pub metadata: LookupTableMetadata,
    pub sampler_config: SamplerConfig,
}

/// A lookup table that can be loaded into a texture.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct LookupTable<T: TexelType> {
    meta: LookupTableMetadata,
    values: Vec<T>,
}

/// Dimensions and format for a [`LookupTable`].
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LookupTableMetadata {
    pub width: NonZeroU32,
    pub height: NonZeroU32,
    pub depth_or_array_layers: DepthOrArrayLayers,
    pub value_type: LookupTableValueType,
}

/// The data type of the values in a [`LookupTable`].
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LookupTableValueType {
    Float32,
    Unsigned8,
}

impl ResourceID for LookupTableID {}

impl Resource for LookupTableBindingInfo {
    type ID = LookupTableID;
}

impl LookupTableBindingInfo {
    pub fn new(
        id: LookupTableID,
        metadata: LookupTableMetadata,
        sampler_config: SamplerConfig,
    ) -> Self {
        Self {
            id,
            metadata,
            sampler_config,
        }
    }

    pub fn id(&self) -> LookupTableID {
        self.id
    }

    pub fn texture_id(&self) -> TextureID {
        TextureID(self.id.0)
    }

    pub fn sampler_id(&self) -> SamplerID {
        SamplerID::from(&self.sampler_config)
    }

    pub fn metadata(&self) -> &LookupTableMetadata {
        &self.metadata
    }

    pub fn sampler_config(&self) -> &SamplerConfig {
        &self.sampler_config
    }
}

impl<T: TexelType> LookupTable<T> {
    /// Defines a lookup table with the given metadata and data. The table is
    /// considered an array of 2D subtables if `depth_or_array_layers` is
    /// `ArrayLayers`, otherwise it is considered 1D if `depth_or_array_layers`
    /// and `height` are 1, 2D if only `depth_or_array_layers` is 1 and 3D
    /// otherwise. The lookup values in the `values` vector are assumed to be
    /// laid out in row-major order, with adjacent values varying in width
    /// first, then height and finally depth.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The value type in `meta` is inconsistent with `T`.
    /// - The table shape is inconsistent with the number of data values.
    pub fn new(meta: LookupTableMetadata, values: Vec<T>) -> Result<Self> {
        if meta.value_type.texel_description() != T::DESCRIPTION {
            bail!(
                "Lookup table value type in metadata ({:?}) inconsistent with actual data type ({:?})",
                meta.value_type.texel_description(),
                T::DESCRIPTION
            );
        }
        if meta.n_values().get() != values.len() as u64 {
            bail!(
                "Lookup table shape ({}, {}, {:?}) inconsistent with number of data values ({})",
                meta.width,
                meta.height,
                meta.depth_or_array_layers,
                values.len()
            );
        }
        Ok(Self { meta, values })
    }

    pub fn metadata(&self) -> &LookupTableMetadata {
        &self.meta
    }
}

impl LookupTableMetadata {
    /// Returns the total number of values in the lookup table.
    pub fn n_values(&self) -> NonZeroU64 {
        let n_values = u64::from(self.width.get())
            * u64::from(self.height.get())
            * u64::from(self.depth_or_array_layers.unwrap().get());

        NonZeroU64::new(n_values).unwrap()
    }
}

impl LookupTableValueType {
    pub fn texel_description(&self) -> TexelDescription {
        match self {
            Self::Float32 => TexelDescription::Float32,
            Self::Unsigned8 => TexelDescription::Grayscale8,
        }
    }
}

/// Creates a texture holding the given lookup table. The texture will be
/// sampled with (bi/tri)linear interpolation, and lookups outside [0, 1]
/// are clamped to the edge values.
///
/// # Errors
/// Returns an error if the row size (width times data value size) is not a
/// multiple of 256 bytes (`wgpu` requires that rows are a multiple of 256
/// bytes for copying data between buffers and textures).
pub fn create_texture_from_lookup_table<T: TexelType>(
    graphics_device: &GraphicsDevice,
    table: &LookupTable<T>,
    label: &str,
) -> Result<Texture> {
    let byte_buffer = bytemuck::cast_slice(&table.values);

    let texture_config = TextureConfig {
        color_space: ColorSpace::Linear,
        ..Default::default()
    };

    Texture::create(
        graphics_device,
        None,
        byte_buffer,
        table.meta.width,
        table.meta.height,
        table.meta.depth_or_array_layers,
        T::DESCRIPTION,
        false,
        texture_config,
        label,
    )
}

/// Stores binding information for the lookup table in the given declaration as
/// well as creation information for the associated texture and sampler in the
/// appropriate registries.
///
/// # Errors
/// Returns an error if:
/// - A lookup table or texture with the same ID already is loaded.
/// - The metadata for the lookup table file can not be read.
#[cfg(feature = "postcard")]
pub fn load_declared_lookup_table(
    texture_registry: &mut crate::TextureRegistry,
    sampler_registry: &mut crate::SamplerRegistry,
    lookup_table_registry: &mut LookupTableRegistry,
    declaration: LookupTableDeclaration,
) -> Result<()> {
    use anyhow::Context;

    log::debug!(
        "Loading lookup table `{}` from {}",
        declaration.id,
        declaration.table_path.display(),
    );

    if lookup_table_registry.contains(declaration.id) {
        bail!(
            "Tried to load lookup table under already existing ID: {}",
            declaration.id
        );
    }

    let metadata = crate::io::read_lookup_table_metadata_from_file(&declaration.table_path)
        .with_context(|| {
            format!(
                "Failed to read lookup table metadata from {}",
                declaration.table_path.display()
            )
        })?;

    let binding_info =
        LookupTableBindingInfo::new(declaration.id, metadata.clone(), declaration.sampler_config);

    let texture_id = binding_info.texture_id();
    let sampler_id = binding_info.sampler_id();

    if texture_registry.contains(texture_id) {
        bail!("Tried to load lookup table texture using already existing texture ID: {texture_id}");
    }

    texture_registry.insert(
        texture_id,
        crate::TextureCreateInfo::LookupTable(LookupTableTextureCreateInfo {
            table_path: declaration.table_path,
            metadata,
            sampler_config: binding_info.sampler_config().clone(),
        }),
    );

    sampler_registry.insert_with_if_absent(sampler_id, || crate::SamplerCreateInfo {
        config: binding_info.sampler_config().clone(),
    });

    lookup_table_registry.insert(binding_info.id(), binding_info);

    Ok(())
}
