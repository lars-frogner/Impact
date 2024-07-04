//! Materials with a fixed color or texture.

use super::MATERIAL_VERTEX_BINDING_START;
use crate::{
    geometry::{InstanceFeature, VertexAttributeSet},
    gpu::{
        rendering::{
            Assets, FixedColorFeatureShaderInput, FixedTextureShaderInput,
            InstanceFeatureShaderInput, MaterialShaderInput, RenderAttachmentQuantitySet,
            RenderPassHints,
        },
        GraphicsDevice,
    },
    impl_InstanceFeature,
    scene::{
        FixedColorComp, FixedTextureComp, InstanceFeatureManager, MaterialComp, MaterialHandle,
        MaterialID, MaterialLibrary, MaterialPropertyTextureGroup, MaterialPropertyTextureGroupID,
        MaterialSpecification, RGBColor, RenderResourcesDesynchronized,
    },
};
use bytemuck::{Pod, Zeroable};
use impact_ecs::{archetype::ArchetypeComponentStorage, setup};
use impact_utils::hash64;
use lazy_static::lazy_static;
use std::sync::RwLock;

/// Material with a fixed, uniform color that is independent
/// of lighting.
///
/// This type stores the material's per-instance data that will
/// be sent to the GPU. It implements [`InstanceFeature`], and
/// can thus be stored in an
/// [`InstanceFeatureStorage`](crate::geometry::InstanceFeatureStorage).
#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct FixedColorMaterial {
    color: RGBColor,
}

/// Marker type for a material with a fixed, textured color that
/// is independent of lighting.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Zeroable, Pod)]
pub struct FixedTextureMaterial;

lazy_static! {
    static ref FIXED_COLOR_MATERIAL_ID: MaterialID = MaterialID(hash64!("FixedColorMaterial"));
    static ref FIXED_TEXTURE_MATERIAL_ID: MaterialID = MaterialID(hash64!("FixedTextureMaterial"));
}

impl FixedColorMaterial {
    pub const VERTEX_ATTRIBUTE_REQUIREMENTS_FOR_SHADER: VertexAttributeSet =
        VertexAttributeSet::empty();
    pub const VERTEX_ATTRIBUTE_REQUIREMENTS_FOR_MESH: VertexAttributeSet =
        Self::VERTEX_ATTRIBUTE_REQUIREMENTS_FOR_SHADER;

    /// Registers this material as a feature type in the given
    /// instance feature manager and adds the material specification
    /// to the given material library.
    pub fn register(
        material_library: &mut MaterialLibrary,
        instance_feature_manager: &mut InstanceFeatureManager,
    ) {
        instance_feature_manager.register_feature_type::<Self>();

        let specification = MaterialSpecification::new(
            Self::VERTEX_ATTRIBUTE_REQUIREMENTS_FOR_MESH,
            Self::VERTEX_ATTRIBUTE_REQUIREMENTS_FOR_SHADER,
            RenderAttachmentQuantitySet::empty(),
            RenderAttachmentQuantitySet::LUMINANCE,
            None,
            vec![Self::FEATURE_TYPE_ID],
            RenderPassHints::empty(),
            MaterialShaderInput::Fixed(None),
        );
        material_library.add_material_specification(*FIXED_COLOR_MATERIAL_ID, specification);
    }

    /// Checks if the entity-to-be with the given components has the component
    /// for this material, and if so, registers the material in the given
    /// instance feature manager and adds the appropriate material component
    /// to the entity.
    pub fn add_material_component_for_entity(
        instance_feature_manager: &RwLock<InstanceFeatureManager>,
        components: &mut ArchetypeComponentStorage,
        desynchronized: &mut RenderResourcesDesynchronized,
    ) {
        setup!(
            {
                desynchronized.set_yes();
                let mut instance_feature_manager = instance_feature_manager.write().unwrap();
            },
            components,
            |fixed_color: &FixedColorComp| -> MaterialComp {
                let material = Self {
                    color: fixed_color.0,
                };

                let feature_id = instance_feature_manager
                    .get_storage_mut::<Self>()
                    .expect("Missing storage for FixedColorMaterial features")
                    .add_feature(&material);

                MaterialComp::new(
                    MaterialHandle::new(*FIXED_COLOR_MATERIAL_ID, Some(feature_id), None),
                    None,
                )
            },
            ![MaterialComp]
        );
    }
}

impl FixedTextureMaterial {
    pub const VERTEX_ATTRIBUTE_REQUIREMENTS_FOR_SHADER: VertexAttributeSet =
        VertexAttributeSet::TEXTURE_COORDS;
    pub const VERTEX_ATTRIBUTE_REQUIREMENTS_FOR_MESH: VertexAttributeSet =
        Self::VERTEX_ATTRIBUTE_REQUIREMENTS_FOR_SHADER;

    const MATERIAL_SHADER_INPUT: MaterialShaderInput =
        MaterialShaderInput::Fixed(Some(FixedTextureShaderInput {
            color_texture_and_sampler_bindings:
                MaterialPropertyTextureGroup::get_texture_and_sampler_bindings(0),
        }));

    /// Adds the material specification to the given material library.
    pub fn register(material_library: &mut MaterialLibrary) {
        let specification = MaterialSpecification::new(
            Self::VERTEX_ATTRIBUTE_REQUIREMENTS_FOR_MESH,
            Self::VERTEX_ATTRIBUTE_REQUIREMENTS_FOR_SHADER,
            RenderAttachmentQuantitySet::empty(),
            RenderAttachmentQuantitySet::LUMINANCE,
            None,
            Vec::new(),
            RenderPassHints::empty(),
            Self::MATERIAL_SHADER_INPUT,
        );
        material_library.add_material_specification(*FIXED_TEXTURE_MATERIAL_ID, specification);
    }

    /// Checks if the entity-to-be with the given components has the component
    /// for this material, and if so, adds the appropriate material property
    /// texture set to the material library if not present and adds the
    /// appropriate material component to the entity.
    pub fn add_material_component_for_entity(
        graphics_device: &GraphicsDevice,
        assets: &Assets,
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
            |fixed_texture: &FixedTextureComp| -> MaterialComp {
                let texture_ids = vec![fixed_texture.0];

                let texture_group_id =
                    MaterialPropertyTextureGroupID::from_texture_ids(&texture_ids);

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
                    MaterialHandle::new(*FIXED_TEXTURE_MATERIAL_ID, None, Some(texture_group_id)),
                    None,
                )
            },
            ![MaterialComp]
        );
    }
}

impl_InstanceFeature!(
    FixedColorMaterial,
    wgpu::vertex_attr_array![MATERIAL_VERTEX_BINDING_START => Float32x3],
    InstanceFeatureShaderInput::FixedColorMaterial(FixedColorFeatureShaderInput {
        color_location: MATERIAL_VERTEX_BINDING_START,
    })
);
