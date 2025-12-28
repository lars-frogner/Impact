//! Model transforms.

use crate::reference_frame::ReferenceFrameA;
use bytemuck::{Pod, Zeroable};
use impact_math::{
    point::Point3A,
    transform::Similarity3A,
    vector::{Vector3, Vector3A},
};
use roc_integration::roc;

define_component_type! {
    /// The similarity transform from the local space of a model to the space of
    /// a parent entity using the model.
    ///
    /// This type only supports a few basic operations, as is primarily intended for
    /// compact storage inside other types and collections. For computations, prefer
    /// the SIMD-friendly 16-byte aligned [`ModelTransformA`].
    #[repr(C)]
    #[roc(parents = "Comp")]
    #[derive(Clone, Copy, Debug, PartialEq, Zeroable, Pod)]
    pub struct ModelTransform {
        /// The offset subtracted from a model-space position before scaling to
        /// transform it to the parent entity's space.
        pub offset: Vector3,
        /// The scaling factor applied to a model-space position after the
        /// offset to transform it to the parent entity's space.
        pub scale: f32,
    }
}

/// The similarity transform from the local space of a model to the space of
/// a parent entity using the model.
///
/// The offset is stored in a 128-bit SIMD register for efficient computation.
/// That leads to an extra 16 bytes in size (4 due to the padded offset and 12
/// due to padding after the scale factor). For cache-friendly storage, prefer
/// [`ModelTransform`].
#[derive(Clone, Debug, PartialEq)]
pub struct ModelTransformA {
    /// The offset subtracted from a model-space position before scaling to
    /// transform it to the parent entity's space.
    pub offset: Vector3A,
    /// The scaling factor applied to a model-space position after the
    /// offset to transform it to the parent entity's space.
    pub scale: f32,
}

#[roc]
impl ModelTransform {
    /// Creates a transform where the parent entity's space is identical to that
    /// of the model.
    #[roc(body = "with_scale(1.0)")]
    #[inline]
    pub const fn identity() -> Self {
        Self::with_scale(1.0)
    }

    /// Creates a transform where the parent entity's space has the given offset
    /// from that of the model.
    #[roc(body = "with_offset_and_scale(offset, 1.0)")]
    #[inline]
    pub const fn with_offset(offset: Vector3) -> Self {
        Self::with_offset_and_scale(offset, 1.0)
    }

    /// Creates a transform where the parent entity's space has the given scale
    /// relative to that of the model.
    #[roc(body = "with_offset_and_scale(Vector3.zero, scale)")]
    #[inline]
    pub const fn with_scale(scale: f32) -> Self {
        Self::with_offset_and_scale(Vector3::zeros(), scale)
    }

    /// Creates a transform where the parent entity's space has the given offset
    /// and scale relative to that of the model.
    #[roc(body = "{ offset, scale }")]
    #[inline]
    pub const fn with_offset_and_scale(offset: Vector3, scale: f32) -> Self {
        Self { offset, scale }
    }

    /// Converts the transform to the 16-byte aligned SIMD-friendly
    /// [`ModelTransformA`].
    #[inline]
    pub fn aligned(&self) -> ModelTransformA {
        ModelTransformA::with_offset_and_scale(self.offset.aligned(), self.scale)
    }
}

impl Default for ModelTransform {
    #[inline]
    fn default() -> Self {
        Self::identity()
    }
}

impl ModelTransformA {
    /// Creates a transform where the parent entity's space is identical to that
    /// of the model.
    #[inline]
    pub const fn identity() -> Self {
        Self::with_scale(1.0)
    }

    /// Creates a transform where the parent entity's space has the given offset
    /// from that of the model.
    #[inline]
    pub const fn with_offset(offset: Vector3A) -> Self {
        Self::with_offset_and_scale(offset, 1.0)
    }

    /// Creates a transform where the parent entity's space has the given scale
    /// relative to that of the model.
    #[inline]
    pub const fn with_scale(scale: f32) -> Self {
        Self::with_offset_and_scale(Vector3A::zeros(), scale)
    }

    /// Creates a transform where the parent entity's space has the given offset
    /// and scale relative to that of the model.
    #[inline]
    pub const fn with_offset_and_scale(offset: Vector3A, scale: f32) -> Self {
        Self { offset, scale }
    }

    /// Creates the [`Similarity3A`] for the transform from model space to the
    /// space of the parent entity.
    #[inline]
    pub fn create_transform_to_entity_space(&self) -> Similarity3A {
        Similarity3A::from_scaled_translation(-self.offset, self.scale)
    }

    /// Transforms the given point from model space to the space of the parent
    /// entity.
    #[inline]
    pub fn transform_point_from_model_space_to_entity_space(&self, point: &Point3A) -> Point3A {
        (point - self.offset) * self.scale
    }

    /// Updates the pre-scaling offset to yield the given offset after scaling.
    #[inline]
    pub fn set_offset_after_scaling(&mut self, offset_after_scaling: Vector3A) {
        self.offset = offset_after_scaling / self.scale;
    }

    /// Sets the pre-scaling offset to the given vector, adjusting the given
    /// entity frame's origin position in its parent space such that the
    /// positions within the reference frame map to the same positions in the
    /// entity's parent space as before.
    #[inline]
    pub fn update_offset_while_preserving_entity_position(
        &mut self,
        entity_frame: &mut ReferenceFrameA,
        offset: Vector3A,
    ) {
        let displacement_in_frame = offset - self.offset;
        let displacement_in_parent_frame = entity_frame
            .orientation
            .rotate_vector(&(self.scale * displacement_in_frame));
        self.offset = offset;
        entity_frame.position += displacement_in_parent_frame;
    }

    /// Converts the transform to the 4-byte aligned cache-friendly
    /// [`ModelTransform`].
    #[inline]
    pub fn unaligned(&self) -> ModelTransform {
        ModelTransform::with_offset_and_scale(self.offset.unaligned(), self.scale)
    }
}

impl Default for ModelTransformA {
    #[inline]
    fn default() -> Self {
        Self::identity()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_abs_diff_eq;
    use impact_math::quaternion::UnitQuaternionA;

    #[test]
    fn updating_offset_while_preserving_position_works() {
        let position = Point3A::new(1.0, 2.0, 3.0);
        let orientation = UnitQuaternionA::from_euler_angles(0.1, 0.2, 0.3);
        let scale = 1.5;
        let original_offset = Vector3A::new(4.0, 2.0, 3.0);
        let new_offset = Vector3A::new(4.5, 1.5, 1.0);

        let mut model_transform = ModelTransformA::with_offset_and_scale(original_offset, scale);
        let mut frame = ReferenceFrameA::new(position, orientation);
        let point_within_frame = Point3A::new(-2.0, 0.5, 3.0);
        let point_before = (frame.create_transform_to_parent_space()
            * model_transform.create_transform_to_entity_space())
        .transform_point(&point_within_frame);

        model_transform.update_offset_while_preserving_entity_position(&mut frame, new_offset);

        let point_after = (frame.create_transform_to_parent_space()
            * model_transform.create_transform_to_entity_space())
        .transform_point(&point_within_frame);

        assert_eq!(frame.orientation, orientation);
        assert_eq!(model_transform.scale, scale);
        assert_eq!(model_transform.offset, new_offset);
        assert_abs_diff_eq!(point_after, point_before, epsilon = 1e-6);
    }
}
