//! [`Component`](impact_ecs::component::Component)s related to materials.

use crate::{
    assets::TextureID,
    components::ComponentRegistry,
    gpu::rendering::fre,
    material::{MaterialHandle, RGBColor},
};
use anyhow::Result;
use bytemuck::{Pod, Zeroable};
use impact_ecs::Component;
use nalgebra::{vector, Vector2};

/// Setup [`Component`](impact_ecs::component::Component) for initializing
/// entities whose appearance is based on the colors associated with the mesh
/// vertices.
///
/// The purpose of this component is to aid in constructing a [`MaterialComp`]
/// for the entity. It is therefore not kept after entity creation.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct VertexColorComp;

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
/// entities that have an albedo (the proportion of incident light diffusely
/// reflected by the material).
///
/// The purpose of this component is to aid in constructing a [`MaterialComp`]
/// for the entity. It is therefore not kept after entity creation.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct AlbedoComp(pub RGBColor);

/// Setup [`Component`](impact_ecs::component::Component) for initializing
/// entities that have a textured albedo (the proportion of incident light
/// diffusely reflected by the material).
///
/// The purpose of this component is to aid in constructing a [`MaterialComp`]
/// for the entity. It is therefore not kept after entity creation.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct AlbedoTextureComp(pub TextureID);

/// Setup [`Component`](impact_ecs::component::Component) for initializing
/// entities that have a uniform specular reflectance at normal incidence (the
/// proportion of incident light specularly reflected by the material when the
/// light direction is perpendicular to the surface).
///
/// The purpose of this component is to aid in constructing a [`MaterialComp`]
/// for the entity. It is therefore not kept after entity creation.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct SpecularReflectanceComp(pub RGBColor);

/// Setup [`Component`](impact_ecs::component::Component) for initializing
/// entities that have a textured specular reflectance at normal incidence (the
/// proportion of incident light specularly reflected by the material when the
/// light direction is perpendicular to the surface).
///
/// The purpose of this component is to aid in constructing a [`MaterialComp`]
/// for the entity. It is therefore not kept after entity creation.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct SpecularReflectanceTextureComp(pub TextureID);

/// Setup [`Component`](impact_ecs::component::Component) for initializing
/// entities that have an emissive surface with a uniform luminance.
///
/// The purpose of this component is to aid in constructing a [`MaterialComp`]
/// for the entity. It is therefore not kept after entity creation.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct EmissiveLuminanceComp(pub RGBColor);

/// Setup [`Component`](impact_ecs::component::Component) for initializing
/// entities that have an emissive surface with a textured luminance.
///
/// The purpose of this component is to aid in constructing a [`MaterialComp`]
/// for the entity. It is therefore not kept after entity creation.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct EmissiveLuminanceTextureComp(pub TextureID);

/// Setup [`Component`](impact_ecs::component::Component) for initializing
/// entities that have a uniform roughness affecting the reflected light.
///
/// The purpose of this component is to aid in constructing a [`MaterialComp`]
/// for the entity. It is therefore not kept after entity creation.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct RoughnessComp(pub fre);

/// Setup [`Component`](impact_ecs::component::Component) for initializing
/// entities that have a textured roughness affecting the reflected light.
///
/// The purpose of this component is to aid in constructing a [`MaterialComp`]
/// for the entity. It is therefore not kept after entity creation.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct RoughnessTextureComp {
    pub texture_id: TextureID,
    pub roughness_scale: fre,
}

/// Setup [`Component`](impact_ecs::component::Component) for initializing
/// entities whose diffuse light reflection properties are described by a
/// microfacet model.
///
/// The purpose of this component is to aid in constructing a [`MaterialComp`]
/// for the entity. It is therefore not kept after entity creation.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct MicrofacetDiffuseReflectionComp;

/// Setup [`Component`](impact_ecs::component::Component) for initializing
/// entities whose specular light reflection properties are described by a
/// microfacet model.
///
/// The purpose of this component is to aid in constructing a [`MaterialComp`]
/// for the entity. It is therefore not kept after entity creation.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct MicrofacetSpecularReflectionComp;

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

/// Setup [`Component`](impact_ecs::component::Component) for initializing
/// entities representing a textured skybox.
///
/// The purpose of this component is to aid in constructing a [`MaterialComp`]
/// for the entity. It is therefore not kept after entity creation.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct SkyboxComp(pub TextureID);

/// [`Component`](impact_ecs::component::Component) for entities that
/// have a material.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct MaterialComp {
    material_handle: MaterialHandle,
    prepass_material_handle: MaterialHandle,
}

