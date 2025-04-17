//! Materials with a fixed color or texture.

use super::super::features::FixedColorMaterialFeature;
use crate::{
    assets::Assets,
    gpu::GraphicsDevice,
    material::{
        MaterialHandle, MaterialID, MaterialInstanceFeatureFlags, MaterialLibrary,
        MaterialPropertyTextureGroup, MaterialPropertyTextureGroupID, MaterialShaderInput,
        MaterialSpecification,
        components::{FixedColorComp, FixedTextureComp, MaterialComp},
    },
    mesh::VertexAttributeSet,
    model::{InstanceFeature, InstanceFeatureManager},
    scene::RenderResourcesDesynchronized,
};
use impact_ecs::{archetype::ArchetypeComponentStorage, setup};
use impact_math::hash64;
use lazy_static::lazy_static;
use std::sync::RwLock;

/// Binding locations for textures used in a fixed material.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FixedMaterialTextureBindings {
    pub color_texture_and_sampler_bindings: Option<(u32, u32)>,
}

lazy_static! {
    static ref FIXED_COLOR_MATERIAL_ID: MaterialID = MaterialID(hash64!("FixedColorMaterial"));
    static ref FIXED_TEXTURE_MATERIAL_ID: MaterialID = MaterialID(hash64!("FixedTextureMaterial"));
}

/// Checks if the entity-to-be with the given components has the component
/// for this material, and if so, registers the material in the given
/// instance feature manager and adds the appropriate material component
/// to the entity.
pub fn setup_fixed_color_material_for_new_entity(
    material_library: &RwLock<MaterialLibrary>,
    instance_feature_manager: &RwLock<InstanceFeatureManager>,
    components: &mut ArchetypeComponentStorage,
    desynchronized: &mut RenderResourcesDesynchronized,
) {
    setup!(
        {
            desynchronized.set_yes();
            let mut material_library = material_library.write().unwrap();
            let mut instance_feature_manager = instance_feature_manager.write().unwrap();
        },
        components,
        |fixed_color: &FixedColorComp| -> MaterialComp {
            setup_fixed_color_material(
                &mut material_library,
                &mut instance_feature_manager,
                fixed_color,
            )
        },
        ![MaterialComp]
    );
}

/// Checks if the entity-to-be with the given components has the component
/// for this material, and if so, adds the appropriate material property
/// texture set to the material library if not present and adds the
/// appropriate material component to the entity.
pub fn setup_fixed_texture_material_for_new_entity(
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
        |fixed_texture: &FixedTextureComp| -> MaterialComp {
            setup_fixed_texture_material(
                graphics_device,
                assets,
                &mut material_library,
                fixed_texture,
            )
        },
        ![MaterialComp]
    );
}

fn setup_fixed_color_material(
    material_library: &mut MaterialLibrary,
    instance_feature_manager: &mut InstanceFeatureManager,
    fixed_color: &FixedColorComp,
) -> MaterialComp {
    let feature_id = instance_feature_manager
        .get_storage_mut::<FixedColorMaterialFeature>()
        .expect("Missing storage for FixedColorMaterialFeature features")
        .add_feature(&FixedColorMaterialFeature::new(fixed_color.0));

    material_library
        .material_specification_entry(*FIXED_COLOR_MATERIAL_ID)
        .or_insert_with(|| {
            MaterialSpecification::new(
                VertexAttributeSet::empty(),
                vec![FixedColorMaterialFeature::FEATURE_TYPE_ID],
                MaterialInstanceFeatureFlags::HAS_COLOR,
                None,
                MaterialShaderInput::Fixed(FixedMaterialTextureBindings {
                    color_texture_and_sampler_bindings: None,
                }),
            )
        });

    MaterialComp::new(MaterialHandle::new(
        *FIXED_COLOR_MATERIAL_ID,
        Some(feature_id),
        None,
    ))
}

fn setup_fixed_texture_material(
    graphics_device: &GraphicsDevice,
    assets: &Assets,
    material_library: &mut MaterialLibrary,
    fixed_texture: &FixedTextureComp,
) -> MaterialComp {
    material_library
        .material_specification_entry(*FIXED_TEXTURE_MATERIAL_ID)
        .or_insert_with(|| {
            MaterialSpecification::new(
                VertexAttributeSet::TEXTURE_COORDS,
                Vec::new(),
                MaterialInstanceFeatureFlags::empty(),
                None,
                MaterialShaderInput::Fixed(FixedMaterialTextureBindings {
                    color_texture_and_sampler_bindings: Some(
                        MaterialPropertyTextureGroup::get_texture_and_sampler_bindings(0),
                    ),
                }),
            )
        });

    let texture_ids = vec![fixed_texture.0];

    let texture_group_id = MaterialPropertyTextureGroupID::from_texture_ids(&texture_ids);

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

    MaterialComp::new(MaterialHandle::new(
        *FIXED_TEXTURE_MATERIAL_ID,
        None,
        Some(texture_group_id),
    ))
}
