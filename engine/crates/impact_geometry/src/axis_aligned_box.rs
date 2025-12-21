//! Representation of axis-aligned boxes.

use crate::Plane;
use Corner::{Lower, Upper};
use approx::AbsDiffEq;
use impact_math::{
    matrix::Matrix4,
    point::Point3,
    vector::{UnitVector3, Vector3},
};

/// A box with orientation aligned with the coordinate system axes. The width,
/// height and depth axes are aligned with the x-, y- and z-axis respectively.
#[derive(Clone, Debug, PartialEq)]
pub struct AxisAlignedBox {
    corners: [Point3; 2],
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

impl AxisAlignedBox {
    /// Creates a new box with the given lower and upper corner points.
    pub fn new(lower_corner: Point3, upper_corner: Point3) -> Self {
        Self {
            corners: [lower_corner, upper_corner],
        }
    }

    /// Creates the axis-aligned bounding box for the set of points in the given
    /// slice.
    ///
    /// # Panics
    /// If the point slice is empty.
    pub fn aabb_for_points(points: &[Point3]) -> Self {
        assert!(
            !points.is_empty(),
            "Tried to create AABB for empty point slice"
        );

        let first_point = points[0];

        let lower_corner = points
            .iter()
            .skip(1)
            .fold(first_point, |lower_corner, point| {
                lower_corner.min_with(point)
            });

        let upper_corner = points
            .iter()
            .skip(1)
            .fold(first_point, |upper_corner, point| {
                upper_corner.max_with(point)
            });

        Self::new(lower_corner, upper_corner)
    }

    /// Creates the axis-aligned bounding box for the set of points in the given
    /// array.
    ///
    /// # Panics
    /// If the point array is empty.
    pub fn aabb_for_point_array<const N: usize>(points: &[Point3; N]) -> Self {
        assert!(N > 0, "Tried to create AABB for empty point array");

        let first_point = points[0];

        let lower_corner = points
            .iter()
            .skip(1)
            .fold(first_point, |lower_corner, point| {
                lower_corner.min_with(point)
            });

        let upper_corner = points
            .iter()
            .skip(1)
            .fold(first_point, |upper_corner, point| {
                upper_corner.max_with(point)
            });

        Self::new(lower_corner, upper_corner)
    }

    /// Creates the axis-aligned box bounding both the given axis-aligned boxes.
    pub fn aabb_from_pair(aabb_1: &Self, aabb_2: &Self) -> Self {
        Self::new(
            aabb_1.lower_corner().min_with(aabb_2.lower_corner()),
            aabb_1.upper_corner().max_with(aabb_2.upper_corner()),
        )
    }

    /// Returns a reference to the lower corner of the box.
    pub fn lower_corner(&self) -> &Point3 {
        &self.corners[0]
    }

    /// Returns a reference to the upper corner of the box.
    pub fn upper_corner(&self) -> &Point3 {
        &self.corners[1]
    }

    /// Calculates and returns the center point of the box.
    pub fn center(&self) -> Point3 {
        Point3::center_of(self.lower_corner(), self.upper_corner())
    }

    /// Returns the extents of the box along the three axes.
    pub fn extents(&self) -> Vector3 {
        self.upper_corner() - self.lower_corner()
    }

    /// Returns the half extents of the box along the three axes.
    pub fn half_extents(&self) -> Vector3 {
        0.5 * self.extents()
    }

    /// Returns an array with all the eight corners of the box. The corners are
    /// ordered from smaller to larger coordinates, with the z-component varying
    /// fastest.
    pub fn all_corners(&self) -> [Point3; 8] {
        [0, 1, 2, 3, 4, 5, 6, 7].map(|idx| self.corner(idx))
    }

    /// Returns the box corner with the given index. The corners are ordered
    /// from smaller to larger coordinates, with the z-component varying
    /// fastest.
    ///
    /// # Panics
    /// If the given index exceeds 7.
    pub fn corner(&self, corner_idx: usize) -> Point3 {
        let corner_components = &ALL_CORNER_COMPONENTS[corner_idx];
        Point3::new(
            self.corners[corner_components[0] as usize].x(),
            self.corners[corner_components[1] as usize].y(),
            self.corners[corner_components[2] as usize].z(),
        )
    }

