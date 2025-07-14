//! Reference frames.

use bytemuck::{Pod, Zeroable};
use impact_math::Float;
use nalgebra::{Point3, Similarity3, Translation3, UnitQuaternion, Vector3};
use roc_integration::roc;

define_component_type! {
    /// A reference frame defined an origin position, an orientation and a scale
    /// factor, as well as an internal offset for displacing the origin within
    /// the reference frame.
    #[roc(parents = "Comp")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct ReferenceFrame {
        /// The offset, expressed in the entity's reference frame (before scaling),
        /// from the original origin of the entity's reference frame to the point
        /// that should be used as the actual origin.
        pub origin_offset: Vector3<f64>,
        /// The coordinates of the origin of the entity's reference frame measured
        /// in the parent space.
        pub position: Point3<f64>,
        /// The 3D orientation of the entity's reference frame in the parent space.
        pub orientation: UnitQuaternion<f64>,
        /// The uniform scale factor of the entity's reference frame (distance in
        /// world space per distance in the reference frame).
        pub scaling: f64,
    }
}

#[roc]
impl ReferenceFrame {
    /// Creates a new reference frame with the given position, orientation and
    /// scaling, retaining the original origin of the entity's reference frame.
    #[roc(body = "{ origin_offset: Vector3.zero, position, orientation, scaling }")]
    pub fn new(position: Point3<f64>, orientation: UnitQuaternion<f64>, scaling: f64) -> Self {
        Self::scaled_with_offset_origin(Vector3::zeros(), position, orientation, scaling)
    }

    /// Creates a new reference frame with the given position and orientation,
    /// retaining the original origin of the entity's reference frame and no
    /// scaling.
    #[roc(body = "new(position, orientation, 1.0)")]
    pub fn unscaled(position: Point3<f64>, orientation: UnitQuaternion<f64>) -> Self {
        Self::new(position, orientation, 1.0)
    }

    /// Creates a new reference frame with the given position, retaining the
    /// original origin of the entity's reference frame and the identity
    /// orientation and scaling.
    #[roc(body = "unoriented_scaled(position, 1.0)")]
    pub fn unoriented(position: Point3<f64>) -> Self {
        Self::unoriented_scaled(position, 1.0)
    }

    /// Creates a new reference frame with the given position and scaling,
    /// retaining the original origin of the entity's reference frame and the
    /// identity orientation.
    #[roc(body = "new(position, UnitQuaternion.identity, scaling)")]
    pub fn unoriented_scaled(position: Point3<f64>, scaling: f64) -> Self {
        Self::new(position, UnitQuaternion::identity(), scaling)
    }

    /// Creates a new reference frame with the given orientation, retaining the
    /// original origin of the entity's reference frame and located at the
    /// origin with no scaling.
    #[roc(body = "unlocated_scaled(orientation, 1.0)")]
    pub fn unlocated(orientation: UnitQuaternion<f64>) -> Self {
        Self::unlocated_scaled(orientation, 1.0)
    }

    /// Creates a new reference frame with the given orientation and scaling,
    /// retaining the original origin of the entity's reference frame and
    /// located at the origin.
    #[roc(body = "new(Point3.origin, orientation, scaling)")]
    pub fn unlocated_scaled(orientation: UnitQuaternion<f64>, scaling: f64) -> Self {
        Self::new(Point3::origin(), orientation, scaling)
    }

    /// Creates a new reference frame with the given scaling, retaining the
    /// original origin of the entity's reference frame and located at the
    /// origin with the identity orientation.
    #[roc(body = "unoriented_scaled(Point3.origin, scaling)")]
    pub fn scaled(scaling: f64) -> Self {
        Self::unoriented_scaled(Point3::origin(), scaling)
    }

    /// Creates a new reference frame with the given origin offset and position,
    /// and with the identity orientation and scaling.
    #[roc(body = "unoriented_scaled_with_offset_origin(origin_offset, position, 1.0)")]
    pub fn unoriented_with_offset_origin(
        origin_offset: Vector3<f64>,
        position: Point3<f64>,
    ) -> Self {
        Self::unoriented_scaled_with_offset_origin(origin_offset, position, 1.0)
    }

