//! Representation of boxes with arbitrary orientations.

use crate::{geometry::AxisAlignedBox, num::Float};
use nalgebra::{Point3, Similarity3, UnitQuaternion, UnitVector3, Vector3};

/// A box with arbitrary position, orientation and extents.
#[derive(Clone, Debug)]
pub struct OrientedBox<F: Float> {
    center: Point3<F>,
    orientation: UnitQuaternion<F>,
    half_width: F,
    half_height: F,
    half_depth: F,
}

impl<F: Float> OrientedBox<F> {
    /// Creates a new box with the given center position, orientation quaternion
    /// and half extents along each of its three axes.
    pub fn new(
        center: Point3<F>,
        orientation: UnitQuaternion<F>,
        half_width: F,
        half_height: F,
        half_depth: F,
    ) -> Self {
        Self {
            center,
            orientation,
            half_width,
            half_height,
            half_depth,
        }
    }

    /// Creates a new box with the given half extents, centered at the origin and
    /// with the width, height and depth axes aligned with the x-, y- and z-axis
    /// respectively.
    pub fn aligned_at_origin(half_width: F, half_height: F, half_depth: F) -> Self {
        Self::new(
            Point3::origin(),
            UnitQuaternion::identity(),
            half_width,
            half_height,
            half_depth,
        )
    }

    /// Creates a new box corresponding to the given axis aligned box.
    pub fn from_axis_aligned_box(axis_aligned_box: &AxisAlignedBox<F>) -> Self {
        Self::new(
            axis_aligned_box.center(),
            UnitQuaternion::identity(),
            F::ONE_HALF * axis_aligned_box.extent_x(),
            F::ONE_HALF * axis_aligned_box.extent_y(),
            F::ONE_HALF * axis_aligned_box.extent_z(),
        )
    }

    /// Returns the center of the box.
    pub fn center(&self) -> &Point3<F> {
        &self.center
    }

    /// Returns the orientation of the box.
    pub fn orientation(&self) -> &UnitQuaternion<F> {
        &self.orientation
    }

    /// Returns half the width of the box.
    pub fn half_width(&self) -> F {
        self.half_width
    }

    /// Returns half the height of the box.
    pub fn half_height(&self) -> F {
        self.half_height
    }

    /// Returns half the depth of the box.
    pub fn half_depth(&self) -> F {
        self.half_depth
    }

    /// Returns the width of the box.
    pub fn width(&self) -> F {
        F::TWO * self.half_width
    }

    /// Returns the height of the box.
    pub fn height(&self) -> F {
        F::TWO * self.half_height
    }

    /// Returns the depth of the box.
    pub fn depth(&self) -> F {
        F::TWO * self.half_depth
    }

    /// Computes the unit vector representing the width axis of the box.
    pub fn compute_width_axis(&self) -> UnitVector3<F> {
        UnitVector3::new_unchecked(self.orientation.transform_vector(&Vector3::x_axis()))
    }

    /// Computes the unit vector representing the height axis of the box.
    pub fn compute_height_axis(&self) -> UnitVector3<F> {
        UnitVector3::new_unchecked(self.orientation.transform_vector(&Vector3::y_axis()))
    }

    /// Computes the unit vector representing the depth axis of the box.
    pub fn compute_depth_axis(&self) -> UnitVector3<F> {
        UnitVector3::new_unchecked(self.orientation.transform_vector(&Vector3::z_axis()))
    }

    /// Creates a new box corresponding to transforming this box with the given
    /// similarity transform.
    pub fn transformed(&self, transform: &Similarity3<F>) -> Self {
        Self::new(
            transform.transform_point(&self.center),
            transform.isometry.rotation * self.orientation,
            transform.scaling() * self.half_width,
            transform.scaling() * self.half_height,
            transform.scaling() * self.half_depth,
        )
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use approx::assert_abs_diff_eq;
    use std::f64::consts::PI;

    #[test]
    fn oriented_box_axes_are_correct() {
        let oriented_box = OrientedBox::new(
            Point3::origin(),
            UnitQuaternion::from_axis_angle(&Vector3::x_axis(), PI / 2.0),
            1.0,
            1.0,
            1.0,
        );
        assert_abs_diff_eq!(oriented_box.compute_width_axis(), Vector3::x_axis());
        assert_abs_diff_eq!(oriented_box.compute_height_axis(), Vector3::z_axis());
        assert_abs_diff_eq!(oriented_box.compute_depth_axis(), -Vector3::y_axis());

        let oriented_box = OrientedBox::new(
            Point3::origin(),
            UnitQuaternion::from_axis_angle(&Vector3::y_axis(), -PI / 2.0),
            1.0,
            1.0,
            1.0,
        );
        assert_abs_diff_eq!(oriented_box.compute_width_axis(), Vector3::z_axis());
        assert_abs_diff_eq!(oriented_box.compute_height_axis(), Vector3::y_axis());
        assert_abs_diff_eq!(oriented_box.compute_depth_axis(), -Vector3::x_axis());
    }
}