    /// Returns the box corner opposite to the corner with the given index. The
    /// corners are ordered from smaller to larger coordinates, with the
    /// z-component varying fastest.
    ///
    /// # Panics
    /// If the given index exceeds 7.
    pub fn opposite_corner(&self, corner_idx: usize) -> Point3 {
        self.corner(OPPOSITE_CORNER_INDICES[corner_idx])
    }

    /// Whether the given point is inside this axis-aligned box. A point exactly on the
    /// surface of the box is considered inside.
    pub fn contains_point(&self, point: &Point3) -> bool {
        point.x() >= self.lower_corner().x()
            && point.x() <= self.upper_corner().x()
            && point.y() >= self.lower_corner().y()
            && point.y() <= self.upper_corner().y()
            && point.z() >= self.lower_corner().z()
            && point.z() <= self.upper_corner().z()
    }

    /// Whether all of the given axis-aligned box is inside this box. If a
    /// corner exactly touches the surface, it is still considered inside.
    pub fn contains_box(&self, other: &Self) -> bool {
        other.lower_corner().x() >= self.lower_corner().x()
            && other.upper_corner().x() <= self.upper_corner().x()
            && other.lower_corner().y() >= self.lower_corner().y()
            && other.upper_corner().y() <= self.upper_corner().y()
            && other.lower_corner().z() >= self.lower_corner().z()
            && other.upper_corner().z() <= self.upper_corner().z()
    }

    /// Whether all of the given axis-aligned box is outside this box. If the
    /// boundaries exactly touch each other, the box is considered inside.
    pub fn box_lies_outside(&self, other: &Self) -> bool {
        !((self.lower_corner().x() <= other.upper_corner().x()
            && self.upper_corner().x() >= other.lower_corner().x())
            && (self.lower_corner().y() <= other.upper_corner().y()
                && self.upper_corner().y() >= other.lower_corner().y())
            && (self.lower_corner().z() <= other.upper_corner().z()
                && self.upper_corner().z() >= other.lower_corner().z()))
    }

