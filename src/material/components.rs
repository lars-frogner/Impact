//! [`Component`](impact_ecs::component::Component)s related to materials.

use crate::{
    component::ComponentRegistry,
    gpu::{rendering::fre, texture::TextureID},
    material::{MaterialHandle, RGBColor},
};
use anyhow::Result;
use bytemuck::{Pod, Zeroable};
use impact_ecs::Component;
use nalgebra::{vector, Vector2};

/// Setup [`Component`](impact_ecs::component::Component) for initializing
/// entities that have a fixed, uniform color that is independent of lighting.
///
/// The purpose of this component is to aid in constructing a [`MaterialComp`]
/// for the entity. It is therefore not kept after entity creation.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct FixedColorComp(pub RGBColor);

/// Setup [`Component`](impact_ecs::component::Component) for initializing
/// entities that have a fixed, textured color that is independent of lighting.
///
/// The purpose of this component is to aid in constructing a [`MaterialComp`]
/// for the entity. It is therefore not kept after entity creation.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct FixedTextureComp(pub TextureID);

/// Setup [`Component`](impact_ecs::component::Component) for initializing
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
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct UniformColorComp(pub RGBColor);

/// Setup [`Component`](impact_ecs::component::Component) for initializing
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
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct TexturedColorComp(pub TextureID);

/// Setup [`Component`](impact_ecs::component::Component) for initializing
/// entities that have a uniform scalar specular reflectance at normal incidence
/// (the proportion of incident light specularly reflected by the material when
/// the light direction is perpendicular to the surface).
///
/// The purpose of this component is to aid in constructing a [`MaterialComp`]
/// for the entity. It is therefore not kept after entity creation.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct UniformSpecularReflectanceComp(pub fre);

/// Setup [`Component`](impact_ecs::component::Component) for initializing
/// entities that have a textured scalar specular reflectance at normal
/// incidence (the proportion of incident light specularly reflected by the
/// material when the light direction is perpendicular to the surface).
///
/// The purpose of this component is to aid in constructing a [`MaterialComp`]
/// for the entity. It is therefore not kept after entity creation.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct TexturedSpecularReflectanceComp {
    pub texture_id: TextureID,
    pub scale_factor: fre,
}

/// Setup [`Component`](impact_ecs::component::Component) for initializing
/// entities that have a uniform surface roughness. The roughness ranges from
/// zero (perfectly smooth) to one (completely diffuse).
///
/// The purpose of this component is to aid in constructing a [`MaterialComp`]
/// for the entity. It is therefore not kept after entity creation.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct UniformRoughnessComp(pub fre);

/// Setup [`Component`](impact_ecs::component::Component) for initializing
/// entities that have a textured surface roughness. The roughness ranges from
/// zero (perfectly smooth) to one (completely diffuse).
///
/// The purpose of this component is to aid in constructing a [`MaterialComp`]
/// for the entity. It is therefore not kept after entity creation.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct TexturedRoughnessComp {
    pub texture_id: TextureID,
    pub scale_factor: fre,
}

/// Setup [`Component`](impact_ecs::component::Component) for initializing
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
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct UniformMetalnessComp(pub fre);

/// Setup [`Component`](impact_ecs::component::Component) for initializing
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
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct TexturedMetalnessComp {
    pub texture_id: TextureID,
    pub scale_factor: fre,
}

/// Setup [`Component`](impact_ecs::component::Component) for initializing
/// entities that have a uniform monochromatic emissive luminance.
///
/// The RGB emissive luminance will be the material's base color multiplied by
/// this scalar.
///
/// The purpose of this component is to aid in constructing a [`MaterialComp`]
/// for the entity. It is therefore not kept after entity creation.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct UniformEmissiveLuminanceComp(pub fre);

/// Setup [`Component`](impact_ecs::component::Component) for initializing
/// entities that have a textured monochromatic emissive luminance.
///
/// The RGB emissive luminance will be the material's base color multiplied by
/// this scalar.
///
/// The purpose of this component is to aid in constructing a [`MaterialComp`]
/// for the entity. It is therefore not kept after entity creation.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct TexturedEmissiveLuminanceComp {
    pub texture_id: TextureID,
    pub scale_factor: fre,
}

