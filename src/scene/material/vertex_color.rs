//! Material using the colors of the mesh vertices.

use crate::{
    geometry::VertexAttributeSet,
    rendering::{MaterialShaderInput, RenderAttachmentQuantitySet, RenderPassHints},
    scene::{
        MaterialComp, MaterialHandle, MaterialID, MaterialLibrary, MaterialSpecification,
        RenderResourcesDesynchronized, VertexColorComp,
    },
};
use bytemuck::{Pod, Zeroable};
use impact_ecs::{archetype::ArchetypeComponentStorage, setup};
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
    pub const VERTEX_ATTRIBUTE_REQUIREMENTS_FOR_SHADER: VertexAttributeSet =
        VertexAttributeSet::COLOR;
    pub const VERTEX_ATTRIBUTE_REQUIREMENTS_FOR_MESH: VertexAttributeSet =
        Self::VERTEX_ATTRIBUTE_REQUIREMENTS_FOR_SHADER;

    /// Adds the material specification for this material to the given
    /// material library.
    pub fn register(material_library: &mut MaterialLibrary) {
        let specification = MaterialSpecification::new(
            Self::VERTEX_ATTRIBUTE_REQUIREMENTS_FOR_MESH,
            Self::VERTEX_ATTRIBUTE_REQUIREMENTS_FOR_SHADER,
            RenderAttachmentQuantitySet::empty(),
            RenderAttachmentQuantitySet::LUMINANCE,
            None,
            Vec::new(),
            RenderPassHints::empty(),
            MaterialShaderInput::VertexColor,
        );
        material_library.add_material_specification(*VERTEX_COLOR_MATERIAL_ID, specification);
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
            || -> MaterialComp {
                MaterialComp::new(
                    MaterialHandle::new(*VERTEX_COLOR_MATERIAL_ID, None, None),
                    None,
                )
            },
            [VertexColorComp],
            ![MaterialComp]
        );
    }
}
