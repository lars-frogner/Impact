//! Materials using a microfacet reflection model.

use super::super::features::create_physical_material_feature;
use crate::{
    assets::Assets,
    gpu::GraphicsDevice,
    material::{
        MaterialHandle, MaterialID, MaterialLibrary, MaterialPropertyTextureGroup,
        MaterialPropertyTextureGroupID, MaterialShaderInput, MaterialSpecification,
        components::{
            MaterialComp, NormalMapComp, ParallaxMapComp, TexturedColorComp,
            TexturedEmissiveLuminanceComp, TexturedMetalnessComp, TexturedRoughnessComp,
            TexturedSpecularReflectanceComp, UniformColorComp, UniformEmissiveLuminanceComp,
            UniformMetalnessComp, UniformRoughnessComp, UniformSpecularReflectanceComp,
        },
    },
    mesh::VertexAttributeSet,
    model::InstanceFeatureManager,
    scene::RenderResourcesDesynchronized,
};
use anyhow::{Result, bail};
use impact_ecs::{archetype::ArchetypeComponentStorage, setup};
use impact_math::hash64;
use std::{collections::hash_map::Entry, sync::RwLock};

/// Binding locations for textures used in a physical material.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PhysicalMaterialTextureBindings {
    pub color_texture_and_sampler_bindings: Option<(u32, u32)>,
    pub specular_reflectance_texture_and_sampler_bindings: Option<(u32, u32)>,
    pub roughness_texture_and_sampler_bindings: Option<(u32, u32)>,
    pub metalness_texture_and_sampler_bindings: Option<(u32, u32)>,
    pub emissive_luminance_texture_and_sampler_bindings: Option<(u32, u32)>,
    pub bump_mapping: Option<PhysicalMaterialBumpMappingTextureBindings>,
}

/// Binding locations for bump mapping-related textures used in a physical
/// material.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum PhysicalMaterialBumpMappingTextureBindings {
    NormalMapping(PhysicalMaterialNormalMappingTextureBindings),
    ParallaxMapping(PhysicalMaterialParallaxMappingTextureBindings),
}

/// Binding locations for normal mapping-related textures used in a physical
/// material.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PhysicalMaterialNormalMappingTextureBindings {
    pub normal_map_texture_and_sampler_bindings: (u32, u32),
}

/// Binding locations for parallax mapping-related textures used in a physical
/// material.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PhysicalMaterialParallaxMappingTextureBindings {
    pub height_map_texture_and_sampler_bindings: (u32, u32),
}

/// Checks if the entity-to-be with the given components has the components for
/// a physical material, and if so, adds the material specification to the
/// material library if not already present, adds the appropriate material
/// property texture set to the material library if not already present,
/// registers the material in the instance feature manager and adds the
/// appropriate material component to the entity.
pub fn setup_physical_material_for_new_entity(
    graphics_device: &GraphicsDevice,
    assets: &Assets,
    material_library: &RwLock<MaterialLibrary>,
    instance_feature_manager: &RwLock<InstanceFeatureManager>,
    components: &mut ArchetypeComponentStorage,
    desynchronized: &mut RenderResourcesDesynchronized,
) -> Result<()> {
    setup!(
        {
            desynchronized.set_yes();
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
            setup_physical_material(
                graphics_device,
                assets,
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
            )
        },
        ![MaterialComp, TexturedColorComp]
    )?;

    setup!(
        {
            desynchronized.set_yes();
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
            setup_physical_material(
                graphics_device,
                assets,
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
            )
        },
        ![MaterialComp, UniformColorComp]
    )
}

