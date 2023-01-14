//! Material with a fixed color.

use crate::{
    geometry::InstanceFeature,
    impl_InstanceFeature,
    rendering::{
        FixedColorFeatureShaderInput, InstanceFeatureShaderInput, MaterialTextureShaderInput,
    },
    scene::{
        FixedColorComp, InstanceFeatureManager, MaterialComp, MaterialID, MaterialLibrary,
        MaterialSpecification, RGBAColor,
    },
};
use bytemuck::{Pod, Zeroable};
use impact_ecs::{archetype::ComponentManager, setup};
use impact_utils::hash64;
use lazy_static::lazy_static;

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
    color: RGBAColor,
}

lazy_static! {
    static ref FIXED_COLOR_MATERIAL_ID: MaterialID = MaterialID(hash64!("FixedColorMaterial"));
}

impl FixedColorMaterial {
    const MATERIAL_TEXTURE_SHADER_INPUT: MaterialTextureShaderInput =
        MaterialTextureShaderInput::None;

    /// Registers this material as a feature type in the given
    /// instance feature manager and adds the material specification
    /// to the given material library. Because this material uses no
    /// textures, the same material specification can be used for all
    /// instances using the material.
    pub fn register(
        material_library: &mut MaterialLibrary,
        instance_feature_manager: &mut InstanceFeatureManager,
    ) {
        instance_feature_manager.register_feature_type::<Self>();

        let specification = MaterialSpecification::new(
            Vec::new(),
            vec![Self::FEATURE_TYPE_ID],
            Self::MATERIAL_TEXTURE_SHADER_INPUT,
        );
        material_library.add_material_specification(*FIXED_COLOR_MATERIAL_ID, specification);
    }

    /// Checks if the entity-to-be with components represented by the
    /// given component manager has the component for this material, and
    /// if so, registers the material in the given instance feature
    /// manager and adds the appropriate material component to the entity.
    pub fn add_material_component_for_entity(
        instance_feature_manager: &mut InstanceFeatureManager,
        component_manager: &mut ComponentManager<'_>,
    ) {
        setup!(
            component_manager,
            |fixed_color: &FixedColorComp| -> MaterialComp {
                let material = Self {
                    color: fixed_color.0,
                };

                let feature_id = instance_feature_manager
                    .get_storage_mut::<Self>()
                    .expect("Missing storage for FixedColorMaterial features")
                    .add_feature(&material);

                MaterialComp {
                    id: *FIXED_COLOR_MATERIAL_ID,
                    feature_id,
                }
            },
            ![MaterialComp]
        );
    }
}

impl_InstanceFeature!(
    FixedColorMaterial,
    wgpu::vertex_attr_array![0 => Float32x4],
    InstanceFeatureShaderInput::FixedColorMaterial(FixedColorFeatureShaderInput {
        color_location: 0,
    })
);