    /// Creates a new reference frame with the given origin offset, position and
    /// scaling, and with the identity orientation.
    #[roc(
        body = "scaled_with_offset_origin(origin_offset, position, UnitQuaternion.identity, scaling)"
    )]
    pub fn unoriented_scaled_with_offset_origin(
        origin_offset: Vector3<f64>,
        position: Point3<f64>,
        scaling: f64,
    ) -> Self {
        Self::scaled_with_offset_origin(
            origin_offset,
            position,
            UnitQuaternion::identity(),
            scaling,
        )
    }

    /// Creates a new reference frame with the given origin offset, position
    /// orientation, and scaling.
    #[roc(body = "{ origin_offset, position, orientation, scaling }")]
    pub fn scaled_with_offset_origin(
        origin_offset: Vector3<f64>,
        position: Point3<f64>,
        orientation: UnitQuaternion<f64>,
        scaling: f64,
    ) -> Self {
        Self {
            origin_offset,
            position,
            orientation,
            scaling,
        }
    }

    /// Creates a new reference frame with the given origin offset, position and
    /// orientation and no scaling.
    #[roc(body = "scaled_with_offset_origin(origin_offset, position, orientation, 1.0)")]
    pub fn with_offset_origin(
        origin_offset: Vector3<f64>,
        position: Point3<f64>,
        orientation: UnitQuaternion<f64>,
    ) -> Self {
        Self::scaled_with_offset_origin(origin_offset, position, orientation, 1.0)
    }

    /// Creates the [`Similarity3`] transform from the entity's reference frame
    /// to the parent space.
    pub fn create_transform_to_parent_space<F: Float>(&self) -> Similarity3<F> {
        Similarity3::from_parts(
            Translation3::from(self.position.cast::<F>()),
            self.orientation.cast::<F>(),
            F::from_f64(self.scaling).unwrap(),
        ) * Translation3::from(-self.origin_offset.cast::<F>())
    }

    /// Sets the reference frame's origin offset to the given vector, adjusting
    /// the frame's origin position in the parent space such that the positions
    /// within the reference frame map to the same positions in the parent space
    /// as before.
    pub fn update_origin_offset_while_preserving_position(&mut self, origin_offset: Vector3<f64>) {
        let displacement_in_frame = origin_offset - self.origin_offset;
        let displacement_in_parent_frame = self
            .orientation
            .transform_vector(&(self.scaling * displacement_in_frame));
        self.origin_offset = origin_offset;
        self.position += displacement_in_parent_frame;
    }
}

impl Default for ReferenceFrame {
    fn default() -> Self {
        Self {
            origin_offset: Vector3::zeros(),
            position: Point3::origin(),
            orientation: UnitQuaternion::identity(),
            scaling: 1.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_abs_diff_eq;
    use nalgebra::{UnitQuaternion, point, vector};

    #[test]
    fn updating_origin_offset_while_preserving_position_works() {
        let position = point![1.0, 2.0, 3.0];
        let orientation = UnitQuaternion::from_euler_angles(0.1, 0.2, 0.3);
        let scaling = 1.5;
        let original_origin_offset = vector![4.0, 2.0, 3.0];
        let new_origin_offset = vector![4.5, 1.5, 1.0];

        let mut frame = ReferenceFrame::scaled_with_offset_origin(
            original_origin_offset,
            position,
            orientation,
            scaling,
        );
        let point_within_frame = point![-2.0, 0.5, 3.0];
        let point_before = frame
            .create_transform_to_parent_space()
            .transform_point(&point_within_frame);

        frame.update_origin_offset_while_preserving_position(new_origin_offset);

        let point_after = frame
            .create_transform_to_parent_space()
            .transform_point(&point_within_frame);

        assert_eq!(frame.orientation, orientation);
        assert_eq!(frame.scaling, scaling);
        assert_eq!(frame.origin_offset, new_origin_offset);
        assert_abs_diff_eq!(point_after, point_before, epsilon = 1e-6);
    }
}
