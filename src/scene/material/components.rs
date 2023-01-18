//! [`Component`](impact_ecs::component::Component)s related to materials.

use crate::{
    geometry::InstanceFeatureID,
    rendering::{fre, TextureID},
    scene::{MaterialID, RGBAColor, RGBColor},
};
use bytemuck::{Pod, Zeroable};
use impact_ecs::Component;

/// [`Component`](impact_ecs::component::Component) for entities that
/// have a material.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct MaterialComp {
    /// The ID of the entity's [`MaterialSpecification`](crate::scene::MaterialSpecification).
    pub id: MaterialID,
    /// The ID of the entry for the entity's per-instance material
    /// data in the [InstanceFeatureStorage](crate::geometry::InstanceFeatureStorage).
    pub feature_id: InstanceFeatureID,
}

/// [`Component`](impact_ecs::component::Component) for entities that
/// have a fixed, uniform color that is independent of lighting.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct FixedColorComp(pub RGBAColor);

/// [`Component`](impact_ecs::component::Component) for entities that
/// have a fixed, textured color that is independent of lighting.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct FixedTextureComp(pub TextureID);

#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct BlinnPhongComp {
    pub ambient: RGBColor,
    pub diffuse: RGBColor,
    pub specular: RGBColor,
    pub shininess: fre,
    pub alpha: fre,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct DiffuseTexturedBlinnPhongComp {
    pub ambient: RGBColor,
    pub specular: RGBColor,
    pub diffuse: TextureID,
    pub shininess: fre,
    pub alpha: fre,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct TexturedBlinnPhongComp {
    pub ambient: RGBColor,
    pub diffuse: TextureID,
    pub specular: TextureID,
    pub shininess: fre,
    pub alpha: fre,
}

impl MaterialComp {
    /// Creates a new component representing a material with the given
    /// IDs for the [`MaterialSpecification`](crate::scene::MaterialSpecification)
    /// and the per-instance material data.
    pub fn new(material_id: MaterialID, feature_id: InstanceFeatureID) -> Self {
        Self {
            id: material_id,
            feature_id,
        }
    }
}
