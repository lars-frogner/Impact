//! Materials with a fixed color or texture.

use crate::{
    Material, MaterialBindGroupSlot, MaterialBindGroupTemplate, MaterialID,
    MaterialInstanceFeatureFlags, MaterialRegistry, MaterialTemplate, MaterialTemplateID,
    MaterialTemplateRegistry, MaterialTextureBindingLocations, MaterialTextureGroup,
    MaterialTextureGroupID, MaterialTextureGroupRegistry, RGBColor,
    features::FixedColorMaterialFeature,
};
use anyhow::{Result, anyhow};
use bytemuck::{Pod, Zeroable};
use impact_math::hash64;
use impact_mesh::VertexAttributeSet;
use impact_model::{InstanceFeatureID, InstanceFeatureTypeID, ModelInstanceManager};
use impact_texture::{SamplerRegistry, TextureID, TextureRegistry};
use roc_integration::roc;
use std::hash::Hash;

define_setup_type! {
    target = MaterialID;
    /// A fixed, uniform color that is independent of lighting.
    #[roc(parents = "Setup")]
    #[repr(C)]
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct FixedColor(pub RGBColor);
}

define_setup_type! {
    target = MaterialID;
    /// A fixed, textured color that is independent of lighting.
    #[roc(parents = "Setup")]
    #[repr(C)]
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct FixedTexture(pub TextureID);
}

/// A complete specification of the properties of a fixed material that is
/// independent of lighting.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct FixedMaterialProperties {
    pub color: Color,
}

/// A fixed, uniform or textured color that is independent of lighting.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub enum Color {
    Uniform(FixedColor),
    Textured(FixedTexture),
}

/// Binding locations for textures used in a fixed material.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FixedMaterialTextureBindingLocations {
    pub color_texture_and_sampler_bindings: Option<(u32, u32)>,
}

pub fn setup_fixed_material<MID: Copy + Eq + Hash>(
    texture_registry: &TextureRegistry,
    sampler_registry: &SamplerRegistry,
    material_registry: &mut MaterialRegistry,
    material_template_registry: &mut MaterialTemplateRegistry,
    material_texture_group_registry: &mut MaterialTextureGroupRegistry,
    model_instance_manager: &mut ModelInstanceManager<MID>,
    properties: FixedMaterialProperties,
    material_id: Option<MaterialID>,
    desynchronized: &mut bool,
) -> Result<MaterialID> {
    let material_id = material_id.unwrap_or_else(|| MaterialID(hash64!(format!("{properties:?}"))));

    if material_registry.contains(material_id) {
        return Ok(material_id);
    }

    match properties.color {
        Color::Uniform(FixedColor(color)) => {
            let instance_feature_id = model_instance_manager
                .get_storage_mut::<FixedColorMaterialFeature>()
                .expect("Missing storage for FixedColorMaterialFeature features")
                .add_feature(&FixedColorMaterialFeature::new(color));

            let template = MaterialTemplate {
                vertex_attribute_requirements: VertexAttributeSet::empty(),
                bind_group_template: MaterialBindGroupTemplate::empty(),
                texture_binding_locations: MaterialTextureBindingLocations::Fixed(
                    FixedMaterialTextureBindingLocations {
                        color_texture_and_sampler_bindings: None,
                    },
                ),
                instance_feature_type_id: instance_feature_id.feature_type_id(),
                instance_feature_flags: MaterialInstanceFeatureFlags::HAS_COLOR,
            };

            let template_id = MaterialTemplateID::for_template(&template);

            let material = Material {
                template_id,
                texture_group_id: MaterialTextureGroupID::empty(),
                instance_feature_id,
            };

            material_registry.insert(material_id, material);

            material_template_registry.insert_with_if_absent(template_id, || template);

            *desynchronized = true;

            Ok(material_id)
        }
        Color::Textured(FixedTexture(texture_id)) => {
            let texture = texture_registry
                .get(texture_id)
                .ok_or_else(|| anyhow!("Missing color texture {texture_id} for fixed material"))?;

            let sampler = sampler_registry
                .get(texture.sampler_id().ok_or_else(|| {
                    anyhow!("Fixed material color texture {texture_id} has no associated sampler")
                })?)
                .ok_or_else(|| {
                    anyhow!("Missing sampler for fixed material color texture {texture_id}")
                })?;

            let bind_group_template = MaterialBindGroupTemplate {
                slots: vec![MaterialBindGroupSlot {
                    texture: texture.bind_group_layout_entry_props(),
                    sampler: sampler.bind_group_layout_entry_props(),
                }],
            };

            let texture_binding_locations =
                MaterialTextureBindingLocations::Fixed(FixedMaterialTextureBindingLocations {
                    color_texture_and_sampler_bindings: Some(
                        MaterialBindGroupTemplate::get_texture_and_sampler_bindings(0),
                    ),
                });

            let template = MaterialTemplate {
                vertex_attribute_requirements: VertexAttributeSet::TEXTURE_COORDS,
                bind_group_template,
                texture_binding_locations,
                instance_feature_type_id: InstanceFeatureTypeID::not_applicable(),
                instance_feature_flags: MaterialInstanceFeatureFlags::empty(),
            };

            let template_id = MaterialTemplateID::for_template(&template);
            let texture_group_id = MaterialTextureGroupID::from_texture_ids(&[texture_id]);

            let material = Material {
                template_id,
                texture_group_id,
                instance_feature_id: InstanceFeatureID::not_applicable(),
            };

            material_registry.insert(material_id, material);

            material_template_registry.insert_with_if_absent(template_id, || template);

            material_texture_group_registry.insert_with_if_absent(texture_group_id, || {
                MaterialTextureGroup {
                    template_id,
                    texture_ids: vec![texture_id],
                }
            });

            Ok(material_id)
        }
    }
}
