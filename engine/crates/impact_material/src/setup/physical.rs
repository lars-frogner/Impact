//! Materials using a microfacet reflection model.

use crate::{
    MaterialHandle, MaterialID, MaterialLibrary, MaterialPropertyTextureGroup,
    MaterialPropertyTextureGroupID, MaterialShaderInput, MaterialSpecification,
    MaterialTextureProvider, RGBColor, features::create_physical_material_feature,
};
use anyhow::{Result, bail};
use bytemuck::{Pod, Zeroable};
use impact_gpu::{device::GraphicsDevice, texture::TextureID};
use impact_math::hash64;
use impact_mesh::VertexAttributeSet;
use impact_model::InstanceFeatureManager;
use nalgebra::{Vector2, vector};
use roc_integration::roc;
use std::{collections::hash_map::Entry, hash::Hash};

define_setup_type! {
    target = MaterialHandle;
    /// A uniform base color.
    ///
    /// The base color affects the color and amount of light reflected and emitted
    /// by the material in a way that depends on the material's conductive
    /// properties. For dielectric materials, the base color is equivalent to the
    /// material's the albedo (the proportion of incident light diffusely
    /// reflected by the material). For metallic materials, the base color affects
    /// the material's specular reflectance. For emissive materials, the base color
    /// affects the material's emissive luminance.
    #[roc(parents = "Setup")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct UniformColor(pub RGBColor);
}

define_setup_type! {
    target = MaterialHandle;
    /// A textured base color.
    ///
    /// The base color affects the color and amount of light reflected and emitted
    /// by the material in a way that depends on the material's conductive
    /// properties. For dielectric materials, the base color is equivalent to the
    /// material's the albedo (the proportion of incident light diffusely
    /// reflected by the material). For metallic materials, the base color affects
    /// the material's specular reflectance. For emissive materials, the base color
    /// affects the material's emissive luminance.
    #[roc(parents = "Setup")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct TexturedColor(pub TextureID);
}

define_setup_type! {
    target = MaterialHandle;
    /// A uniform scalar specular reflectance at normal incidence (the
    /// proportion of incident light specularly reflected by the material when
    /// the light direction is perpendicular to the surface).
    #[roc(parents = "Setup")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct UniformSpecularReflectance(pub f32);
}

define_setup_type! {
    target = MaterialHandle;
    /// A textured scalar specular reflectance at normal incidence (the
    /// proportion of incident light specularly reflected by the material when
    /// the light direction is perpendicular to the surface).
    #[roc(parents = "Setup")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct TexturedSpecularReflectance {
        pub texture_id: TextureID,
        pub scale_factor: f32,
    }
}

define_setup_type! {
    target = MaterialHandle;
    /// A uniform surface roughness. The roughness ranges from zero (perfectly
    /// smooth) to one (completely diffuse).
    #[roc(parents = "Setup")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct UniformRoughness(pub f32);
}

define_setup_type! {
    target = MaterialHandle;
    /// A textured surface roughness. The roughness ranges from zero (perfectly
    /// smooth) to one (completely diffuse).
    #[roc(parents = "Setup")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct TexturedRoughness {
        pub texture_id: TextureID,
        pub scale_factor: f32,
    }
}

define_setup_type! {
    target = MaterialHandle;
    /// A uniform metalness.
    ///
    /// The metalness describes the conductive properties of the material. A value
    /// of zero means that the material is dielectric, while a value of one means
    /// that the material is a metal.
    ///
    /// A dielectric material will have an RGB diffuse reflectance corresponding
    /// to the material's base color, and a specular reflectance that is the
    /// same for each color component (and equal to the scalar specular
    /// reflectance).
    ///
    /// A metallic material will have zero diffuse reflectance, and an RGB
    /// specular reflectance corresponding to the material's base color
    /// multiplied by the scalar specular reflectance.
    #[roc(parents = "Setup")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct UniformMetalness(pub f32);
}

define_setup_type! {
    target = MaterialHandle;
    /// A textured metalness.
    ///
    /// The metalness describes the conductive properties of the material. A value
    /// of zero means that the material is dielectric, while a value of one means
    /// that the material is a metal.
    ///
    /// A dielectric material will have an RGB diffuse reflectance corresponding
    /// to the material's base color, and a specular reflectance that is the
    /// same for each color component (and equal to the scalar specular
    /// reflectance).
    ///
    /// A metallic material will have zero diffuse reflectance, and an RGB
    /// specular reflectance corresponding to the material's base color
    /// multiplied by the scalar specular reflectance.
    #[roc(parents = "Setup")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct TexturedMetalness {
        pub texture_id: TextureID,
        pub scale_factor: f32,
    }
}

define_setup_type! {
    target = MaterialHandle;
    /// A uniform monochromatic emissive luminance.
    ///
    /// The RGB emissive luminance will be the material's base color multiplied by
    /// this scalar.
    #[roc(parents = "Setup")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct UniformEmissiveLuminance(pub f32);
}

define_setup_type! {
    target = MaterialHandle;
    /// A textured monochromatic emissive luminance.
    ///
    /// The RGB emissive luminance will be the material's base color multiplied by
    /// this scalar.
    #[roc(parents = "Setup")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct TexturedEmissiveLuminance {
        pub texture_id: TextureID,
        pub scale_factor: f32,
    }
}

define_setup_type! {
    target = MaterialHandle;
    /// A normal map describing surface details.
    #[roc(parents = "Setup")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct NormalMap(pub TextureID);
}