impl SpecularReflectanceComp {
    pub const IRON: Self = Self(vector![0.562, 0.565, 0.578]);
    pub const COPPER: Self = Self(vector![0.955, 0.638, 0.538]);
    pub const BRASS: Self = Self(vector![0.910, 0.778, 0.423]);
    pub const GOLD: Self = Self(vector![1.000, 0.782, 0.344]);
    pub const ALUMINUM: Self = Self(vector![0.913, 0.922, 0.924]);
    pub const SILVER: Self = Self(vector![0.972, 0.960, 0.915]);

    pub const WATER: Self = Self(vector![0.02, 0.02, 0.02]);
    pub const SKIN: Self = Self(vector![0.028, 0.028, 0.028]);

    pub const LIVING_TISSUE: (fre, fre) = (0.02, 0.04);
    pub const FABRIC: (fre, fre) = (0.04, 0.056);
    pub const STONE: (fre, fre) = (0.035, 0.056);
    pub const PLASTIC: (fre, fre) = (0.04, 0.05);
    pub const GLASS: (fre, fre) = (0.04, 0.05);

    pub fn in_range_of(range: (fre, fre), percentage: fre) -> Self {
        Self(vector![1.0, 1.0, 1.0] * (range.0 + 0.01 * percentage * (range.1 - range.0)))
    }
}

impl RoughnessComp {
    /// Converts the given shininess exponent for Blinn-Phong specular
    /// reflection into a corresponding roughness.
    pub fn from_blinn_phong_shininess(shininess: fre) -> Self {
        Self(fre::sqrt(2.0 / (shininess + 2.0)))
    }

    /// Converts the roughness into a corresponding shininess exponent for
    /// Blinn-Phong specular reflection.
    pub fn to_blinn_phong_shininess(&self) -> fre {
        2.0 / self.0.powi(2) - 2.0
    }

    pub fn to_ggx_roughness(&self) -> fre {
        self.0.powi(2)
    }
}

impl RoughnessTextureComp {
    pub fn unscaled(texture_id: TextureID) -> Self {
        Self {
            texture_id,
            roughness_scale: 1.0,
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
    /// Creates a new component representing the material with the given handle
    /// and (optionally) prepass material handle.
    pub fn new(
        material_handle: MaterialHandle,
        prepass_material_handle: Option<MaterialHandle>,
    ) -> Self {
        assert!(!material_handle.is_not_applicable());

        let prepass_material_handle = if let Some(prepass_material_handle) = prepass_material_handle
        {
            assert!(!material_handle.is_not_applicable());
            prepass_material_handle
        } else {
            MaterialHandle::not_applicable()
        };

        Self {
            material_handle,
            prepass_material_handle,
        }
    }

    /// Returns a reference to the handle for the entity's material.
    pub fn material_handle(&self) -> &MaterialHandle {
        &self.material_handle
    }

    /// Returns a reference to the handle for the prepass material associated
    /// with the entity's material, or [`None`] if the material has no prepass
    /// material.
    pub fn prepass_material_handle(&self) -> Option<&MaterialHandle> {
        if self.prepass_material_handle.is_not_applicable() {
            None
        } else {
            Some(&self.prepass_material_handle)
        }
    }
}

/// Registers all material [`Component`](impact_ecs::component::Component)s.
pub fn register_material_components(registry: &mut ComponentRegistry) -> Result<()> {
    register_setup_component!(registry, VertexColorComp)?;
    register_setup_component!(registry, FixedColorComp)?;
    register_setup_component!(registry, FixedTextureComp)?;
    register_setup_component!(registry, AlbedoComp)?;
    register_setup_component!(registry, AlbedoTextureComp)?;
    register_setup_component!(registry, SpecularReflectanceComp)?;
    register_setup_component!(registry, SpecularReflectanceTextureComp)?;
    register_setup_component!(registry, EmissiveLuminanceComp)?;
    register_setup_component!(registry, EmissiveLuminanceTextureComp)?;
    register_setup_component!(registry, RoughnessComp)?;
    register_setup_component!(registry, RoughnessTextureComp)?;
    register_setup_component!(registry, MicrofacetDiffuseReflectionComp)?;
    register_setup_component!(registry, MicrofacetSpecularReflectionComp)?;
    register_setup_component!(registry, NormalMapComp)?;
    register_setup_component!(registry, ParallaxMapComp)?;
    register_setup_component!(registry, SkyboxComp)?;
    register_component!(registry, MaterialComp)
}
