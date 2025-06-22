//! Specular GGX reflectance lookup table.

use crate::{
    assets::{Assets, TextureID},
    gpu::rendering::brdf,
};
use anyhow::Result;
use impact_gpu::{
    device::GraphicsDevice,
    resource_group::{GPUResourceGroup, GPUResourceGroupID, GPUResourceGroupManager},
    texture::{
        self, SamplerConfig, SamplerID, TexelDescription, TextureAddressingConfig,
        TextureFilteringConfig,
    },
};
use impact_math::{hash32, hash64};
use std::sync::{LazyLock, OnceLock};

const TEXTURE_NAME: &str = "specular_ggx_reflectance_lookup_table";

const SAMPLER_CONFIG: SamplerConfig = SamplerConfig {
    addressing: TextureAddressingConfig::Clamped,
    filtering: TextureFilteringConfig::None,
};

const VISIBILITY: wgpu::ShaderStages = wgpu::ShaderStages::FRAGMENT;

static TEXTURE_ID: LazyLock<TextureID> = LazyLock::new(|| TextureID(hash32!(TEXTURE_NAME)));

static SAMPLER_ID: LazyLock<SamplerID> = LazyLock::new(|| (&SAMPLER_CONFIG).into());

static RESOURCE_GROUP_ID: LazyLock<GPUResourceGroupID> =
    LazyLock::new(|| GPUResourceGroupID(hash64!("SpecularGGXReflectanceLookupTable")));

static BIND_GROUP_LAYOUT: OnceLock<wgpu::BindGroupLayout> = OnceLock::new();

/// Returns the resource group ID for the specular GGX reflectance lookup table
/// texture array and sampler.
pub fn resource_group_id() -> GPUResourceGroupID {
    *RESOURCE_GROUP_ID
}

/// Loads the specular GGX reflectance lookup tables into the assets as a
/// texture array. The tables are read from file or computed.
///
/// # Errors
/// Returns an error if a computed table can not be saved to file. Additionally,
/// see
/// [`Texture::from_lookup_table`](crate::gpu::texture::Texture::from_lookup_table).
pub(super) fn load_lookup_table_into_assets(assets: &mut Assets) -> Result<()> {
    let file_path = assets
        .lookup_table_dir()
        .join(TEXTURE_NAME)
        .with_extension("bc");

    assets.load_texture_from_stored_or_computed_lookup_table(
        TEXTURE_NAME,
        file_path,
        || brdf::create_specular_ggx_reflectance_lookup_tables(1024, 512),
        SAMPLER_CONFIG,
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
        .resource_group_entry(*RESOURCE_GROUP_ID)
        .or_insert_with(|| {
            let texture = assets.textures.get(&*TEXTURE_ID).unwrap();
            let sampler = assets.samplers.get(&*SAMPLER_ID).unwrap();
            GPUResourceGroup::new(
                &assets.graphics_device,
                Vec::new(),
                &[],
                &[texture],
                &[sampler],
                VISIBILITY,
                "Specular GGX reflectance lookup table",
            )
        });
}

/// Returns the bind group layout for the specular GGX reflectance lookup table
/// texture array and sampler. The bind group layout is created and cached if it
/// does not already exist. The layout is compatible with the bind group
/// associated with the GPU resource group (whose ID can be obtained by calling
/// [`resource_group_id`]).
pub fn get_or_create_texture_and_sampler_bind_group_layout(
    graphics_device: &GraphicsDevice,
) -> &'static wgpu::BindGroupLayout {
    BIND_GROUP_LAYOUT.get_or_init(|| {
        let texture_entry = texture::create_texture_bind_group_layout_entry(
            0,
            VISIBILITY,
            TexelDescription::Float32.texture_format(),
            wgpu::TextureViewDimension::D2Array,
        );
        let sampler_entry = texture::create_sampler_bind_group_layout_entry(
            1,
            VISIBILITY,
            wgpu::SamplerBindingType::NonFiltering,
        );
        graphics_device
            .device()
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[texture_entry, sampler_entry],
                label: Some("Specular GGX reflectance lookup table bind group layout"),
            })
    })
}
