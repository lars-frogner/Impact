//! [`Component`](impact_ecs::component::Component)s related to materials.

use crate::{
    gpu::texture::TextureID,
    material::{MaterialHandle, RGBColor},
};
use bytemuck::{Pod, Zeroable};
use impact_ecs::{Component, SetupComponent};
use nalgebra::{Vector2, vector};
use roc_codegen::roc;

/// Setup [`SetupComponent`](impact_ecs::component::SetupComponent) for
/// initializing entities that have a fixed, uniform color that is independent
/// of lighting.
///
/// The purpose of this component is to aid in constructing a [`MaterialComp`]
/// for the entity. It is therefore not kept after entity creation.
#[roc]
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, SetupComponent)]
pub struct FixedColorComp(pub RGBColor);

/// [`SetupComponent`](impact_ecs::component::SetupComponent) for initializing
/// entities that have a fixed, textured color that is independent
/// of lighting.
///
/// The purpose of this component is to aid in constructing a [`MaterialComp`]
/// for the entity. It is therefore not kept after entity creation.
#[roc]
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, SetupComponent)]
pub struct FixedTextureComp(pub TextureID);

/// [`SetupComponent`](impact_ecs::component::SetupComponent) for initializing
/// entities that have a uniform base color.
///
/// The base color affects the color and amount of light reflected and emitted
/// by the material in a way that depends on the material's conductive
/// properties. For dielectric materials, the base color is equivalent to the
/// material's the albedo (the proportion of incident light diffusely
/// reflected by the material). For metallic materials, the base color affects
/// the material's specular reflectance. For emissive materials, the base color
/// affects the material's emissive luminance.
///
/// The purpose of this component is to aid in constructing a [`MaterialComp`]
/// for the entity. It is therefore not kept after entity creation.
#[roc]
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, SetupComponent)]
pub struct UniformColorComp(pub RGBColor);

/// [`SetupComponent`](impact_ecs::component::SetupComponent) for initializing
/// entities that have a textured base color.
///
/// The base color affects the color and amount of light reflected and emitted
/// by the material in a way that depends on the material's conductive
/// properties. For dielectric materials, the base color is equivalent to the
/// material's the albedo (the proportion of incident light diffusely
/// reflected by the material). For metallic materials, the base color affects
/// the material's specular reflectance. For emissive materials, the base color
/// affects the material's emissive luminance.
///
/// The purpose of this component is to aid in constructing a [`MaterialComp`]
/// for the entity. It is therefore not kept after entity creation.
#[roc]
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, SetupComponent)]
pub struct TexturedColorComp(pub TextureID);

/// [`SetupComponent`](impact_ecs::component::SetupComponent) for initializing
/// entities that have a uniform scalar specular reflectance at normal incidence
/// (the proportion of incident light specularly reflected by the material when
/// the light direction is perpendicular to the surface).
///
/// The purpose of this component is to aid in constructing a [`MaterialComp`]
/// for the entity. It is therefore not kept after entity creation.
#[roc]
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, SetupComponent)]
pub struct UniformSpecularReflectanceComp(pub f32);

/// [`SetupComponent`](impact_ecs::component::SetupComponent) for initializing
/// entities that have a textured scalar specular reflectance at normal
/// incidence (the proportion of incident light specularly reflected by the
/// material when the light direction is perpendicular to the surface).
///
/// The purpose of this component is to aid in constructing a [`MaterialComp`]
/// for the entity. It is therefore not kept after entity creation.
#[roc]
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, SetupComponent)]
pub struct TexturedSpecularReflectanceComp {
    pub texture_id: TextureID,
    pub scale_factor: f32,
}

/// [`SetupComponent`](impact_ecs::component::SetupComponent) for initializing
/// entities that have a uniform surface roughness. The roughness ranges from
/// zero (perfectly smooth) to one (completely diffuse).
///
/// The purpose of this component is to aid in constructing a [`MaterialComp`]
/// for the entity. It is therefore not kept after entity creation.
#[roc]
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, SetupComponent)]
pub struct UniformRoughnessComp(pub f32);

/// [`SetupComponent`](impact_ecs::component::SetupComponent) for initializing
/// entities that have a textured surface roughness. The roughness ranges from
/// zero (perfectly smooth) to one (completely diffuse).
///
/// The purpose of this component is to aid in constructing a [`MaterialComp`]
/// for the entity. It is therefore not kept after entity creation.
#[roc]
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, SetupComponent)]
pub struct TexturedRoughnessComp {
    pub texture_id: TextureID,
    pub scale_factor: f32,
}

/// [`SetupComponent`](impact_ecs::component::SetupComponent) for initializing
/// entities that have a uniform metalness.
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
///
/// The purpose of this component is to aid in constructing a [`MaterialComp`]
/// for the entity. It is therefore not kept after entity creation.
#[roc]
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, SetupComponent)]
pub struct UniformMetalnessComp(pub f32);

/// [`SetupComponent`](impact_ecs::component::SetupComponent) for initializing
/// entities that have a textured metalness.
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
///
/// The purpose of this component is to aid in constructing a [`MaterialComp`]
/// for the entity. It is therefore not kept after entity creation.
#[roc]
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, SetupComponent)]
pub struct TexturedMetalnessComp {
    pub texture_id: TextureID,
    pub scale_factor: f32,
}

