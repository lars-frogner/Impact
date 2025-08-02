//! Lookup tables for rendering;

pub mod specular_ggx_reflectance;

use anyhow::Result;
use impact_texture::{
    SamplerRegistry, TextureRegistry,
    lookup_table::{self, LookupTableRegistry},
};
use std::path::Path;

/// Loads the resources for all default lookup tables for rendering into the
/// appropriate registries. The tables are computed and saved to the given
/// directory if they are not already present there.
pub fn initialize_default_lookup_tables(
    texture_registry: &mut TextureRegistry,
    sampler_registry: &mut SamplerRegistry,
    lookup_table_registry: &mut LookupTableRegistry,
    lookup_table_dir: &Path,
) -> Result<()> {
    let specular_ggx_reflectance_table = specular_ggx_reflectance::initialize(lookup_table_dir)?;

    lookup_table::load_declared_lookup_table(
        texture_registry,
        sampler_registry,
        lookup_table_registry,
        specular_ggx_reflectance_table,
    )?;

    Ok(())
}
