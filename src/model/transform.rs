//! Model instance transforms.

use crate::{
    gpu::{
        rendering::fre,
        shader::{InstanceFeatureShaderInput, ModelViewTransformShaderInput},
    },
    impl_InstanceFeature,
    model::InstanceFeatureManager,
};
use bytemuck::{Pod, Zeroable};
use nalgebra::{Similarity3, UnitQuaternion, Vector3};

/// A model-to-camera transform for a specific instance of a model.
///
/// This struct is intended to be passed to the GPU in a vertex buffer. The
/// order of the fields is assumed in the shaders.
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct InstanceModelViewTransform {
    pub rotation: UnitQuaternion<fre>,
    pub translation: Vector3<fre>,
    pub scaling: fre,
}

/// A model-to-light transform for a specific instance of a model.
pub type InstanceModelLightTransform = InstanceModelViewTransform;

const INSTANCE_VERTEX_BINDING_START: u32 = 0;

impl InstanceModelViewTransform {
    /// Creates a new identity transform.
    pub fn identity() -> Self {
        Self {
            rotation: UnitQuaternion::identity(),
            translation: Vector3::zeros(),
            scaling: 1.0,
        }
    }

    /// Creates a new model-to-camera transform corresponding to the given
    /// similarity transform.
    pub fn with_model_view_transform(transform: Similarity3<fre>) -> Self {
        let scaling = transform.scaling();

        Self {
            rotation: transform.isometry.rotation,
            translation: transform.isometry.translation.vector,
            scaling,
        }
    }

    #[cfg(test)]
    pub fn dummy_instance_feature_id() -> super::InstanceFeatureID {
        use crate::model::InstanceFeature;
        super::InstanceFeatureID {
            feature_type_id: InstanceModelViewTransform::FEATURE_TYPE_ID,
            idx: 0,
        }
    }
}

impl InstanceModelLightTransform {
    /// Creates a new model-to-light transform corresponding to the given
    /// similarity transform.
    pub fn with_model_light_transform(transform: Similarity3<fre>) -> Self {
        Self::with_model_view_transform(transform)
    }
}

impl Default for InstanceModelViewTransform {
    fn default() -> Self {
        Self::identity()
    }
}

impl_InstanceFeature!(
    InstanceModelViewTransform,
    wgpu::vertex_attr_array![
        INSTANCE_VERTEX_BINDING_START => Float32x4,
        INSTANCE_VERTEX_BINDING_START + 1 => Float32x4,
    ],
    InstanceFeatureShaderInput::ModelViewTransform(ModelViewTransformShaderInput {
        rotation_location: INSTANCE_VERTEX_BINDING_START,
        translation_and_scaling_location: INSTANCE_VERTEX_BINDING_START + 1,
    })
);

pub fn register_model_feature_types(instance_feature_manager: &mut InstanceFeatureManager) {
    instance_feature_manager.register_feature_type::<InstanceModelViewTransform>();
}
