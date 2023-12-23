//! Representation of axis-aligned boxes.

use super::Point;
use crate::num::Float;
use na::point;
use nalgebra::{self as na, Point3};

use Corner::{Lower, Upper};

/// A box with orientation aligned with the coordinate system axes. The width,
/// height and depth axes are aligned with the x-, y- and z-axis respectively.
#[derive(Clone, Debug, PartialEq)]
pub struct AxisAlignedBox<F: Float> {
    corners: [Point3<F>; 2],
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Corner {
    Lower = 0,
    Upper = 1,
}

const ALL_CORNER_COMPONENTS: [[Corner; 3]; 8] = [
    [Lower, Lower, Lower],
    [Lower, Lower, Upper],
    [Lower, Upper, Lower],
    [Lower, Upper, Upper],
    [Upper, Lower, Lower],
    [Upper, Lower, Upper],
    [Upper, Upper, Lower],
    [Upper, Upper, Upper],
];

const OPPOSITE_CORNER_INDICES: [usize; 8] = [7, 6, 5, 4, 3, 2, 1, 0];

impl<F: Float> AxisAlignedBox<F> {
    /// Creates a new box with the given lower and upper corner points.
    pub fn new(lower_corner: Point3<F>, upper_corner: Point3<F>) -> Self {
        Self {
            corners: [lower_corner, upper_corner],
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

    /// Creates the axis-aligned box bounding both the given axis-aligned boxes.
    pub fn aabb_from_pair(aabb_1: &Self, aabb_2: &Self) -> Self {
        Self::new(
            aabb_1.lower_corner().inf(aabb_2.lower_corner()),
            aabb_1.upper_corner().sup(aabb_2.upper_corner()),
        )
    }

    /// Returns a reference to the lower corner of the box.
    pub fn lower_corner(&self) -> &Point3<F> {
        &self.corners[0]
    }

    /// Returns a reference to the upper corner of the box.
    pub fn upper_corner(&self) -> &Point3<F> {
        &self.corners[1]
    }

    /// Calculates and returns the center point of the box.
    pub fn center(&self) -> Point3<F> {
        na::center(self.lower_corner(), self.upper_corner())
    }

    /// Returns the extent of the box along the x-axis (the width).
    pub fn extent_x(&self) -> F {
        self.upper_corner().x - self.lower_corner().x
    }

    /// Returns the extent of the box along the y-axis (the height).
    pub fn extent_y(&self) -> F {
        self.upper_corner().y - self.lower_corner().y
    }

    /// Returns the extent of the box along the z-axis (the depth).
    pub fn extent_z(&self) -> F {
        self.upper_corner().z - self.lower_corner().z
    }

    /// Returns the box corner with the given index. The corners are ordered
    /// from smaller to larger coordinates, with the z-component varying
    /// fastest.
    ///
    /// # Panics
    /// If the given index exceeds 7.
    pub fn corner(&self, corner_idx: usize) -> Point3<F> {
        let corner_components = &ALL_CORNER_COMPONENTS[corner_idx];
        point![
            self.corners[corner_components[0] as usize].x,
            self.corners[corner_components[1] as usize].y,
            self.corners[corner_components[2] as usize].z
        ]
    }

    /// Returns the box corner opposite to the corner with the given index. The
    /// corners are ordered from smaller to larger coordinates, with the
    /// z-component varying fastest.
    ///
    /// # Panics
    /// If the given index exceeds 7.
    pub fn opposite_corner(&self, corner_idx: usize) -> Point3<F> {
        self.corner(OPPOSITE_CORNER_INDICES[corner_idx])
    }

    /// Whether all of the given axis-aligned box is outside this box. If the
    /// boundaries exactly touch each other, the box is considered inside.
    pub fn box_lies_outside(&self, other: &Self) -> bool {
        !((self.lower_corner().x <= other.upper_corner().x
            && self.upper_corner().x >= other.lower_corner().x)
            && (self.lower_corner().y <= other.upper_corner().y
                && self.upper_corner().y >= other.lower_corner().y)
            && (self.lower_corner().z <= other.upper_corner().z
                && self.upper_corner().z >= other.lower_corner().z))
    }

    /// Computes the corner of the axis aligned box that is closest to the given
    /// point.
    pub fn compute_closest_corner(&self, point: &Point3<F>) -> Point3<F> {
        let mut closest_corner = Point3::origin();
        for dim in 0..3 {
            if (self.lower_corner()[dim] - point[dim]).abs()
                < (self.upper_corner()[dim] - point[dim]).abs()
            {
                closest_corner[dim] = self.lower_corner()[dim];
            } else {
                closest_corner[dim] = self.upper_corner()[dim];
            }
        }
        closest_corner
    }

    /// Computes the corner of the axis aligned box that is farthest from the
    /// given point.
    pub fn compute_farthest_corner(&self, point: &Point3<F>) -> Point3<F> {
        let mut farthest_corner = Point3::origin();
        for dim in 0..3 {
            if (self.lower_corner()[dim] - point[dim]).abs()
                > (self.upper_corner()[dim] - point[dim]).abs()
            {
                farthest_corner[dim] = self.lower_corner()[dim];
            } else {
                farthest_corner[dim] = self.upper_corner()[dim];
            }
        }
        farthest_corner
    }

    /// Computes the axis-aligned bounding box enclosing only the volume
    /// enclosed by both this and the given bounding box, or [`None`] if the two
    /// boxes do not overlap.
    pub fn union_with(&self, other: &Self) -> Option<Self> {
        let lower_corner = self.lower_corner().sup(other.lower_corner());
        let upper_corner = self.upper_corner().inf(other.upper_corner());

        if (upper_corner - lower_corner)
            .iter()
            .any(|&diff| diff < F::ZERO)
        {
            None
        } else {
            Some(Self::new(lower_corner, upper_corner))
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use approx::assert_abs_diff_eq;
    use nalgebra::point;

    #[test]
    fn box_lies_outside_with_non_overlapping_boxes_works() {
        let aabb1 = AxisAlignedBox::new(point![0.0, 0.0, 0.0], point![1.0, 1.0, 1.0]);
        let aabb2 = AxisAlignedBox::new(point![2.0, 2.0, 2.0], point![3.0, 3.0, 3.0]);
        assert!(aabb1.box_lies_outside(&aabb2));
    }

    #[test]
    fn box_lies_outside_with_touching_boxes_works() {
        let aabb1 = AxisAlignedBox::new(point![0.0, 0.0, 0.0], point![1.0, 1.0, 1.0]);
        let aabb2 = AxisAlignedBox::new(point![1.0, 1.0, 1.0], point![2.0, 2.0, 2.0]);
        assert!(!aabb1.box_lies_outside(&aabb2));
    }

    #[test]
    fn box_lies_outside_with_overlapping_boxes_works() {
        let aabb1 = AxisAlignedBox::new(point![0.0, 0.0, 0.0], point![2.0, 2.0, 2.0]);
        let aabb2 = AxisAlignedBox::new(point![1.0, 1.0, 1.0], point![3.0, 3.0, 3.0]);
        assert!(!aabb1.box_lies_outside(&aabb2));
    }

    #[test]
    fn box_lies_outside_with_equal_boxes_works() {
        let aabb1 = AxisAlignedBox::new(point![0.0, 0.0, 0.0], point![1.0, 1.0, 1.0]);
        let aabb2 = AxisAlignedBox::new(point![0.0, 0.0, 0.0], point![1.0, 1.0, 1.0]);
        assert!(!aabb1.box_lies_outside(&aabb2));
    }

    #[test]
    fn box_lies_outside_with_nested_boxes_works() {
        let aabb1 = AxisAlignedBox::new(point![0.0, 0.0, 0.0], point![2.0, 2.0, 2.0]);
        let aabb2 = AxisAlignedBox::new(point![0.5, 0.5, 0.5], point![1.5, 1.5, 1.5]);
        assert!(!aabb1.box_lies_outside(&aabb2));
    }

    #[test]
    fn compute_closest_corner_with_point_inside_box_works() {
        let aabb = AxisAlignedBox::new(point![0.0, 0.0, 0.0], point![1.0, 1.0, 1.0]);
        assert_abs_diff_eq!(
            aabb.compute_closest_corner(&point![0.6, 0.6, 0.6]),
            point![1.0, 1.0, 1.0]
        );
    }

    #[test]
    fn compute_closest_corner_with_point_outside_box_works() {
        let aabb = AxisAlignedBox::new(point![0.0, 0.0, 0.0], point![1.0, 1.0, 1.0]);
        assert_abs_diff_eq!(
            aabb.compute_closest_corner(&point![2.0, 2.0, 2.0]),
            point![1.0, 1.0, 1.0]
        );
    }

    #[test]
    fn compute_closest_corner_with_point_on_box_corner_works() {
        let aabb = AxisAlignedBox::new(point![0.0, 0.0, 0.0], point![1.0, 1.0, 1.0]);
        assert_abs_diff_eq!(
            aabb.compute_closest_corner(&point![1.0, 1.0, 1.0]),
            point![1.0, 1.0, 1.0]
        );
    }

    #[test]
    fn compute_closest_corner_with_point_on_box_edge_works() {
        let aabb = AxisAlignedBox::new(point![0.0, 0.0, 0.0], point![1.0, 1.0, 1.0]);
        assert_abs_diff_eq!(
            aabb.compute_closest_corner(&point![0.0, 0.4, 0.4]),
            point![0.0, 0.0, 0.0]
        );
    }

    #[test]
    fn compute_farthest_corner_with_point_inside_box_works() {
        let aabb = AxisAlignedBox::new(point![0.0, 0.0, 0.0], point![1.0, 1.0, 1.0]);
        assert_abs_diff_eq!(
            aabb.compute_farthest_corner(&point![0.6, 0.6, 0.6]),
            point![0.0, 0.0, 0.0]
        );
    }

    #[test]
    fn compute_farthest_corner_with_point_outside_box_works() {
        let aabb = AxisAlignedBox::new(point![0.0, 0.0, 0.0], point![1.0, 1.0, 1.0]);
        assert_abs_diff_eq!(
            aabb.compute_farthest_corner(&point![2.0, 2.0, 2.0]),
            point![0.0, 0.0, 0.0]
        );
    }

    #[test]
    fn compute_farthest_corner_with_point_on_box_corner_works() {
        let aabb = AxisAlignedBox::new(point![0.0, 0.0, 0.0], point![1.0, 1.0, 1.0]);
        assert_abs_diff_eq!(
            aabb.compute_farthest_corner(&point![1.0, 1.0, 1.0]),
            point![0.0, 0.0, 0.0]
        );
    }

    #[test]
    fn compute_farthest_corner_with_point_on_box_edge_works() {
        let aabb = AxisAlignedBox::new(point![0.0, 0.0, 0.0], point![1.0, 1.0, 1.0]);
        assert_abs_diff_eq!(
            aabb.compute_farthest_corner(&point![0.0, 0.4, 0.4]),
            point![1.0, 1.0, 1.0]
        );
    }

    #[test]
    fn should_get_correct_corners() {
        let lower = point![-1.0, 2.0, -3.0];
        let upper = point![3.0, -2.0, 1.0];
        let aabb = AxisAlignedBox::new(lower, upper);
        assert_abs_diff_eq!(aabb.corner(0), lower);
        assert_abs_diff_eq!(aabb.corner(1), point![lower.x, lower.y, upper.z]);
        assert_abs_diff_eq!(aabb.corner(2), point![lower.x, upper.y, lower.z]);
        assert_abs_diff_eq!(aabb.corner(3), point![lower.x, upper.y, upper.z]);
        assert_abs_diff_eq!(aabb.corner(4), point![upper.x, lower.y, lower.z]);
        assert_abs_diff_eq!(aabb.corner(5), point![upper.x, lower.y, upper.z]);
        assert_abs_diff_eq!(aabb.corner(6), point![upper.x, upper.y, lower.z]);
        assert_abs_diff_eq!(aabb.corner(7), upper);
    }

    #[test]
    fn should_get_correct_opposite_corners() {
        let lower = point![-1.0, 2.0, -3.0];
        let upper = point![3.0, -2.0, 1.0];
        let aabb = AxisAlignedBox::new(lower, upper);
        assert_abs_diff_eq!(aabb.opposite_corner(7), lower);
        assert_abs_diff_eq!(aabb.opposite_corner(6), point![lower.x, lower.y, upper.z]);
        assert_abs_diff_eq!(aabb.opposite_corner(5), point![lower.x, upper.y, lower.z]);
        assert_abs_diff_eq!(aabb.opposite_corner(4), point![lower.x, upper.y, upper.z]);
        assert_abs_diff_eq!(aabb.opposite_corner(3), point![upper.x, lower.y, lower.z]);
        assert_abs_diff_eq!(aabb.opposite_corner(2), point![upper.x, lower.y, upper.z]);
        assert_abs_diff_eq!(aabb.opposite_corner(1), point![upper.x, upper.y, lower.z]);
        assert_abs_diff_eq!(aabb.opposite_corner(0), upper);
    }
}
