//! Representation of boxes with arbitrary orientations.

use crate::{AxisAlignedBox, Plane};
use impact_math::Float;
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

    /// Creates a new box with the given half extents, centered at the origin
    /// and with the width, height and depth axes aligned with the x-, y-
    /// and z-axis respectively.
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

    /// Computes the eight corners of the oriented box.
    pub fn compute_corners(&self) -> [Point3<F>; 8] {
        let half_width_vector = self.compute_width_axis().scale(self.half_width);
        let half_height_vector = self.compute_height_axis().scale(self.half_height);
        let half_depth_vector = self.compute_depth_axis().scale(self.half_depth);
        [
            self.center - half_width_vector - half_height_vector - half_depth_vector,
            self.center - half_width_vector - half_height_vector + half_depth_vector,
            self.center - half_width_vector + half_height_vector - half_depth_vector,
            self.center - half_width_vector + half_height_vector + half_depth_vector,
            self.center + half_width_vector - half_height_vector - half_depth_vector,
            self.center + half_width_vector - half_height_vector + half_depth_vector,
            self.center + half_width_vector + half_height_vector - half_depth_vector,
            self.center + half_width_vector + half_height_vector + half_depth_vector,
        ]
    }

    /// Computes the six planes that bound the oriented box.
    ///
    /// The order and orientation of the planes are consistent with the planes
    /// in a [`Frustum`](crate::Frustum).
    pub fn compute_bounding_planes(&self) -> [Plane<F>; 6] {
        let width_axis = self.compute_width_axis();
        let height_axis = self.compute_height_axis();
        let depth_axis = self.compute_depth_axis();
        let width_of_center = width_axis.dot(&self.center.coords);
        let height_of_center = height_axis.dot(&self.center.coords);
        let depth_of_center = depth_axis.dot(&self.center.coords);
        [
            Plane::new(width_axis, width_of_center - self.half_width),
            Plane::new(-width_axis, -width_of_center - self.half_width),
            Plane::new(height_axis, height_of_center - self.half_height),
            Plane::new(-height_axis, -height_of_center - self.half_height),
            Plane::new(depth_axis, depth_of_center - self.half_depth),
            Plane::new(-depth_axis, -depth_of_center - self.half_depth),
        ]
    }
}

#[cfg(test)]
mod tests {
    use crate::{Frustum, OrthographicTransform};

    use super::*;
    use approx::assert_abs_diff_eq;
    use nalgebra::point;
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

    #[test]
    fn oriented_box_bounding_planes_are_consistent_with_orthographic_frustum_planes() {
        let frustum = Frustum::from_transform(
            OrthographicTransform::new(-1.0, 2.0, -3.0, 4.0, -5.0, 6.0).as_projective(),
        );
        let oriented_box = OrientedBox::from_axis_aligned_box(&AxisAlignedBox::new(
            point![-1.0, -3.0, -5.0],
            point![2.0, 4.0, 6.0],
        ));
        for (frustum_plane, oriented_box_plane) in frustum
            .planes()
            .iter()
            .zip(oriented_box.compute_bounding_planes())
        {
            assert_abs_diff_eq!(oriented_box_plane, frustum_plane, epsilon = 1e-8);
        }
    }
}
