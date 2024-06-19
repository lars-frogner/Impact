//! Material for rendering a skybox.

use crate::{
    geometry::VertexAttributeSet,
    rendering::{
        MaterialPropertyTextureManager, MaterialShaderInput, RenderAttachmentQuantitySet,
        RenderPassHints, SkyboxTextureShaderInput,
    },
    scene::{
        material::SkyboxComp, MaterialComp, MaterialHandle, MaterialID, MaterialLibrary,
        MaterialPropertyTextureSet, MaterialPropertyTextureSetID, MaterialSpecification,
        RenderResourcesDesynchronized,
    },
};
use impact_ecs::{archetype::ArchetypeComponentStorage, setup};
use impact_utils::hash64;
use lazy_static::lazy_static;
use std::sync::RwLock;

lazy_static! {
    static ref SKYBOX_MATERIAL_ID: MaterialID = MaterialID(hash64!("SkyboxMaterial"));
}

/// Checks if the entity-to-be with the given components has the component for a
/// skybox material, and if so, adds the material specification to the material
/// library if not already present, adds the appropriate material property
/// texture set to the material library if not already present and adds the
/// appropriate material component to the entity.
pub fn add_skybox_material_component_for_entity(
    material_library: &RwLock<MaterialLibrary>,
    components: &mut ArchetypeComponentStorage,
    desynchronized: &mut RenderResourcesDesynchronized,
) {
    setup!(
        {
            desynchronized.set_yes();
            let mut material_library = material_library.write().unwrap();
        },
        components,
        |skybox: &SkyboxComp| -> MaterialComp {
            let vertex_attribute_requirements_for_mesh = VertexAttributeSet::POSITION;
            let vertex_attribute_requirements_for_shader = VertexAttributeSet::empty();

            let texture_shader_input = SkyboxTextureShaderInput {
                skybox_cubemap_texture_and_sampler_bindings:
                    MaterialPropertyTextureManager::get_texture_and_sampler_bindings(0),
            };

            let texture_ids = vec![skybox.0];

            // Add material specification unless a specification for the same material exists
            material_library
                .material_specification_entry(*SKYBOX_MATERIAL_ID)
                .or_insert_with(|| {
                    MaterialSpecification::new(
                        vertex_attribute_requirements_for_mesh,
                        vertex_attribute_requirements_for_shader,
                        RenderAttachmentQuantitySet::empty(),
                        RenderAttachmentQuantitySet::SURFACE,
                        None,
                        Vec::new(),
                        RenderPassHints::NO_DEPTH_PREPASS,
                        MaterialShaderInput::Skybox(texture_shader_input),
                    )
                });

            let texture_set_id = MaterialPropertyTextureSetID::from_texture_ids(&texture_ids);

            // Add a new texture set if none with the same textures already exist
            material_library
                .material_property_texture_set_entry(texture_set_id)
                .or_insert_with(|| MaterialPropertyTextureSet::new(texture_ids));

            MaterialComp::new(
                MaterialHandle::new(*SKYBOX_MATERIAL_ID, None, Some(texture_set_id)),
                None,
            )
        },
        ![MaterialComp]
    );
}
