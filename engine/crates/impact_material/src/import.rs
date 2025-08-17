//! Importing materials from declarations.

use crate::{
    MaterialID, MaterialRegistry, MaterialTemplateRegistry, MaterialTextureGroupRegistry,
    setup::{fixed::FixedMaterialProperties, physical::PhysicalMaterialProperties},
};
use anyhow::Result;
use impact_texture::{SamplerRegistry, TextureRegistry};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct MaterialDeclaration {
    pub id: MaterialID,
    pub properties: MaterialProperties,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub enum MaterialProperties {
    Fixed(FixedMaterialProperties),
    Physical(PhysicalMaterialProperties),
}

/// Loads all materials in the given declarations and stores them in the
/// material registries.
///
/// # Errors
/// See [`load_declared_material`].
pub fn load_declared_materials(
    texture_registry: &TextureRegistry,
    sampler_registry: &SamplerRegistry,
    material_registry: &mut MaterialRegistry,
    material_template_registry: &mut MaterialTemplateRegistry,
    material_texture_group_registry: &mut MaterialTextureGroupRegistry,
    material_declarations: &[MaterialDeclaration],
) -> Result<()> {
    for declaration in material_declarations {
        if let Err(error) = load_declared_material(
            texture_registry,
            sampler_registry,
            material_registry,
            material_template_registry,
            material_texture_group_registry,
            declaration,
        ) {
            // Failing to load a material is not fatal, since we might not need it
            impact_log::error!("Failed to load material {}: {error:#}", declaration.id);
        }
    }
    Ok(())
}

/// Loads the material in the given declaration and stores it in the material
/// registries.
///
/// # Errors
/// Returns an error if a texture and/or sampler referenced by the material is
/// missing.
pub fn load_declared_material(
    texture_registry: &TextureRegistry,
    sampler_registry: &SamplerRegistry,
    material_registry: &mut MaterialRegistry,
    material_template_registry: &mut MaterialTemplateRegistry,
    material_texture_group_registry: &mut MaterialTextureGroupRegistry,
    declaration: &MaterialDeclaration,
) -> Result<MaterialID> {
    match &declaration.properties {
        MaterialProperties::Fixed(properties) => crate::setup::fixed::setup_fixed_material(
            texture_registry,
            sampler_registry,
            material_registry,
            material_template_registry,
            material_texture_group_registry,
            properties.clone(),
            Some(declaration.id),
        ),
        MaterialProperties::Physical(properties) => {
            crate::setup::physical::setup_physical_material(
                texture_registry,
                sampler_registry,
                material_registry,
                material_template_registry,
                material_texture_group_registry,
                properties.clone(),
                Some(declaration.id),
            )
        }
    }
}