define_setup_type! {
    target = MaterialHandle;
    /// A parallax map describing surface details.
    #[roc(parents = "Setup")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct ParallaxMap {
        pub height_map_texture_id: TextureID,
        pub displacement_scale: f32,
        pub uv_per_distance: Vector2<f32>,
    }
}

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

#[roc]
impl UniformColor {
    #[roc(expr = "(0.562, 0.565, 0.578)")]
    pub const IRON: Self = Self(vector![0.562, 0.565, 0.578]);
    #[roc(expr = "(0.955, 0.638, 0.538)")]
    pub const COPPER: Self = Self(vector![0.955, 0.638, 0.538]);
    #[roc(expr = "(0.910, 0.778, 0.423)")]
    pub const BRASS: Self = Self(vector![0.910, 0.778, 0.423]);
    #[roc(expr = "(1.000, 0.782, 0.344)")]
    pub const GOLD: Self = Self(vector![1.000, 0.782, 0.344]);
    #[roc(expr = "(0.913, 0.922, 0.924)")]
    pub const ALUMINUM: Self = Self(vector![0.913, 0.922, 0.924]);
    #[roc(expr = "(0.972, 0.960, 0.915)")]
    pub const SILVER: Self = Self(vector![0.972, 0.960, 0.915]);
}

#[roc]
impl UniformSpecularReflectance {
    #[roc(expr = "1.0")]
    pub const METAL: Self = Self(1.0);
    #[roc(expr = "0.02")]
    pub const WATER: Self = Self(0.02);
    #[roc(expr = "0.028")]
    pub const SKIN: Self = Self(0.028);

    #[roc(expr = "(0.02, 0.04)")]
    pub const LIVING_TISSUE: (f32, f32) = (0.02, 0.04);
    #[roc(expr = "(0.04, 0.056)")]
    pub const FABRIC: (f32, f32) = (0.04, 0.056);
    #[roc(expr = "(0.035, 0.056)")]
    pub const STONE: (f32, f32) = (0.035, 0.056);
    #[roc(expr = "(0.04, 0.05)")]
    pub const PLASTIC: (f32, f32) = (0.04, 0.05);
    #[roc(expr = "(0.04, 0.05)")]
    pub const GLASS: (f32, f32) = (0.04, 0.05);

    #[roc(body = "range.0 + 0.01 * percentage * (range.1 - range.0)")]
    pub fn in_range_of(range: (f32, f32), percentage: f32) -> Self {
        Self(range.0 + 0.01 * percentage * (range.1 - range.0))
    }
}

#[roc]
impl TexturedSpecularReflectance {
    #[roc(body = "{ texture_id, scale_factor: 1.0 }")]
    pub fn unscaled(texture_id: TextureID) -> Self {
        Self {
            texture_id,
            scale_factor: 1.0,
        }
    }
}

#[roc]
impl TexturedRoughness {
    #[roc(body = "{ texture_id, scale_factor: 1.0 }")]
    pub fn unscaled(texture_id: TextureID) -> Self {
        Self {
            texture_id,
            scale_factor: 1.0,
        }
    }
}

#[roc]
impl UniformMetalness {
    #[roc(expr = "0.0")]
    pub const DIELECTRIC: Self = Self(0.0);
    #[roc(expr = "1.0")]
    pub const METAL: Self = Self(1.0);
}

#[roc]
impl TexturedMetalness {
    #[roc(body = "{ texture_id, scale_factor: 1.0 }")]
    pub fn unscaled(texture_id: TextureID) -> Self {
        Self {
            texture_id,
            scale_factor: 1.0,
        }
    }
}

#[roc]
impl TexturedEmissiveLuminance {
    #[roc(body = "{ texture_id, scale_factor: 1.0 }")]
    pub fn unscaled(texture_id: TextureID) -> Self {
        Self {
            texture_id,
            scale_factor: 1.0,
        }
    }
}

#[roc]
impl ParallaxMap {
    #[roc(body = "{ height_map_texture_id, displacement_scale, uv_per_distance }")]
    pub fn new(
        height_map_texture_id: TextureID,
        displacement_scale: f32,
        uv_per_distance: Vector2<f32>,
    ) -> Self {
        Self {
            height_map_texture_id,
            displacement_scale,
            uv_per_distance,
        }
    }
}

pub fn setup_physical_material<MID: Clone + Eq + Hash>(
    graphics_device: &GraphicsDevice,
    texture_provider: &impl MaterialTextureProvider,
    material_library: &mut MaterialLibrary,
    instance_feature_manager: &mut InstanceFeatureManager<MID>,
    uniform_color: Option<&UniformColor>,
    textured_color: Option<&TexturedColor>,
    uniform_specular_reflectance: Option<&UniformSpecularReflectance>,
    textured_specular_reflectance: Option<&TexturedSpecularReflectance>,
    uniform_roughness: Option<&UniformRoughness>,
    textured_roughness: Option<&TexturedRoughness>,
    uniform_metalness: Option<&UniformMetalness>,
    textured_metalness: Option<&TexturedMetalness>,
    uniform_emissive_luminance: Option<&UniformEmissiveLuminance>,
    textured_emissive_luminance: Option<&TexturedEmissiveLuminance>,
    normal_map: Option<&NormalMap>,
    parallax_map: Option<&ParallaxMap>,
    desynchronized: &mut bool,
) -> Result<MaterialHandle> {
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
                texture_provider,
                texture_ids,
                texture_group_id.to_string(),
            )?);
        }

        Some(texture_group_id)
    } else {
        None
    };

    *desynchronized = true;

    Ok(MaterialHandle::new(
        material_id,
        Some(feature_id),
        texture_group_id,
    ))
}
