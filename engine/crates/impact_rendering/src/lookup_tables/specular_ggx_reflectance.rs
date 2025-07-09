//! Specular GGX reflectance lookup table.

use crate::brdf;
use impact_gpu::{
    bind_group_layout::BindGroupLayoutRegistry,
    device::GraphicsDevice,
    resource_group::{GPUResourceGroup, GPUResourceGroupID},
    texture::{
        self, SamplerConfig, SamplerID, TexelDescription, TextureAddressingConfig,
        TextureFilteringConfig,
    },
    texture::{TextureID, TextureLookupTable},
    wgpu,
};
use impact_material::MaterialTextureProvider;
use impact_math::{ConstStringHash64, hash32, hash64};
use std::sync::LazyLock;

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

const BIND_GROUP_LAYOUT_ID: ConstStringHash64 =
    ConstStringHash64::new("SpecularGGXReflectanceLookupTable");

/// Returns the name of the specular GGX reflectance lookup table texture.
pub const fn texture_name() -> &'static str {
    TEXTURE_NAME
}

/// Returns the sampler configuration for the specular GGX reflectance lookup
/// table texture.
pub const fn sampler_config() -> SamplerConfig {
    SAMPLER_CONFIG
}

/// Returns the resource group ID for the specular GGX reflectance lookup table
/// texture array and sampler.
pub fn resource_group_id() -> GPUResourceGroupID {
    *RESOURCE_GROUP_ID
}

/// Computes the specular GGX reflectance lookup table;
pub fn compute() -> TextureLookupTable<f32> {
    brdf::create_specular_ggx_reflectance_lookup_tables(1024, 512)
}

/// Creates the resource group for the specular GGX reflectance lookup table
/// texture array and sampler if it does not already exist.
pub fn create_resource_group(
    graphics_device: &GraphicsDevice,
    texture_provider: &impl MaterialTextureProvider,
) -> GPUResourceGroup {
    let texture = texture_provider.get_texture(&TEXTURE_ID).unwrap();
    let sampler = texture_provider.get_sampler(&SAMPLER_ID).unwrap();
    GPUResourceGroup::new(
        graphics_device,
        Vec::new(),
        &[],
        &[texture],
        &[sampler],
        VISIBILITY,
        "Specular GGX reflectance lookup table",
    )
}

/// Returns the bind group layout for the specular GGX reflectance lookup table
/// texture array and sampler. The bind group layout is created and cached if it
/// does not already exist. The layout is compatible with the bind group
/// associated with the GPU resource group (whose ID can be obtained by calling
/// [`resource_group_id`]).
pub fn get_or_create_texture_and_sampler_bind_group_layout(
    graphics_device: &GraphicsDevice,
    bind_group_layout_registry: &BindGroupLayoutRegistry,
) -> wgpu::BindGroupLayout {
    bind_group_layout_registry.get_or_create_layout(BIND_GROUP_LAYOUT_ID, || {
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