/// [`SetupComponent`](impact_ecs::component::SetupComponent) for initializing
/// entities that have a uniform monochromatic emissive luminance.
///
/// The RGB emissive luminance will be the material's base color multiplied by
/// this scalar.
///
/// The purpose of this component is to aid in constructing a [`MaterialComp`]
/// for the entity. It is therefore not kept after entity creation.
#[roc]
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, SetupComponent)]
pub struct UniformEmissiveLuminanceComp(pub f32);

/// [`SetupComponent`](impact_ecs::component::SetupComponent) for initializing
/// entities that have a textured monochromatic emissive luminance.
///
/// The RGB emissive luminance will be the material's base color multiplied by
/// this scalar.
///
/// The purpose of this component is to aid in constructing a [`MaterialComp`]
/// for the entity. It is therefore not kept after entity creation.
#[roc]
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, SetupComponent)]
pub struct TexturedEmissiveLuminanceComp {
    pub texture_id: TextureID,
    pub scale_factor: f32,
}

/// [`SetupComponent`](impact_ecs::component::SetupComponent) for initializing
/// entities whose surface details are described by a normal map.
///
/// The purpose of this component is to aid in constructing a [`MaterialComp`]
/// for the entity. It is therefore not kept after entity creation.
#[roc]
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, SetupComponent)]
pub struct NormalMapComp(pub TextureID);

/// [`SetupComponent`](impact_ecs::component::SetupComponent) for initializing
/// entities whose surface details are described by a parallax map.
///
/// The purpose of this component is to aid in constructing a [`MaterialComp`]
/// for the entity. It is therefore not kept after entity creation.
#[roc]
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, SetupComponent)]
pub struct ParallaxMapComp {
    pub height_map_texture_id: TextureID,
    pub displacement_scale: f32,
    pub uv_per_distance: Vector2<f32>,
}

/// [`Component`](impact_ecs::component::Component) for entities that
/// have a material.
#[roc]
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct MaterialComp {
    material_handle: MaterialHandle,
}

impl UniformColorComp {
    pub const IRON: Self = Self(vector![0.562, 0.565, 0.578]);
    pub const COPPER: Self = Self(vector![0.955, 0.638, 0.538]);
    pub const BRASS: Self = Self(vector![0.910, 0.778, 0.423]);
    pub const GOLD: Self = Self(vector![1.000, 0.782, 0.344]);
    pub const ALUMINUM: Self = Self(vector![0.913, 0.922, 0.924]);
    pub const SILVER: Self = Self(vector![0.972, 0.960, 0.915]);
}

impl UniformSpecularReflectanceComp {
    pub const METAL: Self = Self(1.0);
    pub const WATER: Self = Self(0.02);
    pub const SKIN: Self = Self(0.028);

    pub const LIVING_TISSUE: (f32, f32) = (0.02, 0.04);
    pub const FABRIC: (f32, f32) = (0.04, 0.056);
    pub const STONE: (f32, f32) = (0.035, 0.056);
    pub const PLASTIC: (f32, f32) = (0.04, 0.05);
    pub const GLASS: (f32, f32) = (0.04, 0.05);

    pub fn in_range_of(range: (f32, f32), percentage: f32) -> Self {
        Self(range.0 + 0.01 * percentage * (range.1 - range.0))
    }
}

#[roc]
impl TexturedSpecularReflectanceComp {
    #[roc(body = "{ texture_id, scale_factor: 1.0 }")]
    pub fn unscaled(texture_id: TextureID) -> Self {
        Self {
            texture_id,
            scale_factor: 1.0,
        }
    }
}

#[roc]
impl TexturedRoughnessComp {
    #[roc(body = "{ texture_id, scale_factor: 1.0 }")]
    pub fn unscaled(texture_id: TextureID) -> Self {
        Self {
            texture_id,
            scale_factor: 1.0,
        }
    }
}

impl UniformMetalnessComp {
    pub const DIELECTRIC: Self = Self(0.0);
    pub const METAL: Self = Self(1.0);
}

#[roc]
impl TexturedMetalnessComp {
    #[roc(body = "{ texture_id, scale_factor: 1.0 }")]
    pub fn unscaled(texture_id: TextureID) -> Self {
        Self {
            texture_id,
            scale_factor: 1.0,
        }
    }
}

#[roc]
impl TexturedEmissiveLuminanceComp {
    #[roc(body = "{ texture_id, scale_factor: 1.0 }")]
    pub fn unscaled(texture_id: TextureID) -> Self {
        Self {
            texture_id,
            scale_factor: 1.0,
        }
    }
}

#[roc]
impl ParallaxMapComp {
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

#[roc]
impl MaterialComp {
    /// Creates a new component representing the material with the given handle.
    #[roc(body = "{ material_handle }")]
    pub fn new(material_handle: MaterialHandle) -> Self {
        assert!(!material_handle.is_not_applicable());
        Self { material_handle }
    }

    /// Returns a reference to the handle for the entity's material.
    pub fn material_handle(&self) -> &MaterialHandle {
        &self.material_handle
    }
}