    /// Computes the corner of the axis aligned box that is closest to the given
    /// point.
    pub fn compute_closest_corner(&self, point: &Point3) -> Point3 {
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
    pub fn compute_farthest_corner(&self, point: &Point3) -> Point3 {
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
    pub fn compute_overlap_with(&self, other: &Self) -> Option<Self> {
        let lower_corner = self.lower_corner().max_with(other.lower_corner());
        let upper_corner = self.upper_corner().min_with(other.upper_corner());
        let diff = upper_corner - lower_corner;

        if diff.x() < 0.0 || diff.y() < 0.0 || diff.z() < 0.0 {
            None
        } else {
            Some(Self::new(lower_corner, upper_corner))
        }
    }

    /// Computes the axis-aligned box resulting from scaling this box with the
    /// given uniform scale factor.
    pub fn scaled(&self, scale: f32) -> Self {
        Self::new(scale * self.lower_corner(), scale * self.upper_corner())
    }

    /// Computes the axis-aligned box resulting from scaling the extents of this
    /// box relative to its center with the given uniform scale factor.
    pub fn scaled_about_center(&self, scale: f32) -> Self {
        let center = self.center();
        let scaled_half_extents = (0.5 * scale.abs()) * self.extents();
        Self::new(center - scaled_half_extents, center + scaled_half_extents)
    }

    /// Computes the axis-aligned box resulting from expanding the extents of
    /// this box relative to its center by the given margin on each side.
    pub fn expanded_about_center(&self, margin: f32) -> Self {
        let margin = Vector3::same(margin);
        Self::new(self.lower_corner() - margin, self.upper_corner() + margin)
    }

    /// Computes the axis-aligned box resulting from translating this box with
    /// the given displacement vector.
    pub fn translated(&self, displacement: &Vector3) -> Self {
        Self::new(
            self.lower_corner() + displacement,
            self.upper_corner() + displacement,
        )
    }

    /// Computes the AABB for the transformed version of this AABB.
    pub fn aabb_of_transformed(&self, homogeneous_transform: &Matrix4) -> Self {
        let transformed_center = homogeneous_transform.transform_point(&self.center());

        // Performance trick: transform half-extents by the element-wise
        // absolute value of the linear 3x3 part
        let rotation_scale = homogeneous_transform.linear_part();
        let abs_rotation_scale = rotation_scale.mapped(f32::abs);
        let transformed_half_extents = abs_rotation_scale * self.half_extents();

        Self::new(
            transformed_center - transformed_half_extents,
            transformed_center + transformed_half_extents,
        )
    }

    /// Given a line segment defined by a start point and an offset to the end
    /// point, finds the start and end segment parameter representing the
    /// subsegment lying within the box, or returns [`None`] if the segment lies
    /// completely outside the box.
    pub fn find_contained_subsegment(
        &self,
        segment_start: &Point3,
        offset_from_segment_start_to_end: &Vector3,
    ) -> Option<(f32, f32)> {
        let mut t_min: f32 = 0.0;
        let mut t_max: f32 = 1.0;

        for dim in 0..3 {
            if offset_from_segment_start_to_end[dim] != 0.0 {
                let recip = offset_from_segment_start_to_end[dim].recip();
                let t1 = (self.lower_corner()[dim] - segment_start[dim]) * recip;
                let t2 = (self.upper_corner()[dim] - segment_start[dim]) * recip;

                let (t_entry, t_exit) = if t1 < t2 { (t1, t2) } else { (t2, t1) };

                t_min = t_min.max(t_entry);
                t_max = t_max.min(t_exit);
            } else if segment_start[dim] < self.lower_corner()[dim]
                || segment_start[dim] > self.upper_corner()[dim]
            {
                return None;
            }
        }

        if t_min <= t_max {
            Some((t_min, t_max))
        } else {
            None
        }
    }

    /// Given a ray defined by an origin and direction, finds the distances
    /// along the ray at which the ray enters and exits the box, or returns
    /// [`None`] if the ray does not hit the box.
    pub fn find_ray_intersection(
        &self,
        ray_origin: &Point3,
        ray_direction: &UnitVector3,
    ) -> Option<(f32, f32)> {
        let mut t_min: f32 = 0.0;
        let mut t_max: f32 = f32::INFINITY;

        for dim in 0..3 {
            if ray_direction[dim] != 0.0 {
                let recip = ray_direction[dim].recip();
                let t1 = (self.lower_corner()[dim] - ray_origin[dim]) * recip;
                let t2 = (self.upper_corner()[dim] - ray_origin[dim]) * recip;

                let (t_entry, t_exit) = if t1 < t2 { (t1, t2) } else { (t2, t1) };

                t_min = t_min.max(t_entry);
                t_max = t_max.min(t_exit);

                if t_max < t_min {
                    return None;
                }
            } else if ray_origin[dim] < self.lower_corner()[dim]
                || ray_origin[dim] > self.upper_corner()[dim]
            {
                return None;
            }
        }

        // Require intersection in the forward ray direction
        if t_max >= 0.0 {
            Some((t_min.max(0.0), t_max))
        } else {
            None
        }
    }

    /// Returns a version of this AAB that extrudes as little as possible
    /// into the positive halfspace of the given plane without changing the
    /// volume of the box lying within the negative halfspace.
    pub fn projected_onto_negative_halfspace(&self, plane: &Plane) -> Self {
        const TOLERANCE: f32 = 1e-8;
        let normal = plane.unit_normal();

        let mut fitted = self.clone();

        for (i, j, k) in [(0, 1, 2), (1, 2, 0), (2, 0, 1)] {
            if normal[k].abs() > TOLERANCE {
                let a = normal[i] * self.corners[0][i] + normal[j] * self.corners[0][j];
                let b = normal[i] * self.corners[0][i] + normal[j] * self.corners[1][j];
                let c = normal[i] * self.corners[1][i] + normal[j] * self.corners[0][j];
                let d = normal[i] * self.corners[1][i] + normal[j] * self.corners[1][j];

                let extremal = (plane.displacement() - a.min(b).min(c).min(d)) / normal[k];

                if normal[k].is_sign_positive() {
                    fitted.corners[0][k] = fitted.corners[0][k].min(extremal);
                    fitted.corners[1][k] = fitted.corners[1][k].min(extremal);
                } else {
                    fitted.corners[0][k] = fitted.corners[0][k].max(extremal);
                    fitted.corners[1][k] = fitted.corners[1][k].max(extremal);
                }
            }
        }
        fitted
    }
}

impl AbsDiffEq for AxisAlignedBox {
    type Epsilon = f32;

    fn default_epsilon() -> Self::Epsilon {
        f32::default_epsilon()
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        Point3::abs_diff_eq(self.lower_corner(), other.lower_corner(), epsilon)
            && Point3::abs_diff_eq(self.upper_corner(), other.upper_corner(), epsilon)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_abs_diff_eq;

    #[test]
    fn box_lies_outside_with_non_overlapping_boxes_works() {
        let aabb1 = AxisAlignedBox::new(Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 1.0, 1.0));
        let aabb2 = AxisAlignedBox::new(Point3::new(2.0, 2.0, 2.0), Point3::new(3.0, 3.0, 3.0));
        assert!(aabb1.box_lies_outside(&aabb2));
    }

    #[test]
    fn box_lies_outside_with_touching_boxes_works() {
        let aabb1 = AxisAlignedBox::new(Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 1.0, 1.0));
        let aabb2 = AxisAlignedBox::new(Point3::new(1.0, 1.0, 1.0), Point3::new(2.0, 2.0, 2.0));
        assert!(!aabb1.box_lies_outside(&aabb2));
    }

    #[test]
    fn box_lies_outside_with_overlapping_boxes_works() {
        let aabb1 = AxisAlignedBox::new(Point3::new(0.0, 0.0, 0.0), Point3::new(2.0, 2.0, 2.0));
        let aabb2 = AxisAlignedBox::new(Point3::new(1.0, 1.0, 1.0), Point3::new(3.0, 3.0, 3.0));
        assert!(!aabb1.box_lies_outside(&aabb2));
    }

    #[test]
    fn box_lies_outside_with_equal_boxes_works() {
        let aabb1 = AxisAlignedBox::new(Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 1.0, 1.0));
        let aabb2 = AxisAlignedBox::new(Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 1.0, 1.0));
        assert!(!aabb1.box_lies_outside(&aabb2));
    }