/// Setup [`Component`](impact_ecs::component::Component) for initializing
/// entities whose surface details are described by a normal map.
///
/// The purpose of this component is to aid in constructing a [`MaterialComp`]
/// for the entity. It is therefore not kept after entity creation.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct NormalMapComp(pub TextureID);

/// Setup [`Component`](impact_ecs::component::Component) for initializing
/// entities whose surface details are described by a parallax map.
///
/// The purpose of this component is to aid in constructing a [`MaterialComp`]
/// for the entity. It is therefore not kept after entity creation.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct ParallaxMapComp {
    pub height_map_texture_id: TextureID,
    pub displacement_scale: fre,
    pub uv_per_distance: Vector2<fre>,
}

/// [`Component`](impact_ecs::component::Component) for entities that
/// have a material.
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

    pub const LIVING_TISSUE: (fre, fre) = (0.02, 0.04);
    pub const FABRIC: (fre, fre) = (0.04, 0.056);
    pub const STONE: (fre, fre) = (0.035, 0.056);
    pub const PLASTIC: (fre, fre) = (0.04, 0.05);
    pub const GLASS: (fre, fre) = (0.04, 0.05);

    pub fn in_range_of(range: (fre, fre), percentage: fre) -> Self {
        Self(range.0 + 0.01 * percentage * (range.1 - range.0))
    }
}

impl TexturedSpecularReflectanceComp {
    pub fn unscaled(texture_id: TextureID) -> Self {
        Self {
            texture_id,
            scale_factor: 1.0,
        }
    }
}

impl TexturedRoughnessComp {
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

impl TexturedMetalnessComp {
    pub fn unscaled(texture_id: TextureID) -> Self {
        Self {
            texture_id,
            scale_factor: 1.0,
        }
    }
}

impl TexturedEmissiveLuminanceComp {
    pub fn unscaled(texture_id: TextureID) -> Self {
        Self {
            texture_id,
            scale_factor: 1.0,
        }
    }
}

impl ParallaxMapComp {
    pub fn new(
        height_map_texture_id: TextureID,
        displacement_scale: fre,
        uv_per_distance: Vector2<fre>,
    ) -> Self {
        Self {
            height_map_texture_id,
            displacement_scale,
            uv_per_distance,
        }
    }
}

impl MaterialComp {
    /// Creates a new component representing the material with the given handle.
    pub fn new(material_handle: MaterialHandle) -> Self {
        assert!(!material_handle.is_not_applicable());
        Self { material_handle }
    }

    /// Returns a reference to the handle for the entity's material.
    pub fn material_handle(&self) -> &MaterialHandle {
        &self.material_handle
    }
}

/// Registers all material [`Component`](impact_ecs::component::Component)s.
pub fn register_material_components(registry: &mut ComponentRegistry) -> Result<()> {
    register_setup_component!(registry, FixedColorComp)?;
    register_setup_component!(registry, FixedTextureComp)?;
    register_setup_component!(registry, UniformColorComp)?;
    register_setup_component!(registry, TexturedColorComp)?;
    register_setup_component!(registry, UniformSpecularReflectanceComp)?;
    register_setup_component!(registry, TexturedSpecularReflectanceComp)?;
    register_setup_component!(registry, UniformRoughnessComp)?;
    register_setup_component!(registry, TexturedRoughnessComp)?;
    register_setup_component!(registry, UniformMetalnessComp)?;
    register_setup_component!(registry, TexturedMetalnessComp)?;
    register_setup_component!(registry, UniformEmissiveLuminanceComp)?;
    register_setup_component!(registry, TexturedEmissiveLuminanceComp)?;
    register_setup_component!(registry, NormalMapComp)?;
    register_setup_component!(registry, ParallaxMapComp)?;
    register_component!(registry, MaterialComp)
}
