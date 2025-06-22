//! Model instance transforms.

use crate::{InstanceFeatureManager, impl_InstanceFeature};
use bytemuck::{Pod, Zeroable};
use impact_gpu::wgpu;
use nalgebra::{Similarity3, UnitQuaternion, Vector3};
use std::hash::Hash;

/// Trait for types that can be referenced as an [`InstanceModelViewTransform`].
pub trait AsInstanceModelViewTransform {
    /// Returns a reference to the [`InstanceModelViewTransform`].
    fn as_instance_model_view_transform(&self) -> &InstanceModelViewTransform;
}

/// A model-to-camera transform for a specific instance of a model.
///
/// This struct is intended to be passed to the GPU in a vertex buffer. The
/// order of the fields is assumed in the shaders.
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct InstanceModelViewTransform {
    pub rotation: UnitQuaternion<f32>,
    pub translation: Vector3<f32>,
    pub scaling: f32,
}

pub type InstanceModelLightTransform = InstanceModelViewTransform;

/// A model-to-camera transform for a specific instance of a model, along with
/// the corresponding transform from the previous frame.
///
/// This struct is intended to be passed to the GPU in a vertex buffer. The
/// order of the fields is assumed in the shaders.
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Default, Zeroable, Pod)]
pub struct InstanceModelViewTransformWithPrevious {
    pub current: InstanceModelViewTransform,
    pub previous: InstanceModelViewTransform,
}

const INSTANCE_VERTEX_BINDING_START: u32 = 0;

impl InstanceModelViewTransform {
    /// Returns the binding location of the transform's rotation quaternion in
    /// the instance buffer.
    pub const fn rotation_location() -> u32 {
        INSTANCE_VERTEX_BINDING_START
    }

    /// Returns the binding location of the transform's translation and scaling
    /// in the instance buffer.
    pub const fn translation_and_scaling_location() -> u32 {
        INSTANCE_VERTEX_BINDING_START + 1
    }

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
    pub fn with_model_view_transform(transform: Similarity3<f32>) -> Self {
        let scaling = transform.scaling();

        Self {
            rotation: transform.isometry.rotation,
            translation: transform.isometry.translation.vector,
            scaling,
        }
    }

    /// Creates a new model-to-light transform corresponding to the given
    /// similarity transform.
    pub fn with_model_light_transform(transform: Similarity3<f32>) -> Self {
        Self::with_model_view_transform(transform)
    }
}

impl From<Similarity3<f32>> for InstanceModelViewTransform {
    fn from(transform: Similarity3<f32>) -> Self {
        InstanceModelViewTransform {
            rotation: transform.isometry.rotation,
            translation: transform.isometry.translation.vector,
            scaling: transform.scaling(),
        }
    }
}

impl From<InstanceModelViewTransform> for Similarity3<f32> {
    fn from(transform: InstanceModelViewTransform) -> Self {
        let InstanceModelViewTransform {
            rotation,
            translation,
            scaling,
        } = transform;
        Similarity3::from_parts(translation.into(), rotation, scaling)
    }
}

impl Default for InstanceModelViewTransform {
    fn default() -> Self {
        Self::identity()
    }
}

impl AsInstanceModelViewTransform for InstanceModelViewTransform {
    fn as_instance_model_view_transform(&self) -> &InstanceModelViewTransform {
        self
    }
}

impl InstanceModelViewTransformWithPrevious {
    /// Returns the binding location of the current frame's transform's rotation
    /// quaternion in the instance buffer.
    pub const fn current_rotation_location() -> u32 {
        INSTANCE_VERTEX_BINDING_START
    }

    /// Returns the binding location of the current frame's transform's
    /// translation and scaling in the instance buffer.
    pub const fn current_translation_and_scaling_location() -> u32 {
        INSTANCE_VERTEX_BINDING_START + 1
    }

    /// Returns the binding location of the previous frame's transform's
    /// rotation quaternion in the instance buffer.
    pub const fn previous_rotation_location() -> u32 {
        INSTANCE_VERTEX_BINDING_START + 2
    }

    /// Returns the binding location of the previous frame's transform's
    /// translation and scaling in the instance buffer.
    pub const fn previous_translation_and_scaling_location() -> u32 {
        INSTANCE_VERTEX_BINDING_START + 3
    }

    /// Uses the identity transform for the previous frame.
    pub fn current_only(transform: InstanceModelViewTransform) -> Self {
        Self {
            current: transform,
            previous: InstanceModelViewTransform::identity(),
        }
    }

    /// Sets the transform for the current frame to the given transform and the
    /// transform for the previous frame to the replaced transform.
    pub fn set_transform_for_new_frame(&mut self, transform: InstanceModelViewTransform) {
        self.previous = self.current;
        self.current = transform;
    }
}

impl AsInstanceModelViewTransform for InstanceModelViewTransformWithPrevious {
    fn as_instance_model_view_transform(&self) -> &InstanceModelViewTransform {
        &self.current
    }
}

impl_InstanceFeature!(
    InstanceModelViewTransform,
    wgpu::vertex_attr_array![
        INSTANCE_VERTEX_BINDING_START => Float32x4,
        INSTANCE_VERTEX_BINDING_START + 1 => Float32x4,
    ]
);

impl_InstanceFeature!(
    InstanceModelViewTransformWithPrevious,
    wgpu::vertex_attr_array![
        INSTANCE_VERTEX_BINDING_START => Float32x4,
        INSTANCE_VERTEX_BINDING_START + 1 => Float32x4,
        INSTANCE_VERTEX_BINDING_START + 2 => Float32x4,
        INSTANCE_VERTEX_BINDING_START + 3 => Float32x4,
    ]
);

pub fn register_model_feature_types<MID: Eq + Hash>(
    instance_feature_manager: &mut InstanceFeatureManager<MID>,
) {
    instance_feature_manager.register_feature_type::<InstanceModelViewTransform>();
    instance_feature_manager.register_feature_type::<InstanceModelViewTransformWithPrevious>();
}