    #[test]
    fn box_lies_outside_with_nested_boxes_works() {
        let aabb1 = AxisAlignedBox::new(Point3::new(0.0, 0.0, 0.0), Point3::new(2.0, 2.0, 2.0));
        let aabb2 = AxisAlignedBox::new(Point3::new(0.5, 0.5, 0.5), Point3::new(1.5, 1.5, 1.5));
        assert!(!aabb1.box_lies_outside(&aabb2));
    }

    #[test]
    fn compute_closest_corner_with_point_inside_box_works() {
        let aabb = AxisAlignedBox::new(Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 1.0, 1.0));
        assert_abs_diff_eq!(
            aabb.compute_closest_corner(&Point3::new(0.6, 0.6, 0.6)),
            Point3::new(1.0, 1.0, 1.0)
        );
    }

    #[test]
    fn compute_closest_corner_with_point_outside_box_works() {
        let aabb = AxisAlignedBox::new(Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 1.0, 1.0));
        assert_abs_diff_eq!(
            aabb.compute_closest_corner(&Point3::new(2.0, 2.0, 2.0)),
            Point3::new(1.0, 1.0, 1.0)
        );
    }

    #[test]
    fn compute_closest_corner_with_point_on_box_corner_works() {
        let aabb = AxisAlignedBox::new(Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 1.0, 1.0));
        assert_abs_diff_eq!(
            aabb.compute_closest_corner(&Point3::new(1.0, 1.0, 1.0)),
            Point3::new(1.0, 1.0, 1.0)
        );
    }

    #[test]
    fn compute_closest_corner_with_point_on_box_edge_works() {
        let aabb = AxisAlignedBox::new(Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 1.0, 1.0));
        assert_abs_diff_eq!(
            aabb.compute_closest_corner(&Point3::new(0.0, 0.4, 0.4)),
            Point3::new(0.0, 0.0, 0.0)
        );
    }

    #[test]
    fn compute_farthest_corner_with_point_inside_box_works() {
        let aabb = AxisAlignedBox::new(Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 1.0, 1.0));
        assert_abs_diff_eq!(
            aabb.compute_farthest_corner(&Point3::new(0.6, 0.6, 0.6)),
            Point3::new(0.0, 0.0, 0.0)
        );
    }

    #[test]
    fn compute_farthest_corner_with_point_outside_box_works() {
        let aabb = AxisAlignedBox::new(Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 1.0, 1.0));
        assert_abs_diff_eq!(
            aabb.compute_farthest_corner(&Point3::new(2.0, 2.0, 2.0)),
            Point3::new(0.0, 0.0, 0.0)
        );
    }

    #[test]
    fn compute_farthest_corner_with_point_on_box_corner_works() {
        let aabb = AxisAlignedBox::new(Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 1.0, 1.0));
        assert_abs_diff_eq!(
            aabb.compute_farthest_corner(&Point3::new(1.0, 1.0, 1.0)),
            Point3::new(0.0, 0.0, 0.0)
        );
    }

    #[test]
    fn compute_farthest_corner_with_point_on_box_edge_works() {
        let aabb = AxisAlignedBox::new(Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 1.0, 1.0));
        assert_abs_diff_eq!(
            aabb.compute_farthest_corner(&Point3::new(0.0, 0.4, 0.4)),
            Point3::new(1.0, 1.0, 1.0)
        );
    }

    #[test]
    fn should_get_correct_corners() {
        let lower = Point3::new(-1.0, 2.0, -3.0);
        let upper = Point3::new(3.0, -2.0, 1.0);
        let aabb = AxisAlignedBox::new(lower, upper);
        assert_abs_diff_eq!(aabb.corner(0), lower);
        assert_abs_diff_eq!(aabb.corner(1), Point3::new(lower.x(), lower.y(), upper.z()));
        assert_abs_diff_eq!(aabb.corner(2), Point3::new(lower.x(), upper.y(), lower.z()));
        assert_abs_diff_eq!(aabb.corner(3), Point3::new(lower.x(), upper.y(), upper.z()));
        assert_abs_diff_eq!(aabb.corner(4), Point3::new(upper.x(), lower.y(), lower.z()));
        assert_abs_diff_eq!(aabb.corner(5), Point3::new(upper.x(), lower.y(), upper.z()));
        assert_abs_diff_eq!(aabb.corner(6), Point3::new(upper.x(), upper.y(), lower.z()));
        assert_abs_diff_eq!(aabb.corner(7), upper);
    }

    #[test]
    fn should_get_correct_opposite_corners() {
        let lower = Point3::new(-1.0, 2.0, -3.0);
        let upper = Point3::new(3.0, -2.0, 1.0);
        let aabb = AxisAlignedBox::new(lower, upper);
        assert_abs_diff_eq!(aabb.opposite_corner(7), lower);
        assert_abs_diff_eq!(
            aabb.opposite_corner(6),
            Point3::new(lower.x(), lower.y(), upper.z())
        );
        assert_abs_diff_eq!(
            aabb.opposite_corner(5),
            Point3::new(lower.x(), upper.y(), lower.z())
        );
        assert_abs_diff_eq!(
            aabb.opposite_corner(4),
            Point3::new(lower.x(), upper.y(), upper.z())
        );
        assert_abs_diff_eq!(
            aabb.opposite_corner(3),
            Point3::new(upper.x(), lower.y(), lower.z())
        );
        assert_abs_diff_eq!(
            aabb.opposite_corner(2),
            Point3::new(upper.x(), lower.y(), upper.z())
        );
        assert_abs_diff_eq!(
            aabb.opposite_corner(1),
            Point3::new(upper.x(), upper.y(), lower.z())
        );
        assert_abs_diff_eq!(aabb.opposite_corner(0), upper);
    }

    #[test]
    fn projecting_onto_negative_halfspace_with_box_fully_in_negative_halfspace_unchanged() {
        let aabb = AxisAlignedBox::new(Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 1.0, 1.0));
        let plane =
            Plane::from_normal_and_point(UnitVector3::unit_z(), &Point3::new(0.0, 0.0, 2.0));
        let projected = aabb.projected_onto_negative_halfspace(&plane);
        assert_abs_diff_eq!(projected, aabb);
    }

    #[test]
    fn projecting_onto_negative_halfspace_with_box_fully_in_positive_halfspace_clips_to_plane() {
        let aabb = AxisAlignedBox::new(Point3::new(0.0, 0.0, 2.0), Point3::new(1.0, 1.0, 3.0));
        let plane =
            Plane::from_normal_and_point(UnitVector3::unit_z(), &Point3::new(0.0, 0.0, 1.0));
        let projected = aabb.projected_onto_negative_halfspace(&plane);
        let expected = AxisAlignedBox::new(Point3::new(0.0, 0.0, 1.0), Point3::new(1.0, 1.0, 1.0));
        assert_abs_diff_eq!(projected, expected);
    }

    #[test]
    fn projecting_onto_negative_halfspace_with_intersecting_box_clips_upper_part() {
        let aabb = AxisAlignedBox::new(Point3::new(0.0, 0.0, 0.0), Point3::new(2.0, 2.0, 2.0));
        let plane =
            Plane::from_normal_and_point(UnitVector3::unit_z(), &Point3::new(0.0, 0.0, 1.0));
        let projected = aabb.projected_onto_negative_halfspace(&plane);
        let expected = AxisAlignedBox::new(Point3::new(0.0, 0.0, 0.0), Point3::new(2.0, 2.0, 1.0));
        assert_abs_diff_eq!(projected, expected);
    }

    #[test]
    fn projecting_onto_negative_halfspace_with_plane_through_yz_works() {
        let aabb = AxisAlignedBox::new(Point3::new(-1.0, -1.0, -1.0), Point3::new(1.0, 1.0, 1.0));
        let plane =
            Plane::from_normal_and_point(UnitVector3::unit_x(), &Point3::new(0.5, 0.0, 0.0));
        let projected = aabb.projected_onto_negative_halfspace(&plane);
        let expected =
            AxisAlignedBox::new(Point3::new(-1.0, -1.0, -1.0), Point3::new(0.5, 1.0, 1.0));
        assert_abs_diff_eq!(projected, expected);
    }

    #[test]
    fn projecting_onto_negative_halfspace_with_plane_through_xz_works() {
        let aabb = AxisAlignedBox::new(Point3::new(-1.0, -1.0, -1.0), Point3::new(1.0, 1.0, 1.0));
        let plane =
            Plane::from_normal_and_point(UnitVector3::unit_y(), &Point3::new(0.0, 0.3, 0.0));
        let projected = aabb.projected_onto_negative_halfspace(&plane);
        let expected =
            AxisAlignedBox::new(Point3::new(-1.0, -1.0, -1.0), Point3::new(1.0, 0.3, 1.0));
        assert_abs_diff_eq!(projected, expected);
    }

    #[test]
    fn projecting_onto_negative_halfspace_with_negative_normal_works() {
        let aabb = AxisAlignedBox::new(Point3::new(0.0, 0.0, 0.0), Point3::new(2.0, 2.0, 2.0));
        let plane = Plane::from_normal_and_point(
            UnitVector3::unchecked_from(Vector3::new(0.0, 0.0, -1.0)),
            &Point3::new(0.0, 0.0, 1.0),
        );
        let projected = aabb.projected_onto_negative_halfspace(&plane);
        let expected = AxisAlignedBox::new(Point3::new(0.0, 0.0, 1.0), Point3::new(2.0, 2.0, 2.0));
        assert_abs_diff_eq!(projected, expected);
    }

    #[test]
    fn projecting_onto_negative_halfspace_with_diagonal_plane_works() {
        let aabb = AxisAlignedBox::new(Point3::new(0.0, 0.0, 0.0), Point3::new(2.0, 2.0, 2.0));
        let plane = Plane::from_normal_and_point(
            UnitVector3::normalized_from(Vector3::new(1.0, 1.0, 0.0)),
            &Point3::new(1.0, 1.0, 0.0),
        );
        let projected = aabb.projected_onto_negative_halfspace(&plane);

        // The projection should reduce the box size while preserving volume in negative halfspace
        assert!(projected.corners[0].x() >= aabb.corners[0].x());
        assert!(projected.corners[0].y() >= aabb.corners[0].y());
        assert!(projected.corners[1].x() <= aabb.corners[1].x());
        assert!(projected.corners[1].y() <= aabb.corners[1].y());
        assert_abs_diff_eq!(projected.corners[0].z(), aabb.corners[0].z());
        assert_abs_diff_eq!(projected.corners[1].z(), aabb.corners[1].z());
    }

    #[test]
    fn projecting_onto_negative_halfspace_with_plane_parallel_to_axis_preserves_other_dimensions() {
        let aabb = AxisAlignedBox::new(Point3::new(-2.0, -3.0, -4.0), Point3::new(5.0, 7.0, 8.0));
        let plane =
            Plane::from_normal_and_point(UnitVector3::unit_z(), &Point3::new(0.0, 0.0, 2.0));
        let projected = aabb.projected_onto_negative_halfspace(&plane);

        // X and Y dimensions should be unchanged
        assert_abs_diff_eq!(projected.corners[0].x(), aabb.corners[0].x());
        assert_abs_diff_eq!(projected.corners[0].y(), aabb.corners[0].y());
        assert_abs_diff_eq!(projected.corners[1].x(), aabb.corners[1].x());
        assert_abs_diff_eq!(projected.corners[1].y(), aabb.corners[1].y());

        // Z dimension should be clipped
        assert_abs_diff_eq!(projected.corners[0].z(), aabb.corners[0].z());
        assert_abs_diff_eq!(projected.corners[1].z(), 2.0);
    }

    #[test]
    fn projecting_onto_negative_halfspace_with_box_touching_plane_works() {
        let aabb = AxisAlignedBox::new(Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 1.0, 1.0));
        let plane =
            Plane::from_normal_and_point(UnitVector3::unit_z(), &Point3::new(0.0, 0.0, 1.0));
        let projected = aabb.projected_onto_negative_halfspace(&plane);
        let expected = AxisAlignedBox::new(Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 1.0, 1.0));
        assert_abs_diff_eq!(projected, expected);
    }

    #[test]
    fn find_ray_intersection_with_ray_through_box_center_works() {
        let aabb = AxisAlignedBox::new(Point3::new(0.0, 0.0, 0.0), Point3::new(2.0, 2.0, 2.0));
        let ray_origin = Point3::new(-1.0, 1.0, 1.0);
        let ray_direction = UnitVector3::normalized_from(Vector3::new(1.0, 0.0, 0.0));

        let result = aabb.find_ray_intersection(&ray_origin, &ray_direction);
        assert!(result.is_some());
        let (t_min, t_max) = result.unwrap();
        assert_abs_diff_eq!(t_min, 1.0, epsilon = 1e-6);
        assert_abs_diff_eq!(t_max, 3.0, epsilon = 1e-6);
    }

    #[test]
    fn find_ray_intersection_with_ray_missing_box_returns_none() {
        let aabb = AxisAlignedBox::new(Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 1.0, 1.0));
        let ray_origin = Point3::new(2.0, 2.0, 2.0);
        let ray_direction = UnitVector3::normalized_from(Vector3::new(1.0, 0.0, 0.0));

        let result = aabb.find_ray_intersection(&ray_origin, &ray_direction);
        assert!(result.is_none());
    }

    #[test]
    fn find_ray_intersection_with_ray_starting_inside_box_works() {
        let aabb = AxisAlignedBox::new(Point3::new(0.0, 0.0, 0.0), Point3::new(2.0, 2.0, 2.0));
        let ray_origin = Point3::new(1.0, 1.0, 1.0);
        let ray_direction = UnitVector3::normalized_from(Vector3::new(1.0, 0.0, 0.0));

        let result = aabb.find_ray_intersection(&ray_origin, &ray_direction);
        assert!(result.is_some());
        let (t_min, t_max) = result.unwrap();
        assert_abs_diff_eq!(t_min, 0.0, epsilon = 1e-6);
        assert_abs_diff_eq!(t_max, 1.0, epsilon = 1e-6);
    }

    #[test]
    fn find_ray_intersection_with_ray_parallel_to_box_axis_outside_box_returns_none() {
        let aabb = AxisAlignedBox::new(Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 1.0, 1.0));
        // Ray parallel to x-axis but at y=0.5, z=2.0 (outside the box in z dimension)
        let ray_origin = Point3::new(-1.0, 0.5, 2.0);
        let ray_direction = UnitVector3::normalized_from(Vector3::new(1.0, 0.0, 0.0));

        let result = aabb.find_ray_intersection(&ray_origin, &ray_direction);
        assert!(result.is_none());
    }

    #[test]
    fn find_ray_intersection_with_ray_behind_box_returns_none() {
        let aabb = AxisAlignedBox::new(Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 1.0, 1.0));
        // Ray would intersect box if extended backwards, but only forward direction counts
        let ray_origin = Point3::new(2.0, 0.5, 0.5);
        let ray_direction = UnitVector3::normalized_from(Vector3::new(1.0, 0.0, 0.0));

        let result = aabb.find_ray_intersection(&ray_origin, &ray_direction);
        assert!(result.is_none());
    }

    #[test]
    fn find_ray_intersection_with_diagonal_ray_works() {
        let aabb = AxisAlignedBox::new(Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 1.0, 1.0));
        let ray_origin = Point3::new(-1.0, -1.0, -1.0);
        let ray_direction = UnitVector3::normalized_from(Vector3::new(1.0, 1.0, 1.0));

        let result = aabb.find_ray_intersection(&ray_origin, &ray_direction);
        assert!(result.is_some());
        // Ray enters box when all coordinates reach 0, exits when all reach 1
        let (t_min, t_max) = result.unwrap();
        let sqrt_3 = f32::sqrt(3.0);
        assert_abs_diff_eq!(t_min, sqrt_3, epsilon = 1e-6);
        assert_abs_diff_eq!(t_max, sqrt_3 * 2.0, epsilon = 1e-6);
    }

    #[test]
    fn find_ray_intersection_with_zero_direction_component_inside_bounds_works() {
        let aabb = AxisAlignedBox::new(Point3::new(0.0, 0.0, 0.0), Point3::new(2.0, 2.0, 2.0));
        // Ray with zero y-component but y-coordinate inside box bounds
        let ray_origin = Point3::new(-1.0, 1.0, 1.0);
        let ray_direction = UnitVector3::normalized_from(Vector3::new(1.0, 0.0, 0.0));

        let result = aabb.find_ray_intersection(&ray_origin, &ray_direction);
        assert!(result.is_some());
        let (t_min, t_max) = result.unwrap();
        assert_abs_diff_eq!(t_min, 1.0, epsilon = 1e-6);
        assert_abs_diff_eq!(t_max, 3.0, epsilon = 1e-6);
    }

    #[test]
    fn find_ray_intersection_with_zero_direction_component_outside_bounds_returns_none() {
        let aabb = AxisAlignedBox::new(Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 1.0, 1.0));
        // Ray with zero y-component but y-coordinate outside box bounds
        let ray_origin = Point3::new(-1.0, 2.0, 0.5);
        let ray_direction = UnitVector3::normalized_from(Vector3::new(1.0, 0.0, 0.0));

        let result = aabb.find_ray_intersection(&ray_origin, &ray_direction);
        assert!(result.is_none());
    }
}
