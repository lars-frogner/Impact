//! Material for rendering a skybox.

use crate::{
    assert_uniform_valid,
    assets::Assets,
    gpu::{
        rendering::{fre, render_command::RenderPassHints},
        shader::{MaterialShaderInput, SkyboxShaderInput},
        texture::attachment::RenderAttachmentQuantitySet,
        uniform::{self, SingleUniformGPUBuffer, UniformBufferable},
        GraphicsDevice,
    },
    material::{
        components::{MaterialComp, SkyboxComp},
        MaterialHandle, MaterialID, MaterialLibrary, MaterialPropertyTextureGroup,
        MaterialPropertyTextureGroupID, MaterialSpecificResourceGroup, MaterialSpecification,
    },
    mesh::VertexAttributeSet,
};
use bytemuck::{Pod, Zeroable};
use impact_ecs::{archetype::ArchetypeComponentStorage, setup};
use impact_utils::{hash64, ConstStringHash64};
use lazy_static::lazy_static;
use std::{borrow::Cow, sync::RwLock};

lazy_static! {
    static ref SKYBOX_MATERIAL_ID: MaterialID = MaterialID(hash64!("SkyboxMaterial"));
}

/// Uniform holding the maximum possible luminance from a skybox.
///
/// The size of this struct has to be a multiple of 16 bytes as required for
/// uniforms.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
struct SkyboxProperties {
    max_luminance: fre,
    _pad: [u8; 12],
}

impl SkyboxProperties {
    fn new(max_luminance: fre) -> Self {
        Self {
            max_luminance,
            _pad: [0; 12],
        }
    }
}

impl UniformBufferable for SkyboxProperties {
    const ID: ConstStringHash64 = ConstStringHash64::new("Skybox properties");

    fn create_bind_group_layout_entry(
        binding: u32,
        visibility: wgpu::ShaderStages,
    ) -> wgpu::BindGroupLayoutEntry {
        uniform::create_uniform_buffer_bind_group_layout_entry(binding, visibility)
    }
}
assert_uniform_valid!(SkyboxProperties);

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
    let properties_uniform = SkyboxProperties::new(skybox.max_luminance);

    let properties_uniform_buffer = SingleUniformGPUBuffer::for_uniform(
        graphics_device,
        &properties_uniform,
        wgpu::ShaderStages::FRAGMENT,
        Cow::Borrowed("Skybox properties"),
    );
    let material_specific_resources = MaterialSpecificResourceGroup::new(
        graphics_device,
        vec![properties_uniform_buffer],
        &[],
        "Skybox properties",
    );

    let texture_shader_input = SkyboxShaderInput {
        parameters_uniform_binding: 0,
        skybox_cubemap_texture_and_sampler_bindings:
            MaterialPropertyTextureGroup::get_texture_and_sampler_bindings(0),
    };

    let texture_ids = vec![skybox.texture_id];

    // Add material specification unless a specification for the same material exists
    material_library
        .material_specification_entry(*SKYBOX_MATERIAL_ID)
        .or_insert_with(|| {
            MaterialSpecification::new(
                VertexAttributeSet::POSITION,
                VertexAttributeSet::empty(),
                RenderAttachmentQuantitySet::empty(),
                RenderAttachmentQuantitySet::LUMINANCE,
                Some(material_specific_resources),
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
