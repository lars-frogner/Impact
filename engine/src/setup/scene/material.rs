//! Setup of materials for new entities.

use anyhow::Result;
use impact_ecs::{archetype::ArchetypeComponentStorage, setup};
use impact_gpu::device::GraphicsDevice;
use impact_material::{
    MaterialHandle, MaterialLibrary, MaterialTextureProvider,
    setup::{
        self,
        fixed::{FixedColor, FixedTexture},
        physical::{
            NormalMap, ParallaxMap, TexturedColor, TexturedEmissiveLuminance, TexturedMetalness,
            TexturedRoughness, TexturedSpecularReflectance, UniformColor, UniformEmissiveLuminance,
            UniformMetalness, UniformRoughness, UniformSpecularReflectance,
        },
    },
};
use impact_model::InstanceFeatureManager;
use std::{hash::Hash, sync::RwLock};

/// Checks if the entites-to-be with the given components have the components
/// for a material, and if so, adds the material specifications to the material
/// library if not already present, adds the appropriate material property
/// texture sets to the material library if not already present, registers the
/// materials in the instance feature manager and adds the appropriate material
/// components to the entities.
pub fn setup_materials_for_new_entities<MID: Clone + Eq + Hash>(
    graphics_device: &GraphicsDevice,
    texture_provider: &impl MaterialTextureProvider,
    material_library: &RwLock<MaterialLibrary>,
    instance_feature_manager: &RwLock<InstanceFeatureManager<MID>>,
    components: &mut ArchetypeComponentStorage,
    desynchronized: &mut bool,
) -> Result<()> {
    setup_fixed_color_materials_for_new_entities(
        material_library,
        instance_feature_manager,
        components,
        desynchronized,
    );

    setup_fixed_texture_materials_for_new_entities(
        graphics_device,
        texture_provider,
        material_library,
        components,
    )?;

    setup_physical_materials_for_new_entities(
        graphics_device,
        texture_provider,
        material_library,
        instance_feature_manager,
        components,
        desynchronized,
    )?;

    Ok(())
}

fn setup_fixed_color_materials_for_new_entities<MID: Clone + Eq + Hash>(
    material_library: &RwLock<MaterialLibrary>,
    instance_feature_manager: &RwLock<InstanceFeatureManager<MID>>,
    components: &mut ArchetypeComponentStorage,
    desynchronized: &mut bool,
) {
    setup!(
        {
            let mut material_library = material_library.write().unwrap();
            let mut instance_feature_manager = instance_feature_manager.write().unwrap();
        },
        components,
        |fixed_color: &FixedColor| -> MaterialHandle {
            setup::fixed::setup_fixed_color_material(
                &mut material_library,
                &mut instance_feature_manager,
                fixed_color,
                desynchronized,
            )
        },
        ![MaterialHandle]
    );
}

fn setup_fixed_texture_materials_for_new_entities(
    graphics_device: &GraphicsDevice,
    texture_provider: &impl MaterialTextureProvider,
    material_library: &RwLock<MaterialLibrary>,
    components: &mut ArchetypeComponentStorage,
) -> Result<()> {
    setup!(
        {
            let mut material_library = material_library.write().unwrap();
        },
        components,
        |fixed_texture: &FixedTexture| -> Result<MaterialHandle> {
            setup::fixed::setup_fixed_texture_material(
                graphics_device,
                texture_provider,
                &mut material_library,
                fixed_texture,
            )
        },
        ![MaterialHandle]
    )
}

fn setup_physical_materials_for_new_entities<MID: Clone + Eq + Hash>(
    graphics_device: &GraphicsDevice,
    texture_provider: &impl MaterialTextureProvider,
    material_library: &RwLock<MaterialLibrary>,
    instance_feature_manager: &RwLock<InstanceFeatureManager<MID>>,
    components: &mut ArchetypeComponentStorage,
    desynchronized: &mut bool,
) -> Result<()> {
    setup!(
        {
            let mut material_library = material_library.write().unwrap();
            let mut instance_feature_manager = instance_feature_manager.write().unwrap();
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
         -> Result<MaterialHandle> {
            setup::physical::setup_physical_material(
                graphics_device,
                texture_provider,
                &mut material_library,
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
                desynchronized,
            )
        },
        ![MaterialHandle, TexturedColor]
    )?;

    setup!(
        {
            let mut material_library = material_library.write().unwrap();
            let mut instance_feature_manager = instance_feature_manager.write().unwrap();
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
         -> Result<MaterialHandle> {
            setup::physical::setup_physical_material(
                graphics_device,
                texture_provider,
                &mut material_library,
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
                desynchronized,
            )
        },
        ![MaterialHandle, UniformColor]
    )
}
