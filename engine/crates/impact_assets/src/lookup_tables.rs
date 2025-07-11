//! Management of lookup tables.

pub mod specular_ggx_reflectance;

use crate::Assets;
use anyhow::Result;
use impact_gpu::resource_group::GPUResourceGroupManager;

/// Loads all default lookup tables into the assets as textures. The tables are
/// read from file or computed. Also creates GPU resource groups for the loaded
/// lookup table textures and their samplers.
///
/// # Errors
/// Returns an error if a computed table can not be saved to file. Additionally,
/// see
/// [`Texture::from_lookup_table`](impact_gpu::texture::Texture::from_lookup_table).
pub fn initialize_default_lookup_tables(
    assets: &mut Assets,
    gpu_resource_group_manager: &mut GPUResourceGroupManager,
) -> Result<()> {
    specular_ggx_reflectance::load_lookup_table_into_assets(assets)?;
    specular_ggx_reflectance::create_resource_group(assets, gpu_resource_group_manager);
    Ok(())
}
