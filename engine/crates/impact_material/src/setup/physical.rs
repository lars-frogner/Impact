//! Materials using a microfacet reflection model.

use crate::{
    Material, MaterialBindGroupSlot, MaterialBindGroupTemplate, MaterialID, MaterialRegistry,
    MaterialTemplate, MaterialTemplateID, MaterialTemplateRegistry,
    MaterialTextureBindingLocations, MaterialTextureGroup, MaterialTextureGroupID,
    MaterialTextureGroupRegistry, RGBColor, values::MaterialPropertyValues,
};
use anyhow::{Result, anyhow, bail};
use approx::abs_diff_eq;
use bytemuck::{Pod, Zeroable};
use impact_math::{
    hash64,
    vector::{Vector2, Vector3P},
};
use impact_mesh::VertexAttributeSet;
use impact_texture::{SamplerRegistry, TextureID, TextureRegistry};
use roc_integration::roc;
use std::hash::Hash;

define_setup_type! {
    target = MaterialID;
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
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    pub struct UniformColor(pub RGBColor);
}

define_setup_type! {
    target = MaterialID;
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
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    pub struct TexturedColor(pub TextureID);
}

define_setup_type! {
    target = MaterialID;
    /// A uniform scalar specular reflectance at normal incidence (the
    /// proportion of incident light specularly reflected by the material when
    /// the light direction is perpendicular to the surface).
    #[roc(parents = "Setup")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    pub struct UniformSpecularReflectance(pub f32);
}

define_setup_type! {
    target = MaterialID;
    /// A textured scalar specular reflectance at normal incidence (the
    /// proportion of incident light specularly reflected by the material when
    /// the light direction is perpendicular to the surface).
    #[roc(parents = "Setup")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    pub struct TexturedSpecularReflectance {
        pub texture_id: TextureID,
        pub scale_factor: f64,
    }
}

define_setup_type! {
    target = MaterialID;
    /// A uniform surface roughness. The roughness ranges from zero (perfectly
    /// smooth) to one (completely diffuse).
    #[roc(parents = "Setup")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    pub struct UniformRoughness(pub f32);
}

define_setup_type! {
    target = MaterialID;
    /// A textured surface roughness. The roughness ranges from zero (perfectly
    /// smooth) to one (completely diffuse).
    #[roc(parents = "Setup")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    pub struct TexturedRoughness {
        pub texture_id: TextureID,
        pub scale_factor: f64,
    }
}

define_setup_type! {
    target = MaterialID;
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
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    pub struct UniformMetalness(pub f32);
}

define_setup_type! {
    target = MaterialID;
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
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    pub struct TexturedMetalness {
        pub texture_id: TextureID,
        pub scale_factor: f64,
    }
}

define_setup_type! {
    target = MaterialID;
    /// A uniform monochromatic emissive luminance.
    ///
    /// The RGB emissive luminance will be the material's base color multiplied by
    /// this scalar.
    #[roc(parents = "Setup")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    pub struct UniformEmissiveLuminance(pub f32);
}

define_setup_type! {
    target = MaterialID;
    /// A textured monochromatic emissive luminance.
    ///
    /// The RGB emissive luminance will be the material's base color multiplied by
    /// this scalar.
    #[roc(parents = "Setup")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    pub struct TexturedEmissiveLuminance {
        pub texture_id: TextureID,
        pub scale_factor: f64,
    }
}

define_setup_type! {
    target = MaterialID;
    /// A normal map describing surface details.
    #[roc(parents = "Setup")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    pub struct NormalMap(pub TextureID);
}

define_setup_type! {
    target = MaterialID;
    /// A parallax map describing surface details.
    #[roc(parents = "Setup")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    pub struct ParallaxMap {
        pub height_map_texture_id: TextureID,
        pub displacement_scale: f64,
        pub uv_per_distance: Vector2,
    }
}

