//! Material using the colors of the mesh vertices.

use std::sync::RwLock;

use crate::{
    gpu::{
        rendering::{RenderAttachmentQuantitySet, RenderPassHints},
        shader::MaterialShaderInput,
    },
    material::{
        components::{MaterialComp, VertexColorComp},
        MaterialHandle, MaterialID, MaterialLibrary, MaterialSpecification,
    },
    mesh::VertexAttributeSet,
};
use impact_ecs::{archetype::ArchetypeComponentStorage, setup};
use impact_utils::hash64;
use lazy_static::lazy_static;

lazy_static! {
    static ref VERTEX_COLOR_MATERIAL_ID: MaterialID = MaterialID(hash64!("VertexColorMaterial"));
}

/// Checks if the entity-to-be with the given components has the
/// component for this material, and if so, adds the appropriate
/// material component to the entity.
pub fn add_vertex_color_material_component_for_entity(
    material_library: &RwLock<MaterialLibrary>,
    components: &mut ArchetypeComponentStorage,
) {
    setup!(
        {
            let mut material_library = material_library.write().unwrap();
        },
        components,
        || -> MaterialComp { setup_vertex_color_material(&mut material_library) },
        [VertexColorComp],
        ![MaterialComp]
    );
}

pub fn setup_vertex_color_material(material_library: &mut MaterialLibrary) -> MaterialComp {
    material_library
        .material_specification_entry(*VERTEX_COLOR_MATERIAL_ID)
        .or_insert_with(|| {
            MaterialSpecification::new(
                VertexAttributeSet::COLOR,
                VertexAttributeSet::COLOR,
                RenderAttachmentQuantitySet::empty(),
                RenderAttachmentQuantitySet::LUMINANCE,
                None,
                Vec::new(),
                RenderPassHints::empty(),
                MaterialShaderInput::VertexColor,
            )
        });

    MaterialComp::new(
        MaterialHandle::new(*VERTEX_COLOR_MATERIAL_ID, None, None),
        None,
    )
}
