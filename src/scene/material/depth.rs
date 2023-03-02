//! Materials visualizing the depth of each fragment.

use crate::{
    geometry::VertexAttributeSet,
    rendering::MaterialShaderInput,
    scene::{
        LightSpaceDepthComp, MaterialComp, MaterialID, MaterialLibrary, MaterialSpecification,
        RenderResourcesDesynchronized,
    },
};
use bytemuck::{Pod, Zeroable};
use impact_ecs::{archetype::ArchetypeComponentStorage, setup};
use impact_utils::hash64;
use lazy_static::lazy_static;

/// Marker type for a material visualizing the depth of each fragment in the
/// clip space of a light source.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Zeroable, Pod)]
pub struct LightSpaceDepthMaterial;

lazy_static! {
    static ref LIGHT_SPACE_DEPTH_MATERIAL_ID: MaterialID =
        MaterialID(hash64!("LightSpaceDepthMaterial"));
}

impl LightSpaceDepthMaterial {
    pub const VERTEX_ATTRIBUTE_REQUIREMENTS: VertexAttributeSet = VertexAttributeSet::POSITION;

    const MATERIAL_SHADER_INPUT: MaterialShaderInput = MaterialShaderInput::LightSpaceDepth;

    /// Adds the material specification for this material to the given
    /// material library. Because this material uses no textures, the
    /// same material specification can be used for all instances using
    /// the material.
    pub fn register(material_library: &mut MaterialLibrary) {
        let specification = MaterialSpecification::new(
            Self::VERTEX_ATTRIBUTE_REQUIREMENTS,
            Vec::new(),
            Self::MATERIAL_SHADER_INPUT,
        );
        material_library.add_material_specification(*LIGHT_SPACE_DEPTH_MATERIAL_ID, specification);
    }

    /// Checks if the entity-to-be with the given components has the
    /// component for this material, and if so, adds the appropriate
    /// material component to the entity.
    pub fn add_material_component_for_entity(
        components: &mut ArchetypeComponentStorage,
        _desynchronized: &mut RenderResourcesDesynchronized,
    ) {
        setup!(
            components,
            || -> MaterialComp { MaterialComp::new(*LIGHT_SPACE_DEPTH_MATERIAL_ID, None, None) },
            [LightSpaceDepthComp],
            ![MaterialComp]
        );
    }
}
