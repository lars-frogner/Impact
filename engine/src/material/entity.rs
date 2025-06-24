//! Management of materials for entities.

use anyhow::Result;
use impact_ecs::{archetype::ArchetypeComponentStorage, setup};
use impact_gpu::device::GraphicsDevice;
use impact_material::{
    MaterialLibrary, MaterialTextureProvider,
    components::{
        FixedColorComp, FixedTextureComp, MaterialComp, NormalMapComp, ParallaxMapComp,
        TexturedColorComp, TexturedEmissiveLuminanceComp, TexturedMetalnessComp,
        TexturedRoughnessComp, TexturedSpecularReflectanceComp, UniformColorComp,
        UniformEmissiveLuminanceComp, UniformMetalnessComp, UniformRoughnessComp,
        UniformSpecularReflectanceComp,
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
        |fixed_color: &FixedColorComp| -> MaterialComp {
            impact_material::entity::fixed::setup_fixed_color_material(
                &mut material_library,
                &mut instance_feature_manager,
                fixed_color,
                desynchronized,
            )
        },
        ![MaterialComp]
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
        |fixed_texture: &FixedTextureComp| -> Result<MaterialComp> {
            impact_material::entity::fixed::setup_fixed_texture_material(
                graphics_device,
                texture_provider,
                &mut material_library,
                fixed_texture,
            )
        },
        ![MaterialComp]
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
        |uniform_color: &UniformColorComp,
         uniform_specular_reflectance: Option<&UniformSpecularReflectanceComp>,
         textured_specular_reflectance: Option<&TexturedSpecularReflectanceComp>,
         uniform_roughness: Option<&UniformRoughnessComp>,
         textured_roughness: Option<&TexturedRoughnessComp>,
         uniform_metalness: Option<&UniformMetalnessComp>,
         textured_metalness: Option<&TexturedMetalnessComp>,
         uniform_emissive_luminance: Option<&UniformEmissiveLuminanceComp>,
         textured_emissive_luminance: Option<&TexturedEmissiveLuminanceComp>,
         normal_map: Option<&NormalMapComp>,
         parallax_map: Option<&ParallaxMapComp>|
         -> Result<MaterialComp> {
            impact_material::entity::physical::setup_physical_material(
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
        ![MaterialComp, TexturedColorComp]
    )?;

    setup!(
        {
            let mut material_library = material_library.write().unwrap();
            let mut instance_feature_manager = instance_feature_manager.write().unwrap();
        },
        components,
        |textured_color: &TexturedColorComp,
         uniform_specular_reflectance: Option<&UniformSpecularReflectanceComp>,
         textured_specular_reflectance: Option<&TexturedSpecularReflectanceComp>,
         uniform_roughness: Option<&UniformRoughnessComp>,
         textured_roughness: Option<&TexturedRoughnessComp>,
         uniform_metalness: Option<&UniformMetalnessComp>,
         textured_metalness: Option<&TexturedMetalnessComp>,
         uniform_emissive_luminance: Option<&UniformEmissiveLuminanceComp>,
         textured_emissive_luminance: Option<&TexturedEmissiveLuminanceComp>,
         normal_map: Option<&NormalMapComp>,
         parallax_map: Option<&ParallaxMapComp>|
         -> Result<MaterialComp> {
            impact_material::entity::physical::setup_physical_material(
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
        ![MaterialComp, UniformColorComp]
    )
}
