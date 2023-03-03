//! [`Component`](impact_ecs::component::Component)s related to materials.

use crate::{
    geometry::InstanceFeatureID,
    rendering::{fre, TextureID},
    scene::{MaterialID, MaterialPropertyTextureSetID, RGBColor},
};
use bytemuck::{Pod, Zeroable};
use impact_ecs::Component;

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
pub struct BlinnPhongComp {
    pub diffuse: RGBColor,
    pub specular: RGBColor,
    pub shininess: fre,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct DiffuseTexturedBlinnPhongComp {
    pub specular: RGBColor,
    pub diffuse: TextureID,
    pub shininess: fre,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct TexturedBlinnPhongComp {
    pub diffuse: TextureID,
    pub specular: TextureID,
    pub shininess: fre,
}

/// Marker [`Component`](impact_ecs::component::Component) for entities
/// colored according to their depth in the clip space of a light source.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct LightSpaceDepthComp;

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
