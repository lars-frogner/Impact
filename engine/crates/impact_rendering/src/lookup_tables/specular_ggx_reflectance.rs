//! Specular GGX reflectance lookup table.

use crate::brdf;
use anyhow::Result;
use impact_gpu::texture::{SamplerConfig, TextureAddressingConfig, TextureFilteringConfig};
use impact_math::hash64;
use impact_texture::lookup_table::{LookupTableDeclaration, LookupTableID};
use std::{path::Path, sync::LazyLock};

const NAME: &str = "specular_ggx_reflectance_lookup_table";

static LOOKUP_TABLE_ID: LazyLock<LookupTableID> = LazyLock::new(|| LookupTableID(hash64!(NAME)));

/// Returns the ID of the specular GGX reflectance lookup table.
pub fn lookup_table_id() -> LookupTableID {
    *LOOKUP_TABLE_ID
}

/// Computes and saves the specular GGX reflectance lookup tables to the given
/// directory if they are not already present.
///
/// # Returns
/// The [`LookupTableDeclaration`] that can be used to load the lookup table
/// texture.
///
/// # Errors
/// Returns an error if a computed table can not be saved to file.
pub fn initialize(lookup_table_dir: &Path) -> Result<LookupTableDeclaration> {
    let table_path = lookup_table_dir.join(NAME).with_extension("bc");

    if !table_path.exists() {
        let table = brdf::create_specular_ggx_reflectance_lookup_tables(256, 64);
        impact_texture::io::save_lookup_table_to_file(&table, &table_path)?;
    }

    Ok(LookupTableDeclaration {
        id: *LOOKUP_TABLE_ID,
        table_path,
        sampler_config: SamplerConfig {
            addressing: TextureAddressingConfig::Clamped,
            filtering: TextureFilteringConfig::None,
        },
    })
}
