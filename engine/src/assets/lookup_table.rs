//! Management of lookup tables.

pub mod specular_ggx_reflectance;

use crate::assets::Assets;
use anyhow::{Context, Result};
use impact_gpu::{
    resource_group::GPUResourceGroupManager,
    texture::{TexelType, TextureLookupTable},
};
use serde::{Serialize, de::DeserializeOwned};
use std::{
    fs::File,
    io::{BufReader, Read},
    path::Path,
};

/// Loads all default lookup tables into the assets as textures. The tables are
/// read from file or computed. Also creates GPU resource groups for the loaded
/// lookup table textures and their samplers.
///
/// # Errors
/// Returns an error if a computed table can not be saved to file. Additionally,
/// see
/// [`Texture::from_lookup_table`](crate::gpu::texture::Texture::from_lookup_table).
pub fn initialize_default_lookup_tables(
    assets: &mut Assets,
    gpu_resource_group_manager: &mut GPUResourceGroupManager,
) -> Result<()> {
    specular_ggx_reflectance::load_lookup_table_into_assets(assets)?;
    specular_ggx_reflectance::create_resource_group(assets, gpu_resource_group_manager);
    Ok(())
}

/// Serializes a lookup table into the `Bincode` format and saves it at the
/// given path.
pub fn save_lookup_table_to_file<T>(
    table: &TextureLookupTable<T>,
    output_file_path: impl AsRef<Path>,
) -> Result<()>
where
    T: TexelType + Serialize,
{
    let byte_buffer = bincode::serde::encode_to_vec(table, bincode::config::standard())?;
    impact_io::save_data_as_binary(output_file_path, &byte_buffer)?;
    Ok(())
}

/// Loads and returns the `Bincode` serialized lookup table at the given path.
pub fn read_lookup_table_from_file<T>(file_path: impl AsRef<Path>) -> Result<TextureLookupTable<T>>
where
    T: TexelType + DeserializeOwned,
{
    let file_path = file_path.as_ref();
    let file = File::open(file_path).with_context(|| {
        format!(
            "Failed to open texture lookup table at {}",
            file_path.display()
        )
    })?;
    let mut reader = BufReader::new(file);
    let mut buffer = Vec::new();
    reader.read_to_end(&mut buffer)?;
    let (table, _) = bincode::serde::decode_from_slice(&buffer, bincode::config::standard())?;
    Ok(table)
}
