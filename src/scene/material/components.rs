//! [`Component`](impact_ecs::component::Component)s related to materials.

use crate::{
    rendering::{fre, TextureID},
    scene::{MaterialHandle, RGBColor},
};
use bytemuck::{Pod, Zeroable};
use impact_ecs::Component;
use nalgebra::{vector, Vector2};

/// Marker [`Component`](impact_ecs::component::Component) for entities
/// using the colors of the mesh vertices.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct VertexColorComp;

/// [`Component`](impact_ecs::component::Component) for entities that
/// have a fixed, uniform color that is independent of lighting.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct FixedColorComp(pub RGBColor);

/// [`Component`](impact_ecs::component::Component) for entities that
/// have a fixed, textured color that is independent of lighting.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct FixedTextureComp(pub TextureID);

#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct DiffuseColorComp(pub RGBColor);

#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct DiffuseTextureComp(pub TextureID);

#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct SpecularColorComp(pub RGBColor);

#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct SpecularTextureComp(pub TextureID);

#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct EmissiveColorComp(pub RGBColor);

#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct EmissiveTextureComp(pub TextureID);

#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct RoughnessComp(pub fre);

#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct RoughnessTextureComp {
    pub texture_id: TextureID,
    pub roughness_scale: fre,
}

#[repr(transparent)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct MicrofacetDiffuseReflection;

#[repr(transparent)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct MicrofacetSpecularReflection;

#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct NormalMapComp(pub TextureID);

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
    prepass_material_handle: MaterialHandle,
}

impl SpecularColorComp {
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
    /// Converts the roughness into a corresponding shininess exponent for
    /// Blinn-Phong specular reflection.
    pub fn from_blinn_phong_shininess(shininess: fre) -> Self {
        Self(fre::ln(8192.0 / shininess) / fre::ln(8192.0))
    }

    /// Converts the given shininess exponent for Blinn-Phong specular
    /// reflection into a corresponding roughness.
    pub fn to_blinn_phong_shininess(&self) -> fre {
        fre::powf(8192.0, 1.0 - self.0)
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