/// A complete specification of the properties of a physical material.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct PhysicalMaterialProperties {
    pub color: Color,
    #[cfg_attr(feature = "serde", serde(default))]
    pub specular_reflectance: SpecularReflectance,
    #[cfg_attr(feature = "serde", serde(default))]
    pub roughness: Roughness,
    #[cfg_attr(feature = "serde", serde(default))]
    pub metalness: Metalness,
    #[cfg_attr(feature = "serde", serde(default))]
    pub emissive_luminance: EmissiveLuminance,
    #[cfg_attr(feature = "serde", serde(default))]
    pub bump_map: Option<BumpMap>,
}

/// A uniform or textured base color.
///
/// The base color affects the color and amount of light reflected and emitted
/// by the material in a way that depends on the material's conductive
/// properties. For dielectric materials, the base color is equivalent to the
/// material's the albedo (the proportion of incident light diffusely
/// reflected by the material). For metallic materials, the base color affects
/// the material's specular reflectance. For emissive materials, the base color
/// affects the material's emissive luminance.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub enum Color {
    Uniform(UniformColor),
    Textured(TexturedColor),
}

/// A uniform of textured scalar specular reflectance at normal incidence (the
/// proportion of incident light specularly reflected by the material when the
/// light direction is perpendicular to the surface).
///
/// If `Auto`, a uniform specular reflectance of 1.0 will be used if the
/// material is metallic, otherwise 0.0 will be used.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, Default)]
pub enum SpecularReflectance {
    #[default]
    Auto,
    Uniform(UniformSpecularReflectance),
    Textured(TexturedSpecularReflectance),
}

/// A uniform or textured surface roughness. The roughness ranges from zero
/// (perfectly smooth) to one (completely diffuse).
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub enum Roughness {
    Uniform(UniformRoughness),
    Textured(TexturedRoughness),
}

/// A uniform or textured metalness.
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub enum Metalness {
    Uniform(UniformMetalness),
    Textured(TexturedMetalness),
}

/// A uniform or textured monochromatic emissive luminance.
///
/// The RGB emissive luminance will be the material's base color multiplied by
/// this scalar.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub enum EmissiveLuminance {
    Uniform(UniformEmissiveLuminance),
    Textured(TexturedEmissiveLuminance),
}

/// A normal map or parallax map describing surface details.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub enum BumpMap {
    Normal(NormalMap),
    Parallax(ParallaxMap),
}

/// Binding locations for textures used in a physical material.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PhysicalMaterialTextureBindingLocations {
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
    pub const IRON: Self = Self(Vector3P::new(0.562, 0.565, 0.578));
    #[roc(expr = "(0.955, 0.638, 0.538)")]
    pub const COPPER: Self = Self(Vector3P::new(0.955, 0.638, 0.538));
    #[roc(expr = "(0.910, 0.778, 0.423)")]
    pub const BRASS: Self = Self(Vector3P::new(0.910, 0.778, 0.423));
    #[roc(expr = "(1.000, 0.782, 0.344)")]
    pub const GOLD: Self = Self(Vector3P::new(1.000, 0.782, 0.344));
    #[roc(expr = "(0.913, 0.922, 0.924)")]
    pub const ALUMINUM: Self = Self(Vector3P::new(0.913, 0.922, 0.924));
    #[roc(expr = "(0.972, 0.960, 0.915)")]
    pub const SILVER: Self = Self(Vector3P::new(0.972, 0.960, 0.915));
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
        displacement_scale: f64,
        uv_per_distance: Vector2,
    ) -> Self {
        Self {
            height_map_texture_id,
            displacement_scale,
            uv_per_distance,
        }
    }
}

