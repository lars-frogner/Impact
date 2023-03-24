//! [`Component`](impact_ecs::component::Component)s related to materials.

use crate::{
    geometry::InstanceFeatureID,
    rendering::{fre, TextureID},
    scene::{MaterialID, MaterialPropertyTextureSetID, RGBColor},
};
use bytemuck::{Pod, Zeroable};
use impact_ecs::Component;
use nalgebra::vector;

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
pub struct RoughnessComp(pub fre);

#[repr(transparent)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct MicrofacetDiffuseReflection;

#[repr(transparent)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct MicrofacetSpecularReflection;

/// [`Component`](impact_ecs::component::Component) for entities that
/// have a material.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct MaterialComp {
    /// The ID of the entity's [`MaterialSpecification`](crate::scene::MaterialSpecification).
    pub material_id: MaterialID,
    /// The ID of the entry for the entity's per-instance material properties in
    /// the [InstanceFeatureStorage](crate::geometry::InstanceFeatureStorage)
    /// (may be N/A).
    pub material_property_feature_id: InstanceFeatureID,
    /// The ID of the entity's
    /// [`MaterialPropertyTextureSet`](crate::scene::MaterialPropertyTextureSet)
    /// (may represent an empty set).
    pub material_property_texture_set_id: MaterialPropertyTextureSetID,
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

impl MaterialComp {
    /// Creates a new component representing a material with the given
    /// IDs for the [`MaterialSpecification`](crate::scene::MaterialSpecification)
    /// and the per-instance material data (which is optional).
    pub fn new(
        material_id: MaterialID,
        material_property_feature_id: Option<InstanceFeatureID>,
        material_property_texture_set_id: Option<MaterialPropertyTextureSetID>,
    ) -> Self {
        let material_property_feature_id =
            material_property_feature_id.unwrap_or_else(InstanceFeatureID::not_applicable);
        let material_property_texture_set_id =
            material_property_texture_set_id.unwrap_or_else(MaterialPropertyTextureSetID::empty);
        Self {
            material_id,
            material_property_feature_id,
            material_property_texture_set_id,
        }
    }

    /// Returns the ID of the entry for the entity's per-instance material
    /// properties in the
    /// [`InstanceFeatureStorage`](crate::geometry::InstanceFeatureStorage), or
    /// [`None`] if there are no untextured per-instance material properties.
    pub fn material_property_feature_id(&self) -> Option<InstanceFeatureID> {
        if self.material_property_feature_id.is_not_applicable() {
            None
        } else {
            Some(self.material_property_feature_id)
        }
    }

    /// Returns the ID of the material property texture set, or [`None`] if no material properties are textured.
    pub fn material_property_texture_set_id(&self) -> Option<MaterialPropertyTextureSetID> {
        if self.material_property_texture_set_id.is_empty() {
            None
        } else {
            Some(self.material_property_texture_set_id)
        }
    }
}
