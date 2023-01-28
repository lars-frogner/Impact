//! Material using the colors of the mesh vertices.

use crate::{
    geometry::InstanceFeatureID,
    rendering::MaterialTextureShaderInput,
    scene::{MaterialComp, MaterialID, MaterialLibrary, MaterialSpecification, VertexColorComp},
};
use bytemuck::{Pod, Zeroable};
use impact_ecs::{archetype::ComponentManager, setup};
use impact_utils::hash64;
use lazy_static::lazy_static;

/// Marker type for a material using the interpolated vertex colors
/// of a mesh to determine fragment color.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Zeroable, Pod)]
pub struct VertexColorMaterial;

lazy_static! {
    static ref VERTEX_COLOR_MATERIAL_ID: MaterialID = MaterialID(hash64!("VertexColorMaterial"));
}

impl VertexColorMaterial {
    const MATERIAL_TEXTURE_SHADER_INPUT: MaterialTextureShaderInput =
        MaterialTextureShaderInput::None;

    /// Adds the material specification for this material to the given
    /// material library. Because this material uses no textures, the
    /// same material specification can be used for all instances using
    /// the material.
    pub fn register(material_library: &mut MaterialLibrary) {
        let specification =
            MaterialSpecification::new(Vec::new(), Vec::new(), Self::MATERIAL_TEXTURE_SHADER_INPUT);
        material_library.add_material_specification(*VERTEX_COLOR_MATERIAL_ID, specification);
    }

    /// Checks if the entity-to-be with components represented by the
    /// given component manager has the component for this material, and
    /// if so, adds the appropriate material component to the entity.
    pub fn add_material_component_for_entity(component_manager: &mut ComponentManager<'_>) {
        setup!(
            component_manager,
            || -> MaterialComp {
                MaterialComp {
                    id: *VERTEX_COLOR_MATERIAL_ID,
                    feature_id: InstanceFeatureID::not_applicable(),
                }
            },
            [VertexColorComp],
            ![MaterialComp]
        );
    }
}
