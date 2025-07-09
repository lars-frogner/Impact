//! Materials with a fixed color or texture.

use crate::{
    MaterialHandle, MaterialID, MaterialInstanceFeatureFlags, MaterialLibrary,
    MaterialPropertyTextureGroup, MaterialPropertyTextureGroupID, MaterialShaderInput,
    MaterialSpecification, MaterialTextureProvider, RGBColor, features::FixedColorMaterialFeature,
};
use anyhow::Result;
use bytemuck::{Pod, Zeroable};
use impact_gpu::{device::GraphicsDevice, texture::TextureID};
use impact_math::hash64;
use impact_mesh::VertexAttributeSet;
use impact_model::{InstanceFeature, InstanceFeatureManager};
use roc_integration::roc;
use std::{collections::hash_map::Entry, hash::Hash, sync::LazyLock};

define_setup_type! {
    target = MaterialHandle;
    /// A fixed, uniform color that is independent of lighting.
    #[roc(parents = "Setup")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct FixedColor(pub RGBColor);
}

define_setup_type! {
    target = MaterialHandle;
    /// A fixed, textured color that is independent of lighting.
    #[roc(parents = "Setup")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct FixedTexture(pub TextureID);
}

/// Binding locations for textures used in a fixed material.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FixedMaterialTextureBindings {
    pub color_texture_and_sampler_bindings: Option<(u32, u32)>,
}

static FIXED_COLOR_MATERIAL_ID: LazyLock<MaterialID> =
    LazyLock::new(|| MaterialID(hash64!("FixedColorMaterial")));
static FIXED_TEXTURE_MATERIAL_ID: LazyLock<MaterialID> =
    LazyLock::new(|| MaterialID(hash64!("FixedTextureMaterial")));

pub fn setup_fixed_color_material<MID: Clone + Eq + Hash>(
    material_library: &mut MaterialLibrary,
    instance_feature_manager: &mut InstanceFeatureManager<MID>,
    fixed_color: &FixedColor,
    desynchronized: &mut bool,
) -> MaterialHandle {
    let feature_id = instance_feature_manager
        .get_storage_mut::<FixedColorMaterialFeature>()
        .expect("Missing storage for FixedColorMaterialFeature features")
        .add_feature(&FixedColorMaterialFeature::new(fixed_color.0));

    material_library
        .material_specification_entry(*FIXED_COLOR_MATERIAL_ID)
        .or_insert_with(|| {
            MaterialSpecification::new(
                VertexAttributeSet::empty(),
                vec![FixedColorMaterialFeature::FEATURE_TYPE_ID],
                MaterialInstanceFeatureFlags::HAS_COLOR,
                None,
                MaterialShaderInput::Fixed(FixedMaterialTextureBindings {
                    color_texture_and_sampler_bindings: None,
                }),
            )
        });

    *desynchronized = true;

    MaterialHandle::new(*FIXED_COLOR_MATERIAL_ID, Some(feature_id), None)
}

pub fn setup_fixed_texture_material(
    graphics_device: &GraphicsDevice,
    texture_provider: &impl MaterialTextureProvider,
    material_library: &mut MaterialLibrary,
    fixed_texture: &FixedTexture,
) -> Result<MaterialHandle> {
    material_library
        .material_specification_entry(*FIXED_TEXTURE_MATERIAL_ID)
        .or_insert_with(|| {
            MaterialSpecification::new(
                VertexAttributeSet::TEXTURE_COORDS,
                Vec::new(),
                MaterialInstanceFeatureFlags::empty(),
                None,
                MaterialShaderInput::Fixed(FixedMaterialTextureBindings {
                    color_texture_and_sampler_bindings: Some(
                        MaterialPropertyTextureGroup::get_texture_and_sampler_bindings(0),
                    ),
                }),
            )
        });

    let texture_ids = vec![fixed_texture.0];

    let texture_group_id = MaterialPropertyTextureGroupID::from_texture_ids(&texture_ids);

    if let Entry::Vacant(entry) =
        material_library.material_property_texture_group_entry(texture_group_id)
    {
        entry.insert(MaterialPropertyTextureGroup::new(
            graphics_device,
            texture_provider,
            texture_ids,
            texture_group_id.to_string(),
        )?);
    };

    Ok(MaterialHandle::new(
        *FIXED_TEXTURE_MATERIAL_ID,
        None,
        Some(texture_group_id),
    ))
}
