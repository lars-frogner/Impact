//! Management of materials for entities.

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

/// Checks if the entity-to-be with the given components has the components for
/// a material, and if so, adds the material specification to the material
/// library if not already present, adds the appropriate material property
/// texture set to the material library if not already present, registers the
/// material in the instance feature manager and adds the appropriate material
/// component to the entity.
pub fn setup_material_for_new_entity<MID: Eq + Hash>(
    graphics_device: &GraphicsDevice,
    texture_provider: &impl MaterialTextureProvider,
    material_library: &RwLock<MaterialLibrary>,
    instance_feature_manager: &RwLock<InstanceFeatureManager<MID>>,
    components: &mut ArchetypeComponentStorage,
    desynchronized: &mut bool,
) -> Result<()> {
    setup_fixed_color_material_for_new_entity(
        material_library,
        instance_feature_manager,
        components,
        desynchronized,
    );

    setup_fixed_texture_material_for_new_entity(
        graphics_device,
        texture_provider,
        material_library,
        components,
    )?;

    setup_physical_material_for_new_entity(
        graphics_device,
        texture_provider,
        material_library,
        instance_feature_manager,
        components,
        desynchronized,
    )?;

    Ok(())
}

/// Checks if the entity-to-be with the given components has the component
/// for this material, and if so, registers the material in the given
/// instance feature manager and adds the appropriate material component
/// to the entity.
fn setup_fixed_color_material_for_new_entity<MID: Eq + Hash>(
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

/// Checks if the entity-to-be with the given components has the component
/// for this material, and if so, adds the appropriate material property
/// texture set to the material library if not present and adds the
/// appropriate material component to the entity.
fn setup_fixed_texture_material_for_new_entity(
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

/// Checks if the entity-to-be with the given components has the components for
/// a physical material, and if so, adds the material specification to the
/// material library if not already present, adds the appropriate material
/// property texture set to the material library if not already present,
/// registers the material in the instance feature manager and adds the
/// appropriate material component to the entity.
fn setup_physical_material_for_new_entity<MID: Eq + Hash>(
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
