//! Model transforms.

use crate::ReferenceFrame;
use bytemuck::{Pod, Zeroable};
use impact_math::{point::Point3, transform::Similarity3, vector::Vector3};
use roc_integration::roc;

define_component_type! {
    /// The similarity transform from the local space of a model to the space of
    /// a parent entity using the model.
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

#[roc]
impl ModelTransform {
    /// Creates a transform where the parent entity's space is identical to that
    /// of the model.
    #[roc(body = "with_scale(1.0)")]
    pub fn identity() -> Self {
        Self::with_scale(1.0)
    }

    /// Creates a transform where the parent entity's space has the given offset
    /// from that of the model.
    #[roc(body = "with_offset_and_scale(offset, 1.0)")]
    pub fn with_offset(offset: Vector3) -> Self {
        Self::with_offset_and_scale(offset, 1.0)
    }

    /// Creates a transform where the parent entity's space has the given scale
    /// relative to that of the model.
    #[roc(body = "with_offset_and_scale(Vector3.zero, scale)")]
    pub fn with_scale(scale: f32) -> Self {
        Self::with_offset_and_scale(Vector3::zeros(), scale)
    }

    /// Creates a transform where the parent entity's space has the given offset
    /// and scale relative to that of the model.
    #[roc(body = "{ offset, scale }")]
    pub fn with_offset_and_scale(offset: Vector3, scale: f32) -> Self {
        Self { offset, scale }
    }

    /// Creates the [`Similarity3`] for the transform from model space to the
    /// space of the parent entity.
    pub fn crate_transform_to_entity_space(&self) -> Similarity3 {
        Similarity3::from_scaled_translation(-self.offset, self.scale)
    }

    /// Transforms the given point from model space to the space of the parent
    /// entity.
    pub fn transform_point_from_model_space_to_entity_space(&self, point: &Point3) -> Point3 {
        (point - self.offset) * self.scale
    }

    /// Updates the pre-scaling offset to yield the given offset after scaling.
    pub fn set_offset_after_scaling(&mut self, offset_after_scaling: Vector3) {
        self.offset = offset_after_scaling / self.scale;
    }

    /// Sets the pre-scaling offset to the given vector, adjusting the given
    /// entity frame's origin position in its parent space such that the
    /// positions within the reference frame map to the same positions in the
    /// entity's parent space as before.
    pub fn update_offset_while_preserving_entity_position(
        &mut self,
        entity_frame: &mut ReferenceFrame,
        offset: Vector3,
    ) {
        let displacement_in_frame = offset - self.offset;
        let displacement_in_parent_frame = entity_frame
            .orientation
            .transform_vector(&(self.scale * displacement_in_frame));
        self.offset = offset;
        entity_frame.position += displacement_in_parent_frame;
    }
}

impl Default for ModelTransform {
    fn default() -> Self {
        Self::identity()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_abs_diff_eq;
    use impact_math::{point::Point3, quaternion::UnitQuaternion};

    #[test]
    fn updating_offset_while_preserving_position_works() {
        let position = Point3::new(1.0, 2.0, 3.0);
        let orientation = UnitQuaternion::from_euler_angles(0.1, 0.2, 0.3);
        let scale = 1.5;
        let original_offset = Vector3::new(4.0, 2.0, 3.0);
        let new_offset = Vector3::new(4.5, 1.5, 1.0);

        let mut model_transform = ModelTransform::with_offset_and_scale(original_offset, scale);
        let mut frame = ReferenceFrame::new(position, orientation);
        let point_within_frame = Point3::new(-2.0, 0.5, 3.0);
        let point_before = (frame.create_transform_to_parent_space()
            * model_transform.crate_transform_to_entity_space())
        .transform_point(&point_within_frame);

        model_transform.update_offset_while_preserving_entity_position(&mut frame, new_offset);

        let point_after = (frame.create_transform_to_parent_space()
            * model_transform.crate_transform_to_entity_space())
        .transform_point(&point_within_frame);

        assert_eq!(frame.orientation, orientation);
        assert_eq!(model_transform.scale, scale);
        assert_eq!(model_transform.offset, new_offset);
        assert_abs_diff_eq!(point_after, point_before, epsilon = 1e-6);
    }
}
