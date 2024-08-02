//! Material using the colors of the mesh vertices.

use crate::{
    gpu::{
        rendering::render_command::{Blending, RenderPipelineHints},
        shader::MaterialShaderInput,
        texture::attachment::{
            RenderAttachmentInputDescriptionSet, RenderAttachmentOutputDescription,
            RenderAttachmentOutputDescriptionSet, RenderAttachmentQuantity,
        },
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
use std::sync::RwLock;

lazy_static! {
    static ref VERTEX_COLOR_MATERIAL_ID: MaterialID = MaterialID(hash64!("VertexColorMaterial"));
}

/// Checks if the entity-to-be with the given components has the
/// component for this material, and if so, adds the appropriate
/// material component to the entity.
pub fn setup_vertex_color_material_for_new_entity(
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
            let output_render_attachments = RenderAttachmentOutputDescriptionSet::single(
                RenderAttachmentQuantity::Luminance,
                RenderAttachmentOutputDescription::default().with_blending(Blending::Additive),
            );
            MaterialSpecification::new(
                VertexAttributeSet::COLOR,
                VertexAttributeSet::COLOR,
                RenderAttachmentInputDescriptionSet::empty(),
                output_render_attachments,
                None,
                Vec::new(),
                RenderPipelineHints::empty(),
                MaterialShaderInput::VertexColor,
            )
        });

    MaterialComp::new(
        MaterialHandle::new(*VERTEX_COLOR_MATERIAL_ID, None, None),
        None,
    )
}
