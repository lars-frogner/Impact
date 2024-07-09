//! Material for rendering a skybox.

use crate::{
    assets::Assets,
    gpu::{
        rendering::{RenderAttachmentQuantitySet, RenderPassHints},
        shader::{MaterialShaderInput, SkyboxTextureShaderInput},
        GraphicsDevice,
    },
    material::{
        components::{MaterialComp, SkyboxComp},
        MaterialHandle, MaterialID, MaterialLibrary, MaterialPropertyTextureGroup,
        MaterialPropertyTextureGroupID, MaterialSpecification,
    },
    mesh::VertexAttributeSet,
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
pub fn setup_skybox_material_for_new_entity(
    graphics_device: &GraphicsDevice,
    assets: &Assets,
    material_library: &RwLock<MaterialLibrary>,
    components: &mut ArchetypeComponentStorage,
) {
    setup!(
        {
            let mut material_library = material_library.write().unwrap();
        },
        components,
        |skybox: &SkyboxComp| -> MaterialComp {
            setup_skybox_material(graphics_device, assets, &mut material_library, skybox)
        },
        ![MaterialComp]
    );
}

pub fn setup_skybox_material(
    graphics_device: &GraphicsDevice,
    assets: &Assets,
    material_library: &mut MaterialLibrary,
    skybox: &SkyboxComp,
) -> MaterialComp {
    let texture_shader_input = SkyboxTextureShaderInput {
        skybox_cubemap_texture_and_sampler_bindings:
            MaterialPropertyTextureGroup::get_texture_and_sampler_bindings(0),
    };

    let texture_ids = vec![skybox.0];

    // Add material specification unless a specification for the same material exists
    material_library
        .material_specification_entry(*SKYBOX_MATERIAL_ID)
        .or_insert_with(|| {
            MaterialSpecification::new(
                VertexAttributeSet::POSITION,
                VertexAttributeSet::empty(),
                RenderAttachmentQuantitySet::empty(),
                RenderAttachmentQuantitySet::LUMINANCE,
                None,
                Vec::new(),
                RenderPassHints::NO_DEPTH_PREPASS,
                MaterialShaderInput::Skybox(texture_shader_input),
            )
        });

    let texture_group_id = MaterialPropertyTextureGroupID::from_texture_ids(&texture_ids);

    // Add a new texture set if none with the same textures already exist
    material_library
        .material_property_texture_group_entry(texture_group_id)
        .or_insert_with(|| {
            MaterialPropertyTextureGroup::new(
                graphics_device,
                assets,
                texture_ids,
                texture_group_id.to_string(),
            )
            .expect("Missing textures from assets")
        });

    MaterialComp::new(
        MaterialHandle::new(*SKYBOX_MATERIAL_ID, None, Some(texture_group_id)),
        None,
    )
}