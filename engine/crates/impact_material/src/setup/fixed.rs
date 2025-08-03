//! Materials with a fixed color or texture.

use crate::{
    Material, MaterialBindGroupSlot, MaterialBindGroupTemplate, MaterialID, MaterialRegistry,
    MaterialTemplate, MaterialTemplateID, MaterialTemplateRegistry,
    MaterialTextureBindingLocations, MaterialTextureGroup, MaterialTextureGroupID,
    MaterialTextureGroupRegistry, RGBColor, values::MaterialPropertyValues,
};
use anyhow::{Result, anyhow};
use bytemuck::{Pod, Zeroable};
use impact_math::hash64;
use impact_mesh::VertexAttributeSet;
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

pub fn setup_fixed_material(
    texture_registry: &TextureRegistry,
    sampler_registry: &SamplerRegistry,
    material_registry: &mut MaterialRegistry,
    material_template_registry: &mut MaterialTemplateRegistry,
    material_texture_group_registry: &mut MaterialTextureGroupRegistry,
    properties: FixedMaterialProperties,
    material_id: Option<MaterialID>,
) -> Result<MaterialID> {
    let material_id = material_id.unwrap_or_else(|| MaterialID(hash64!(format!("{properties:?}"))));

    if material_registry.contains(material_id) {
        return Ok(material_id);
    }

    match properties.color {
        Color::Uniform(FixedColor(color)) => {
            let property_values =
                MaterialPropertyValues::from_fixed_material_properties(Some(color));

            let template = MaterialTemplate {
                vertex_attribute_requirements: VertexAttributeSet::empty(),
                bind_group_template: MaterialBindGroupTemplate::empty(),
                texture_binding_locations: MaterialTextureBindingLocations::Fixed(
                    FixedMaterialTextureBindingLocations {
                        color_texture_and_sampler_bindings: None,
                    },
                ),
                property_flags: property_values.flags(),
                instance_feature_type_id: property_values.instance_feature_type_id(),
            };

            let template_id = MaterialTemplateID::for_template(&template);

            let material = Material {
                template_id,
                texture_group_id: MaterialTextureGroupID::empty(),
                property_values,
            };

            material_registry.insert(material_id, material);

            material_template_registry.insert_with_if_absent(template_id, || template);

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

            let property_values = MaterialPropertyValues::from_fixed_material_properties(None);

            let template = MaterialTemplate {
                vertex_attribute_requirements: VertexAttributeSet::TEXTURE_COORDS,
                bind_group_template,
                texture_binding_locations,
                property_flags: property_values.flags(),
                instance_feature_type_id: property_values.instance_feature_type_id(),
            };

            let template_id = MaterialTemplateID::for_template(&template);
            let texture_group_id = MaterialTextureGroupID::from_texture_ids(&[texture_id]);

            let material = Material {
                template_id,
                texture_group_id,
                property_values,
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
