//! Representation of axis-aligned boxes.

use super::Point;
use crate::num::Float;
use nalgebra::{self as na, Point3};

/// A box with orientation aligned with the coordinate system axes. The width,
/// height and depth axes are aligned with the x-, y- and z-axis respectively.
#[derive(Clone, Debug, PartialEq)]
pub struct AxisAlignedBox<F: Float> {
    lower_corner: Point3<F>,
    upper_corner: Point3<F>,
}

impl<F: Float> AxisAlignedBox<F> {
    /// Creates a new box with the given lower and upper corner points.
    pub fn new(lower_corner: Point3<F>, upper_corner: Point3<F>) -> Self {
        Self {
            lower_corner,
            upper_corner,
        }
    }

    /// Creates the axis-aligned bounding box for the set of points in the given
    /// slice.
    ///
    /// # Panics
    /// If the point slice is empty.
    pub fn aabb_for_points(points: &[impl Point<F>]) -> Self {
        assert!(
            !points.is_empty(),
            "Tried to create AABB for empty point slice"
        );

        let first_point = *points[0].point();

        let lower_corner = points
            .iter()
            .skip(1)
            .fold(first_point, |lower_corner, point| {
                lower_corner.inf(point.point())
            });

        let upper_corner = points
            .iter()
            .skip(1)
            .fold(first_point, |upper_corner, point| {
                upper_corner.sup(point.point())
            });

        Self::new(lower_corner, upper_corner)
    }

    /// Creates the axis-aligned bounding box for the set of points in the given
    /// array.
    ///
    /// # Panics
    /// If the point array is empty.
    pub fn aabb_for_point_array<const N: usize>(points: &[impl Point<F>; N]) -> Self {
        assert!(N > 0, "Tried to create AABB for empty point array");

        let first_point = *points[0].point();

        let lower_corner = points
            .iter()
            .skip(1)
            .fold(first_point, |lower_corner, point| {
                lower_corner.inf(point.point())
            });

        let upper_corner = points
            .iter()
            .skip(1)
            .fold(first_point, |upper_corner, point| {
                upper_corner.sup(point.point())
            });

        Self::new(lower_corner, upper_corner)
    }

    /// Returns a reference to the lower corner of the box.
    pub fn lower_corner(&self) -> &Point3<F> {
        &self.lower_corner
    }

    /// Returns a reference to the upper corner of the box.
    pub fn upper_corner(&self) -> &Point3<F> {
        &self.upper_corner
    }

    /// Calculates and returns the center point of the box.
    pub fn center(&self) -> Point3<F> {
        na::center(&self.lower_corner, &self.upper_corner)
    }

    /// Returns the extent of the box along the x-axis (the width).
    pub fn extent_x(&self) -> F {
        self.upper_corner.x - self.lower_corner.x
    }

    /// Returns the extent of the box along the y-axis (the height).
    pub fn extent_y(&self) -> F {
        self.upper_corner.y - self.lower_corner.y
    }

    /// Returns the extent of the box along the z-axis (the depth).
    pub fn extent_z(&self) -> F {
        self.upper_corner.z - self.lower_corner.z
    }
}
