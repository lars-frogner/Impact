//! Material with a fixed color.

use crate::{
    geometry::InstanceFeature,
    impl_InstanceFeature_for_VertexBufferable,
    rendering::{self, Assets, ShaderID, VertexBufferable},
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
    /// Registers this material as a feature type in the given
    /// instance feature manager, prepares a shader for the
    /// material and adds the material specification to the given
    /// material library. Because this material uses no textures,
    /// the same material specification can be used for all
    /// instances using the material.
    pub fn register(
        assets: &mut Assets,
        material_library: &mut MaterialLibrary,
        instance_feature_manager: &mut InstanceFeatureManager,
    ) {
        instance_feature_manager.register_feature_type::<Self>();

        // Construct shader with correct features and get ID (create ShaderBuilder).
        // Shader ID is added to assets if not present.

        let specification = MaterialSpecification::new(
            ShaderID(hash64!("FixedColorMaterial")),
            Vec::new(),
            vec![Self::FEATURE_TYPE_ID],
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

impl VertexBufferable for FixedColorMaterial {
    const BUFFER_LAYOUT: wgpu::VertexBufferLayout<'static> =
        rendering::create_vertex_buffer_layout_for_vertex::<Self>(
            &wgpu::vertex_attr_array![0 => Float32x4],
        );
}

impl_InstanceFeature_for_VertexBufferable!(FixedColorMaterial);
