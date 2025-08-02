//! Setup of materials for new entities.

use crate::resource::ResourceManager;
use anyhow::Result;
use impact_ecs::{archetype::ArchetypeComponentStorage, setup};
use impact_material::{
    MaterialID,
    setup::{
        self,
        fixed::{Color, FixedColor, FixedMaterialProperties, FixedTexture},
        physical::{
            NormalMap, ParallaxMap, TexturedColor, TexturedEmissiveLuminance, TexturedMetalness,
            TexturedRoughness, TexturedSpecularReflectance, UniformColor, UniformEmissiveLuminance,
            UniformMetalness, UniformRoughness, UniformSpecularReflectance,
        },
    },
};
use impact_model::InstanceFeatureManager;
use parking_lot::RwLock;
use std::hash::Hash;

/// Checks if the entites-to-be with the given components have the components
/// for a material, and if so, adds the material specifications to the material
/// library if not already present, adds the appropriate material property
/// texture sets to the material library if not already present, registers the
/// materials in the instance feature manager and adds the appropriate material
/// components to the entities.
pub fn setup_materials_for_new_entities<MID: Clone + Eq + Hash>(
    resource_manager: &RwLock<ResourceManager>,
    instance_feature_manager: &RwLock<InstanceFeatureManager<MID>>,
    components: &mut ArchetypeComponentStorage,
    desynchronized: &mut bool,
) -> Result<()> {
    setup_fixed_materials_for_new_entities(
        resource_manager,
        instance_feature_manager,
        components,
        desynchronized,
    )?;

    setup_physical_materials_for_new_entities(
        resource_manager,
        instance_feature_manager,
        components,
        desynchronized,
    )?;

    Ok(())
}

fn setup_fixed_materials_for_new_entities<MID: Clone + Eq + Hash>(
    resource_manager: &RwLock<ResourceManager>,
    instance_feature_manager: &RwLock<InstanceFeatureManager<MID>>,
    components: &mut ArchetypeComponentStorage,
    desynchronized: &mut bool,
) -> Result<()> {
    setup!(
        {
            let mut resource_manager = resource_manager.write();
            let mut instance_feature_manager = instance_feature_manager.write();
        },
        components,
        |fixed_color: &FixedColor| -> Result<MaterialID> {
            let resource_manager = &mut *resource_manager;
            setup::fixed::setup_fixed_material(
                &resource_manager.textures,
                &resource_manager.samplers,
                &mut resource_manager.materials,
                &mut resource_manager.material_templates,
                &mut resource_manager.material_texture_groups,
                &mut instance_feature_manager,
                FixedMaterialProperties {
                    color: Color::Uniform(*fixed_color),
                },
                None,
                desynchronized,
            )
        },
        ![MaterialID]
    )?;
    setup!(
        {
            let mut resource_manager = resource_manager.write();
            let mut instance_feature_manager = instance_feature_manager.write();
        },
        components,
        |fixed_texture: &FixedTexture| -> Result<MaterialID> {
            let resource_manager = &mut *resource_manager;
            setup::fixed::setup_fixed_material(
                &resource_manager.textures,
                &resource_manager.samplers,
                &mut resource_manager.materials,
                &mut resource_manager.material_templates,
                &mut resource_manager.material_texture_groups,
                &mut instance_feature_manager,
                FixedMaterialProperties {
                    color: Color::Textured(*fixed_texture),
                },
                None,
                desynchronized,
            )
        },
        ![MaterialID]
    )
}

fn setup_physical_materials_for_new_entities<MID: Clone + Eq + Hash>(
    resource_manager: &RwLock<ResourceManager>,
    instance_feature_manager: &RwLock<InstanceFeatureManager<MID>>,
    components: &mut ArchetypeComponentStorage,
    desynchronized: &mut bool,
) -> Result<()> {
    setup!(
        {
            let mut resource_manager = resource_manager.write();
            let mut instance_feature_manager = instance_feature_manager.write();
        },
        components,
        |uniform_color: &UniformColor,
         uniform_specular_reflectance: Option<&UniformSpecularReflectance>,
         textured_specular_reflectance: Option<&TexturedSpecularReflectance>,
         uniform_roughness: Option<&UniformRoughness>,
         textured_roughness: Option<&TexturedRoughness>,
         uniform_metalness: Option<&UniformMetalness>,
         textured_metalness: Option<&TexturedMetalness>,
         uniform_emissive_luminance: Option<&UniformEmissiveLuminance>,
         textured_emissive_luminance: Option<&TexturedEmissiveLuminance>,
         normal_map: Option<&NormalMap>,
         parallax_map: Option<&ParallaxMap>|
         -> Result<MaterialID> {
            let resource_manager = &mut *resource_manager;
            setup::physical::setup_physical_material_from_optional_parts(
                &resource_manager.textures,
                &resource_manager.samplers,
                &mut resource_manager.materials,
                &mut resource_manager.material_templates,
                &mut resource_manager.material_texture_groups,
                &mut instance_feature_manager,
                Some(uniform_color),
                None,
                uniform_specular_reflectance,
                textured_specular_reflectance,
                uniform_roughness,
                textured_roughness,
                uniform_metalness,
                textured_metalness,
                uniform_emissive_luminance,
                textured_emissive_luminance,
                normal_map,
                parallax_map,
                None,
                desynchronized,
            )
        },
        ![MaterialID, TexturedColor]
    )?;

    setup!(
        {
            let mut resource_manager = resource_manager.write();
            let mut instance_feature_manager = instance_feature_manager.write();
        },
        components,
        |textured_color: &TexturedColor,
         uniform_specular_reflectance: Option<&UniformSpecularReflectance>,
         textured_specular_reflectance: Option<&TexturedSpecularReflectance>,
         uniform_roughness: Option<&UniformRoughness>,
         textured_roughness: Option<&TexturedRoughness>,
         uniform_metalness: Option<&UniformMetalness>,
         textured_metalness: Option<&TexturedMetalness>,
         uniform_emissive_luminance: Option<&UniformEmissiveLuminance>,
         textured_emissive_luminance: Option<&TexturedEmissiveLuminance>,
         normal_map: Option<&NormalMap>,
         parallax_map: Option<&ParallaxMap>|
         -> Result<MaterialID> {
            let resource_manager = &mut *resource_manager;
            setup::physical::setup_physical_material_from_optional_parts(
                &resource_manager.textures,
                &resource_manager.samplers,
                &mut resource_manager.materials,
                &mut resource_manager.material_templates,
                &mut resource_manager.material_texture_groups,
                &mut instance_feature_manager,
                None,
                Some(textured_color),
                uniform_specular_reflectance,
                textured_specular_reflectance,
                uniform_roughness,
                textured_roughness,
                uniform_metalness,
                textured_metalness,
                uniform_emissive_luminance,
                textured_emissive_luminance,
                normal_map,
                parallax_map,
                None,
                desynchronized,
            )
        },
        ![MaterialID, UniformColor]
    )
}
