//! Specular GGX reflectance lookup table.

use crate::Assets;
use anyhow::Result;
use impact_gpu::resource_group::GPUResourceGroupManager;
use impact_rendering::lookup_tables::specular_ggx_reflectance;

/// Loads the specular GGX reflectance lookup tables into the assets as a
/// texture array. The tables are read from file or computed.
///
/// # Errors
/// Returns an error if a computed table can not be saved to file. Additionally,
/// see
/// [`Texture::from_lookup_table`](impact_gpu::texture::Texture::from_lookup_table).
pub(super) fn load_lookup_table_into_assets(assets: &mut Assets) -> Result<()> {
    let file_path = assets
        .lookup_table_dir()
        .join(specular_ggx_reflectance::texture_name())
        .with_extension("bc");

    assets.load_texture_from_stored_or_computed_lookup_table(
        specular_ggx_reflectance::texture_name(),
        file_path,
        specular_ggx_reflectance::compute,
        specular_ggx_reflectance::sampler_config(),
    )?;
    Ok(())
}

/// Creates the resource group for the specular GGX reflectance lookup table
/// texture array and sampler if it does not already exist.
pub(super) fn create_resource_group(
    assets: &Assets,
    gpu_resource_group_manager: &mut GPUResourceGroupManager,
) {
    gpu_resource_group_manager
        .resource_group_entry(specular_ggx_reflectance::resource_group_id())
        .or_insert_with(|| {
            specular_ggx_reflectance::create_resource_group(&assets.graphics_device, assets)
        });
}