impl PhysicalMaterialProperties {
    fn from_optional_parts(
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
    ) -> Result<Self> {
        let color = match (uniform_color, textured_color) {
            (Some(uniform_color), None) => Color::Uniform(*uniform_color),
            (None, Some(textured_color)) => Color::Textured(*textured_color),
            (None, None) => {
                bail!("Tried to create physical material with no color");
            }
            (Some(_), Some(_)) => {
                bail!("Tried to create physical material with both uniform and textured color");
            }
        };

        let specular_reflectance = match (
            uniform_specular_reflectance,
            textured_specular_reflectance,
        ) {
            (Some(uniform_specular_reflectance), None) => {
                SpecularReflectance::Uniform(*uniform_specular_reflectance)
            }
            (None, Some(textured_specular_reflectance)) => {
                SpecularReflectance::Textured(*textured_specular_reflectance)
            }
            (None, None) => SpecularReflectance::default(),
            (Some(_), Some(_)) => {
                bail!(
                    "Tried to create physical material with both uniform and textured specular reflectance"
                );
            }
        };

        let roughness = match (uniform_roughness, textured_roughness) {
            (Some(uniform_roughness), None) => Roughness::Uniform(*uniform_roughness),
            (None, Some(textured_roughness)) => Roughness::Textured(*textured_roughness),
            (None, None) => Roughness::default(),
            (Some(_), Some(_)) => {
                bail!("Tried to create physical material with both uniform and textured roughness");
            }
        };

        let metalness = match (uniform_metalness, textured_metalness) {
            (Some(uniform_metalness), None) => Metalness::Uniform(*uniform_metalness),
            (None, Some(textured_metalness)) => Metalness::Textured(*textured_metalness),
            (None, None) => Metalness::default(),
            (Some(_), Some(_)) => {
                bail!("Tried to create physical material with both uniform and textured metalness");
            }
        };

        let emissive_luminance = match (uniform_emissive_luminance, textured_emissive_luminance) {
            (Some(uniform_emissive_luminance), None) => {
                EmissiveLuminance::Uniform(*uniform_emissive_luminance)
            }
            (None, Some(textured_emissive_luminance)) => {
                EmissiveLuminance::Textured(*textured_emissive_luminance)
            }
            (None, None) => EmissiveLuminance::default(),
            (Some(_), Some(_)) => {
                bail!(
                    "Tried to create physical material with both uniform and textured emissive luminance"
                );
            }
        };

        let bump_map = match (normal_map, parallax_map) {
            (Some(normal_map), None) => Some(BumpMap::Normal(*normal_map)),
            (None, Some(parallax_map)) => Some(BumpMap::Parallax(*parallax_map)),
            (None, None) => None,
            (Some(_), Some(_)) => {
                bail!("Tried to create physical material with normal mapping and parallax mapping");
            }
        };

        Ok(Self {
            color,
            specular_reflectance,
            roughness,
            metalness,
            emissive_luminance,
            bump_map,
        })
    }
}

impl Default for Roughness {
    fn default() -> Self {
        Self::Uniform(UniformRoughness(1.0))
    }
}

impl Metalness {
    fn is_zero(&self) -> bool {
        matches!(
            self,
            Self::Uniform(UniformMetalness(metalness))
                if abs_diff_eq!(*metalness, 0.0)
        )
    }
}

impl Default for Metalness {
    fn default() -> Self {
        Self::Uniform(UniformMetalness(0.0))
    }
}

impl Default for EmissiveLuminance {
    fn default() -> Self {
        Self::Uniform(UniformEmissiveLuminance(0.0))
    }
}

pub fn setup_physical_material_from_optional_parts(
    texture_registry: &TextureRegistry,
    sampler_registry: &SamplerRegistry,
    material_registry: &mut MaterialRegistry,
    material_template_registry: &mut MaterialTemplateRegistry,
    material_texture_group_registry: &mut MaterialTextureGroupRegistry,
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
    material_id: Option<MaterialID>,
) -> Result<MaterialID> {
    let properties = PhysicalMaterialProperties::from_optional_parts(
        uniform_color,
        textured_color,
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
    )?;
    setup_physical_material(
        texture_registry,
        sampler_registry,
        material_registry,
        material_template_registry,
        material_texture_group_registry,
        properties,
        material_id,
    )
}