pub fn setup_physical_material(
    graphics_device: &GraphicsDevice,
    assets: &Assets,
    material_library: &mut MaterialLibrary,
    instance_feature_manager: &mut InstanceFeatureManager,
    uniform_color: Option<&UniformColorComp>,
    textured_color: Option<&TexturedColorComp>,
    uniform_specular_reflectance: Option<&UniformSpecularReflectanceComp>,
    textured_specular_reflectance: Option<&TexturedSpecularReflectanceComp>,
    uniform_roughness: Option<&UniformRoughnessComp>,
    textured_roughness: Option<&TexturedRoughnessComp>,
    uniform_metalness: Option<&UniformMetalnessComp>,
    textured_metalness: Option<&TexturedMetalnessComp>,
    uniform_emissive_luminance: Option<&UniformEmissiveLuminanceComp>,
    textured_emissive_luminance: Option<&TexturedEmissiveLuminanceComp>,
    normal_map: Option<&NormalMapComp>,
    parallax_map: Option<&ParallaxMapComp>,
) -> Result<MaterialComp> {
    let mut material_name_parts = Vec::with_capacity(8);

    let mut texture_ids = Vec::with_capacity(4);

    let mut bindings = PhysicalMaterialTextureBindings {
        color_texture_and_sampler_bindings: None,
        specular_reflectance_texture_and_sampler_bindings: None,
        roughness_texture_and_sampler_bindings: None,
        metalness_texture_and_sampler_bindings: None,
        emissive_luminance_texture_and_sampler_bindings: None,
        bump_mapping: None,
    };

    match (uniform_color, textured_color) {
        (Some(_), None) => {
            material_name_parts.push("UniformColor");
        }
        (None, Some(color)) => {
            material_name_parts.push("TexturedColor");
            bindings.color_texture_and_sampler_bindings = Some(
                MaterialPropertyTextureGroup::get_texture_and_sampler_bindings(texture_ids.len()),
            );
            texture_ids.push(color.0);
        }
        (None, None) => {
            bail!("Tried to create physical material with no color");
        }
        (Some(_), Some(_)) => {
            bail!("Tried to create physical material with both uniform and textured color");
        }
    }

    let specular_reflectance_value =
        match (uniform_specular_reflectance, textured_specular_reflectance) {
            (Some(specular_reflectance), None) => {
                material_name_parts.push("UniformSpecularReflectance");
                specular_reflectance.0
            }
            (None, Some(specular_reflectance)) => {
                material_name_parts.push("TexturedSpecularReflectance");

                bindings.specular_reflectance_texture_and_sampler_bindings = Some(
                    MaterialPropertyTextureGroup::get_texture_and_sampler_bindings(
                        texture_ids.len(),
                    ),
                );
                texture_ids.push(specular_reflectance.texture_id);

                specular_reflectance.scale_factor
            }
            _ => {
                if uniform_metalness.is_some() || textured_metalness.is_some() {
                    1.0
                } else {
                    0.0
                }
            }
        };

    let roughness_value = match (uniform_roughness, textured_roughness) {
        (Some(roughness), None) => {
            material_name_parts.push("UniformRoughness");
            roughness.0
        }
        (None, Some(roughness)) => {
            material_name_parts.push("TexturedRoughness");

            bindings.roughness_texture_and_sampler_bindings = Some(
                MaterialPropertyTextureGroup::get_texture_and_sampler_bindings(texture_ids.len()),
            );
            texture_ids.push(roughness.texture_id);

            roughness.scale_factor
        }
        _ => 1.0,
    };

    let metalness_value = match (uniform_metalness, textured_metalness) {
        (Some(metalness), None) => {
            material_name_parts.push("UniformMetalness");
            metalness.0
        }
        (None, Some(metalness)) => {
            material_name_parts.push("TexturedMetalness");

            bindings.metalness_texture_and_sampler_bindings = Some(
                MaterialPropertyTextureGroup::get_texture_and_sampler_bindings(texture_ids.len()),
            );
            texture_ids.push(metalness.texture_id);

            metalness.scale_factor
        }
        _ => 0.0,
    };

    let emissive_luminance_value = match (uniform_emissive_luminance, textured_emissive_luminance) {
        (Some(emissive_luminance), None) => {
            material_name_parts.push("UniformEmissiveLuminance");
            emissive_luminance.0
        }
        (None, Some(emissive_luminance)) => {
            material_name_parts.push("TexturedEmissiveLuminance");

            bindings.emissive_luminance_texture_and_sampler_bindings = Some(
                MaterialPropertyTextureGroup::get_texture_and_sampler_bindings(texture_ids.len()),
            );
            texture_ids.push(emissive_luminance.texture_id);

            emissive_luminance.scale_factor
        }
        _ => 0.0,
    };

    match (normal_map, parallax_map) {
        (Some(_), Some(_)) => {
            bail!("Tried to create physical material with normal mapping and parallax mapping");
        }
        (Some(normal_map), None) => {
            material_name_parts.push("NormalMapping");

            bindings.bump_mapping =
                Some(PhysicalMaterialBumpMappingTextureBindings::NormalMapping(
                    PhysicalMaterialNormalMappingTextureBindings {
                        normal_map_texture_and_sampler_bindings:
                            MaterialPropertyTextureGroup::get_texture_and_sampler_bindings(
                                texture_ids.len(),
                            ),
                    },
                ));

            texture_ids.push(normal_map.0);
        }
        (None, Some(parallax_map)) => {
            material_name_parts.push("ParallaxMapping");

            bindings.bump_mapping =
                Some(PhysicalMaterialBumpMappingTextureBindings::ParallaxMapping(
                    PhysicalMaterialParallaxMappingTextureBindings {
                        height_map_texture_and_sampler_bindings:
                            MaterialPropertyTextureGroup::get_texture_and_sampler_bindings(
                                texture_ids.len(),
                            ),
                    },
                ));

            texture_ids.push(parallax_map.height_map_texture_id);
        }
        (None, None) => {}
    }

    let mut vertex_attribute_requirements = VertexAttributeSet::POSITION;

    if !texture_ids.is_empty() {
        vertex_attribute_requirements |= VertexAttributeSet::TEXTURE_COORDS;
    }
    if bindings.bump_mapping.is_some() {
        vertex_attribute_requirements |= VertexAttributeSet::TANGENT_SPACE_QUATERNION;
    } else {
        vertex_attribute_requirements |= VertexAttributeSet::NORMAL_VECTOR;
    }

    let material_id = MaterialID(hash64!(format!(
        "{}PhysicalMaterial",
        material_name_parts.join(""),
    )));

    let (feature_type_id, feature_id, instance_feature_flags) = create_physical_material_feature(
        instance_feature_manager,
        uniform_color,
        specular_reflectance_value,
        roughness_value,
        metalness_value,
        emissive_luminance_value,
        parallax_map,
    );

    // Add material specification unless a specification for the same material
    // exists
    material_library
        .material_specification_entry(material_id)
        .or_insert_with(|| {
            MaterialSpecification::new(
                vertex_attribute_requirements,
                vec![feature_type_id],
                instance_feature_flags,
                None,
                MaterialShaderInput::Physical(bindings),
            )
        });

    let texture_group_id = if !texture_ids.is_empty() {
        let texture_group_id = MaterialPropertyTextureGroupID::from_texture_ids(&texture_ids);

        // Add a new texture set if none with the same textures already exist
        if let Entry::Vacant(entry) =
            material_library.material_property_texture_group_entry(texture_group_id)
        {
            entry.insert(MaterialPropertyTextureGroup::new(
                graphics_device,
                assets,
                texture_ids,
                texture_group_id.to_string(),
            )?);
        }

        Some(texture_group_id)
    } else {
        None
    };

    Ok(MaterialComp::new(MaterialHandle::new(
        material_id,
        Some(feature_id),
        texture_group_id,
    )))
}
