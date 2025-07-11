//! Model instance transforms.

use crate::{InstanceFeatureManager, impl_InstanceFeatureForGPU};
use bytemuck::{Pod, Zeroable};
use impact_gpu::vertex_attribute_ranges::INSTANCE_START;
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

/// A model-to-light transform for a specific instance of a model.
///
/// This struct is intended to be passed to the GPU in a vertex buffer. The
/// order of the fields is assumed in the shaders.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct InstanceModelLightTransform(InstanceModelViewTransform);

impl InstanceModelViewTransform {
    /// Returns the binding location of the transform's rotation quaternion in
    /// the instance buffer.
    pub const fn rotation_location() -> u32 {
        INSTANCE_START
    }

    /// Returns the binding location of the transform's translation and scaling
    /// in the instance buffer.
    pub const fn translation_and_scaling_location() -> u32 {
        INSTANCE_START + 1
    }

    /// Creates a new identity transform.
    pub fn identity() -> Self {
        Self {
            rotation: UnitQuaternion::identity(),
            translation: Vector3::zeros(),
            scaling: 1.0,
        }
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
        INSTANCE_START
    }

    /// Returns the binding location of the current frame's transform's
    /// translation and scaling in the instance buffer.
    pub const fn current_translation_and_scaling_location() -> u32 {
        INSTANCE_START + 1
    }

    /// Returns the binding location of the previous frame's transform's
    /// rotation quaternion in the instance buffer.
    pub const fn previous_rotation_location() -> u32 {
        INSTANCE_START + 2
    }

    /// Returns the binding location of the previous frame's transform's
    /// translation and scaling in the instance buffer.
    pub const fn previous_translation_and_scaling_location() -> u32 {
        INSTANCE_START + 3
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

impl InstanceModelLightTransform {
    /// Returns the binding location of the transform's rotation quaternion in
    /// the instance buffer.
    pub const fn rotation_location() -> u32 {
        INSTANCE_START
    }

    /// Returns the binding location of the transform's translation and scaling
    /// in the instance buffer.
    pub const fn translation_and_scaling_location() -> u32 {
        INSTANCE_START + 1
    }

    /// Creates a new identity transform.
    pub fn identity() -> Self {
        Self(InstanceModelViewTransform::identity())
    }
}

impl From<Similarity3<f32>> for InstanceModelLightTransform {
    fn from(transform: Similarity3<f32>) -> Self {
        Self(InstanceModelViewTransform::from(transform))
    }
}

impl From<InstanceModelLightTransform> for Similarity3<f32> {
    fn from(transform: InstanceModelLightTransform) -> Self {
        transform.0.into()
    }
}

impl Default for InstanceModelLightTransform {
    fn default() -> Self {
        Self::identity()
    }
}

impl AsInstanceModelViewTransform for InstanceModelLightTransform {
    fn as_instance_model_view_transform(&self) -> &InstanceModelViewTransform {
        &self.0
    }
}

impl_InstanceFeatureForGPU!(
    InstanceModelViewTransform,
    wgpu::vertex_attr_array![
        INSTANCE_START => Float32x4,
        INSTANCE_START + 1 => Float32x4,
    ]
);

impl_InstanceFeatureForGPU!(
    InstanceModelViewTransformWithPrevious,
    wgpu::vertex_attr_array![
        INSTANCE_START => Float32x4,
        INSTANCE_START + 1 => Float32x4,
        INSTANCE_START + 2 => Float32x4,
        INSTANCE_START + 3 => Float32x4,
    ]
);

impl_InstanceFeatureForGPU!(
    InstanceModelLightTransform,
    wgpu::vertex_attr_array![
        INSTANCE_START => Float32x4,
        INSTANCE_START + 1 => Float32x4,
    ]
);

pub fn register_model_feature_types<MID: Clone + Eq + Hash>(
    instance_feature_manager: &mut InstanceFeatureManager<MID>,
) {
    instance_feature_manager.register_feature_type::<InstanceModelViewTransform>();
    instance_feature_manager.register_feature_type::<InstanceModelViewTransformWithPrevious>();
    instance_feature_manager.register_feature_type::<InstanceModelLightTransform>();
}