pub fn setup_physical_material(
    texture_registry: &TextureRegistry,
    sampler_registry: &SamplerRegistry,
    material_registry: &mut MaterialRegistry,
    material_template_registry: &mut MaterialTemplateRegistry,
    material_texture_group_registry: &mut MaterialTextureGroupRegistry,
    properties: PhysicalMaterialProperties,
    material_id: Option<MaterialID>,
) -> Result<MaterialID> {
    let material_id = material_id.unwrap_or_else(|| MaterialID(hash64!(format!("{properties:?}"))));

    if material_registry.contains(material_id) {
        return Ok(material_id);
    }

    let mut bind_group_slots = Vec::with_capacity(4);
    let mut texture_ids = Vec::with_capacity(4);

    let mut bindings = PhysicalMaterialTextureBindingLocations {
        color_texture_and_sampler_bindings: None,
        specular_reflectance_texture_and_sampler_bindings: None,
        roughness_texture_and_sampler_bindings: None,
        metalness_texture_and_sampler_bindings: None,
        emissive_luminance_texture_and_sampler_bindings: None,
        bump_mapping: None,
    };

    let uniform_color = match properties.color {
        Color::Uniform(uniform_color) => Some(uniform_color),
        Color::Textured(TexturedColor(texture_id)) => {
            bindings.color_texture_and_sampler_bindings = Some(
                MaterialBindGroupTemplate::get_texture_and_sampler_bindings(bind_group_slots.len()),
            );
            bind_group_slots.push(obtain_bind_group_slot(
                texture_registry,
                sampler_registry,
                texture_id,
                "color",
            )?);
            texture_ids.push(texture_id);
            None
        }
    };

    let specular_reflectance_value = match properties.specular_reflectance {
        SpecularReflectance::Auto => {
            if properties.metalness.is_zero() {
                0.0
            } else {
                1.0
            }
        }
        SpecularReflectance::Uniform(UniformSpecularReflectance(specular_reflectance)) => {
            specular_reflectance
        }
        SpecularReflectance::Textured(TexturedSpecularReflectance {
            texture_id,
            scale_factor,
        }) => {
            bindings.specular_reflectance_texture_and_sampler_bindings = Some(
                MaterialBindGroupTemplate::get_texture_and_sampler_bindings(bind_group_slots.len()),
            );
            bind_group_slots.push(obtain_bind_group_slot(
                texture_registry,
                sampler_registry,
                texture_id,
                "specular reflectance",
            )?);
            texture_ids.push(texture_id);

            scale_factor as f32
        }
    };

    let roughness_value = match properties.roughness {
        Roughness::Uniform(UniformRoughness(roughness)) => roughness,
        Roughness::Textured(TexturedRoughness {
            texture_id,
            scale_factor,
        }) => {
            bindings.roughness_texture_and_sampler_bindings = Some(
                MaterialBindGroupTemplate::get_texture_and_sampler_bindings(bind_group_slots.len()),
            );
            bind_group_slots.push(obtain_bind_group_slot(
                texture_registry,
                sampler_registry,
                texture_id,
                "roughness",
            )?);
            texture_ids.push(texture_id);

            scale_factor as f32
        }
    };

    let metalness_value = match properties.metalness {
        Metalness::Uniform(UniformMetalness(metalness)) => metalness,
        Metalness::Textured(TexturedMetalness {
            texture_id,
            scale_factor,
        }) => {
            bindings.metalness_texture_and_sampler_bindings = Some(
                MaterialBindGroupTemplate::get_texture_and_sampler_bindings(bind_group_slots.len()),
            );
            bind_group_slots.push(obtain_bind_group_slot(
                texture_registry,
                sampler_registry,
                texture_id,
                "metalness",
            )?);
            texture_ids.push(texture_id);

            scale_factor as f32
        }
    };

    let emissive_luminance_value = match properties.emissive_luminance {
        EmissiveLuminance::Uniform(UniformEmissiveLuminance(emissive_luminance)) => {
            emissive_luminance
        }
        EmissiveLuminance::Textured(TexturedEmissiveLuminance {
            texture_id,
            scale_factor,
        }) => {
            bindings.emissive_luminance_texture_and_sampler_bindings = Some(
                MaterialBindGroupTemplate::get_texture_and_sampler_bindings(bind_group_slots.len()),
            );
            bind_group_slots.push(obtain_bind_group_slot(
                texture_registry,
                sampler_registry,
                texture_id,
                "emissive luminance",
            )?);
            texture_ids.push(texture_id);

            scale_factor as f32
        }
    };

    let parallax_map = match properties.bump_map {
        None => None,
        Some(BumpMap::Normal(NormalMap(texture_id))) => {
            bindings.bump_mapping =
                Some(PhysicalMaterialBumpMappingTextureBindings::NormalMapping(
                    PhysicalMaterialNormalMappingTextureBindings {
                        normal_map_texture_and_sampler_bindings:
                            MaterialBindGroupTemplate::get_texture_and_sampler_bindings(
                                bind_group_slots.len(),
                            ),
                    },
                ));
            bind_group_slots.push(obtain_bind_group_slot(
                texture_registry,
                sampler_registry,
                texture_id,
                "normal map",
            )?);
            texture_ids.push(texture_id);
            None
        }
        Some(BumpMap::Parallax(parallax_map)) => {
            bindings.bump_mapping =
                Some(PhysicalMaterialBumpMappingTextureBindings::ParallaxMapping(
                    PhysicalMaterialParallaxMappingTextureBindings {
                        height_map_texture_and_sampler_bindings:
                            MaterialBindGroupTemplate::get_texture_and_sampler_bindings(
                                bind_group_slots.len(),
                            ),
                    },
                ));
            bind_group_slots.push(obtain_bind_group_slot(
                texture_registry,
                sampler_registry,
                parallax_map.height_map_texture_id,
                "height map",
            )?);
            texture_ids.push(parallax_map.height_map_texture_id);
            Some(parallax_map)
        }
    };

    let mut vertex_attribute_requirements = VertexAttributeSet::POSITION;

    if !texture_ids.is_empty() {
        vertex_attribute_requirements |= VertexAttributeSet::TEXTURE_COORDS;
    }
    if bindings.bump_mapping.is_some() {
        vertex_attribute_requirements |= VertexAttributeSet::TANGENT_SPACE_QUATERNION;
    } else {
        vertex_attribute_requirements |= VertexAttributeSet::NORMAL_VECTOR;
    }

    let property_values = MaterialPropertyValues::from_physical_material_properties(
        uniform_color.as_ref(),
        specular_reflectance_value,
        roughness_value,
        metalness_value,
        emissive_luminance_value,
        parallax_map.as_ref(),
    );

    let template = MaterialTemplate {
        vertex_attribute_requirements,
        bind_group_template: MaterialBindGroupTemplate {
            slots: bind_group_slots,
        },
        texture_binding_locations: MaterialTextureBindingLocations::Physical(bindings),
        property_flags: property_values.flags(),
        instance_feature_type_id: property_values.instance_feature_type_id(),
    };

    let template_id = MaterialTemplateID::for_template(&template);
    let texture_group_id = MaterialTextureGroupID::from_texture_ids(&texture_ids);

    let material = Material {
        template_id,
        texture_group_id,
        property_values,
    };

    material_registry.insert(material_id, material);

    material_template_registry.insert_with_if_absent(template_id, || template);

    if !texture_ids.is_empty() {
        material_texture_group_registry.insert_with_if_absent(texture_group_id, || {
            MaterialTextureGroup {
                template_id,
                texture_ids,
            }
        });
    }

    Ok(material_id)
}

fn obtain_bind_group_slot(
    texture_registry: &TextureRegistry,
    sampler_registry: &SamplerRegistry,
    texture_id: TextureID,
    texture_kind: &str,
) -> Result<MaterialBindGroupSlot> {
    let texture = texture_registry
        .get(texture_id)
        .ok_or_else(|| anyhow!("Missing {texture_kind} texture {texture_id} for material"))?;

    let sampler = sampler_registry
        .get(texture.sampler_id().ok_or_else(|| {
            anyhow!("Material {texture_kind} texture {texture_id} has no associated sampler")
        })?)
        .ok_or_else(|| {
            anyhow!("Missing sampler for material {texture_kind} texture {texture_id}")
        })?;

    Ok(MaterialBindGroupSlot {
        texture: texture.bind_group_layout_entry_props(),
        sampler: sampler.bind_group_layout_entry_props(),
    })
}
