//! Representation of boxes with arbitrary orientations.

use crate::{AxisAlignedBox, Plane};
use impact_math::Float;
use nalgebra::{Isometry3, Point3, Similarity3, UnitQuaternion, UnitVector3, Vector3, point};

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

    /// Whether the given point is inside this box. A point exactly on the
    /// surface of the box is considered inside.
    pub fn contains_point(&self, point: &Point3<F>) -> bool {
        let point_in_box_frame = self.transform_point_to_box_frame(point);

        point_in_box_frame.x.abs() <= self.half_width
            && point_in_box_frame.y.abs() <= self.half_height
            && point_in_box_frame.z.abs() <= self.half_depth
    }

    /// Transforms the given point to the frame with origin at the center of the
    /// box and with x-, y- and z-axes aligned with the width-, height- and
    /// depth-axes of the box, respectively.
    pub fn transform_point_to_box_frame(&self, point: &Point3<F>) -> Point3<F> {
        self.orientation
            .inverse_transform_vector(&(point - self.center))
            .into()
    }

    /// Transforms the given point from the frame with origin at the center of the
    /// box and with x-, y- and z-axes aligned with the width-, height- and
    /// depth-axes of the box, respectively.
    pub fn transform_point_from_box_frame(&self, point: &Point3<F>) -> Point3<F> {
        self.center + self.orientation.transform_vector(&point.coords)
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

    /// Creates a new box corresponding to transforming this box with the given
    /// isometry transform.
    pub fn translated_and_rotated(&self, transform: &Isometry3<F>) -> Self {
        Self::new(
            transform.transform_point(&self.center),
            transform.rotation * self.orientation,
            self.half_width,
            self.half_height,
            self.half_depth,
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

/// Determines the region where box A and B intersect, and computes the
/// axis-aligned bounding boxes for this region as seen from the orientation of
/// both boxes.
///
/// Without loss of generality, box A is assumed axis-aligned. The first
/// returned AABB is the subdomain of box A enclosing the part of box B
/// intersecting A. The second returned AABB is the subdomain of box B enclosing
/// the part of A intersecting B, expressed in box B's reference frame, which is
/// the frame where B is axis-aligned with its center at the origin.
pub fn compute_box_intersection_bounds<F: Float>(
    box_a: &AxisAlignedBox<F>,
    box_b: &OrientedBox<F>,
) -> Option<(AxisAlignedBox<F>, AxisAlignedBox<F>)> {
    // Edge list (used for both boxes)
    const EDGES: [(usize, usize); 12] = [
        (0, 1),
        (2, 3),
        (4, 5),
        (6, 7),
        (0, 2),
        (1, 3),
        (4, 6),
        (5, 7),
        (0, 4),
        (1, 5),
        (2, 6),
        (3, 7),
    ];

    // Running bounds with +/- infinity
    let mut box_a_lower = Point3::new(F::INFINITY, F::INFINITY, F::INFINITY);
    let mut box_a_upper = Point3::new(-F::INFINITY, -F::INFINITY, -F::INFINITY);
    let mut box_b_lower = Point3::new(F::INFINITY, F::INFINITY, F::INFINITY);
    let mut box_b_upper = Point3::new(-F::INFINITY, -F::INFINITY, -F::INFINITY);
    let mut boxes_intersect = false;

    let mut expand_bounds = |point_in_box_a_frame: &Point3<F>, point_in_box_b_frame: &Point3<F>| {
        box_a_lower = box_a_lower.inf(point_in_box_a_frame);
        box_a_upper = box_a_upper.sup(point_in_box_a_frame);
        box_b_lower = box_b_lower.inf(point_in_box_b_frame);
        box_b_upper = box_b_upper.sup(point_in_box_b_frame);
        boxes_intersect = true;
    };

    // Expand bounds for all edges of box B intersecting box A

    let box_b_corners = box_b.compute_corners();

    for (i, j) in EDGES {
        let box_b_edge_start = box_b_corners[i];
        let box_b_edge_end = box_b_corners[j];
        let box_b_edge_vector = box_b_edge_end - box_b_edge_start;

        if let Some((t_min, t_max)) =
            box_a.find_contained_subsegment(box_b_edge_start, box_b_edge_vector)
        {
            let contained_box_b_edge_start = box_b_edge_start + box_b_edge_vector * t_min;
            let contained_box_b_edge_end = box_b_edge_start + box_b_edge_vector * t_max;

            let contained_box_b_edge_start_in_box_b_frame =
                box_b.transform_point_to_box_frame(&contained_box_b_edge_start);
            let contained_box_b_edge_end_in_box_b_frame =
                box_b.transform_point_to_box_frame(&contained_box_b_edge_end);

            expand_bounds(
                &contained_box_b_edge_start,
                &contained_box_b_edge_start_in_box_b_frame,
            );
            expand_bounds(
                &contained_box_b_edge_end,
                &contained_box_b_edge_end_in_box_b_frame,
            );
        }
    }

    // Expand bounds for all edges of box A intersecting box B

    let box_a_corners_in_box_b_frame = box_a
        .all_corners()
        .map(|corner| box_b.transform_point_to_box_frame(&corner));

    let box_b_in_box_b_frame = AxisAlignedBox::new(
        point![
            -box_b.half_width(),
            -box_b.half_height(),
            -box_b.half_depth()
        ],
        point![box_b.half_width(), box_b.half_height(), box_b.half_depth()],
    );

    for (i, j) in EDGES {
        let box_a_edge_start_in_box_b_frame = box_a_corners_in_box_b_frame[i];
        let box_a_edge_end_in_box_b_frame = box_a_corners_in_box_b_frame[j];
        let box_a_edge_vector_in_box_b_frame =
            box_a_edge_end_in_box_b_frame - box_a_edge_start_in_box_b_frame;

        if let Some((t_min, t_max)) = box_b_in_box_b_frame.find_contained_subsegment(
            box_a_edge_start_in_box_b_frame,
            box_a_edge_vector_in_box_b_frame,
        ) {
            let contained_box_a_edge_start_in_box_b_frame =
                box_a_edge_start_in_box_b_frame + box_a_edge_vector_in_box_b_frame * t_min;
            let contained_box_a_edge_end_in_box_b_frame =
                box_a_edge_start_in_box_b_frame + box_a_edge_vector_in_box_b_frame * t_max;

            let contained_box_a_edge_start =
                box_b.transform_point_from_box_frame(&contained_box_a_edge_start_in_box_b_frame);
            let contained_box_a_edge_end =
                box_b.transform_point_from_box_frame(&contained_box_a_edge_end_in_box_b_frame);

            expand_bounds(
                &contained_box_a_edge_start,
                &contained_box_a_edge_start_in_box_b_frame,
            );
            expand_bounds(
                &contained_box_a_edge_end,
                &contained_box_a_edge_end_in_box_b_frame,
            );
        }
    }

    if boxes_intersect {
        Some((
            AxisAlignedBox::new(box_a_lower, box_a_upper),
            AxisAlignedBox::new(box_b_lower, box_b_upper),
        ))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Frustum, OrthographicTransform};
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

    #[test]
    fn axis_aligned_box_contains_center_point() {
        let oriented_box = OrientedBox::aligned_at_origin(2.0, 3.0, 1.5);
        assert!(oriented_box.contains_point(&Point3::origin()));
    }

    #[test]
    fn axis_aligned_box_contains_interior_point() {
        let oriented_box = OrientedBox::aligned_at_origin(2.0, 3.0, 1.5);
        assert!(oriented_box.contains_point(&point![1.5, 2.5, 1.0]));
    }

    #[test]
    fn axis_aligned_box_contains_surface_points() {
        let oriented_box = OrientedBox::aligned_at_origin(2.0, 3.0, 1.5);

        // Points on each face should be inside
        assert!(oriented_box.contains_point(&point![2.0, 0.0, 0.0])); // +X face
        assert!(oriented_box.contains_point(&point![-2.0, 0.0, 0.0])); // -X face
        assert!(oriented_box.contains_point(&point![0.0, 3.0, 0.0])); // +Y face
        assert!(oriented_box.contains_point(&point![0.0, -3.0, 0.0])); // -Y face
        assert!(oriented_box.contains_point(&point![0.0, 0.0, 1.5])); // +Z face
        assert!(oriented_box.contains_point(&point![0.0, 0.0, -1.5])); // -Z face
    }

    #[test]
    fn axis_aligned_box_contains_corner_points() {
        let oriented_box = OrientedBox::aligned_at_origin(2.0, 3.0, 1.5);

        // Test representative corners
        assert!(oriented_box.contains_point(&point![2.0, 3.0, 1.5]));
        assert!(oriented_box.contains_point(&point![-2.0, -3.0, -1.5]));
    }

    #[test]
    fn axis_aligned_box_excludes_exterior_points() {
        let oriented_box = OrientedBox::aligned_at_origin(2.0, 3.0, 1.5);

        // Points outside each face should be excluded
        assert!(!oriented_box.contains_point(&point![2.1, 0.0, 0.0])); // Beyond +X
        assert!(!oriented_box.contains_point(&point![-2.1, 0.0, 0.0])); // Beyond -X
        assert!(!oriented_box.contains_point(&point![0.0, 3.1, 0.0])); // Beyond +Y
        assert!(!oriented_box.contains_point(&point![0.0, -3.1, 0.0])); // Beyond -Y
        assert!(!oriented_box.contains_point(&point![0.0, 0.0, 1.6])); // Beyond +Z
        assert!(!oriented_box.contains_point(&point![0.0, 0.0, -1.6])); // Beyond -Z
    }

    #[test]
    fn rotated_box_contains_center_point() {
        let center = point![1.0, 2.0, 3.0];
        let rotation = UnitQuaternion::from_axis_angle(&Vector3::z_axis(), PI / 4.0);
        let oriented_box = OrientedBox::new(center, rotation, 1.0, 1.0, 1.0);

        assert!(oriented_box.contains_point(&center));
    }

    #[test]
    fn rotated_box_contains_rotated_corner_point() {
        let center = point![0.0, 0.0, 0.0];
        let rotation = UnitQuaternion::from_axis_angle(&Vector3::z_axis(), PI / 4.0);
        let oriented_box = OrientedBox::new(center, rotation, 1.0, 1.0, 1.0);

        // Corner in box frame, transformed to world frame
        let corner_in_box_frame = Vector3::new(1.0, 1.0, 1.0);
        let corner_in_world = center + rotation.transform_vector(&corner_in_box_frame);

        assert!(oriented_box.contains_point(&corner_in_world));
    }

    #[test]
    fn rotated_box_excludes_exterior_points() {
        let rotation = UnitQuaternion::from_axis_angle(&Vector3::z_axis(), PI / 4.0);
        let oriented_box = OrientedBox::new(Point3::origin(), rotation, 1.0, 1.0, 1.0);

        // Point outside rotated bounds
        assert!(!oriented_box.contains_point(&point![1.5, 1.5, 0.0]));
        assert!(!oriented_box.contains_point(&point![2.0, 0.0, 0.0]));
    }

    #[test]
    fn translated_box_contains_center_point() {
        let center = point![5.0, -3.0, 2.0];
        let oriented_box = OrientedBox::new(center, UnitQuaternion::identity(), 2.0, 1.5, 3.0);

        assert!(oriented_box.contains_point(&center));
    }

    #[test]
    fn translated_box_contains_surface_points() {
        let center = point![5.0, -3.0, 2.0];
        let oriented_box = OrientedBox::new(center, UnitQuaternion::identity(), 2.0, 1.5, 3.0);

        // Points on box faces should be inside
        assert!(oriented_box.contains_point(&point![3.0, -3.0, 2.0])); // -X face
        assert!(oriented_box.contains_point(&point![7.0, -3.0, 2.0])); // +X face
    }

    #[test]
    fn translated_box_excludes_exterior_points() {
        let center = point![5.0, -3.0, 2.0];
        let oriented_box = OrientedBox::new(center, UnitQuaternion::identity(), 2.0, 1.5, 3.0);

        // Points outside box bounds should be excluded
        assert!(!oriented_box.contains_point(&point![2.9, -3.0, 2.0])); // Beyond -X
        assert!(!oriented_box.contains_point(&point![7.1, -3.0, 2.0])); // Beyond +X
    }

    #[test]
    fn compute_box_intersection_bounds_with_non_intersecting_boxes_returns_none() {
        // Box A: centered at origin, size 2x2x2
        let box_a = AxisAlignedBox::new(point![-1.0, -1.0, -1.0], point![1.0, 1.0, 1.0]);

        // Box B: far away, no intersection
        let box_b = OrientedBox::new(
            point![5.0, 0.0, 0.0],
            UnitQuaternion::identity(),
            1.0,
            1.0,
            1.0,
        );

        assert!(compute_box_intersection_bounds(&box_a, &box_b).is_none());
    }

    #[test]
    fn compute_box_intersection_bounds_with_identical_axis_aligned_boxes_works() {
        // Both boxes are identical and axis-aligned
        let box_a = AxisAlignedBox::new(point![-1.0, -1.0, -1.0], point![1.0, 1.0, 1.0]);
        let box_b = OrientedBox::new(
            point![0.0, 0.0, 0.0],
            UnitQuaternion::identity(),
            1.0,
            1.0,
            1.0,
        );

        let (bounds_a, bounds_b) = compute_box_intersection_bounds(&box_a, &box_b).unwrap();

        // Both bounds should encompass the entire boxes
        assert_abs_diff_eq!(
            bounds_a.lower_corner(),
            &point![-1.0, -1.0, -1.0],
            epsilon = 1e-10
        );
        assert_abs_diff_eq!(
            bounds_a.upper_corner(),
            &point![1.0, 1.0, 1.0],
            epsilon = 1e-10
        );
        assert_abs_diff_eq!(
            bounds_b.lower_corner(),
            &point![-1.0, -1.0, -1.0],
            epsilon = 1e-10
        );
        assert_abs_diff_eq!(
            bounds_b.upper_corner(),
            &point![1.0, 1.0, 1.0],
            epsilon = 1e-10
        );
    }

    #[test]
    fn compute_box_intersection_bounds_with_partial_overlap_works() {
        // Box A: centered at origin, size 2x2x2
        let box_a = AxisAlignedBox::new(point![-1.0, -1.0, -1.0], point![1.0, 1.0, 1.0]);

        // Box B: translated so it partially overlaps
        let box_b = OrientedBox::new(
            point![1.0, 0.0, 0.0],
            UnitQuaternion::identity(),
            1.0,
            1.0,
            1.0,
        );

        let (bounds_a, bounds_b) = compute_box_intersection_bounds(&box_a, &box_b).unwrap();

        // The intersection region in box A's frame should be on the +X side
        assert_abs_diff_eq!(bounds_a.lower_corner().x, 0.0, epsilon = 1e-10);
        assert_abs_diff_eq!(bounds_a.upper_corner().x, 1.0, epsilon = 1e-10);
        assert_abs_diff_eq!(bounds_a.lower_corner().y, -1.0, epsilon = 1e-10);
        assert_abs_diff_eq!(bounds_a.upper_corner().y, 1.0, epsilon = 1e-10);
        assert_abs_diff_eq!(bounds_a.lower_corner().z, -1.0, epsilon = 1e-10);
        assert_abs_diff_eq!(bounds_a.upper_corner().z, 1.0, epsilon = 1e-10);

        // The intersection region in box B's frame should be on the -X side
        assert_abs_diff_eq!(bounds_b.lower_corner().x, -1.0, epsilon = 1e-10);
        assert_abs_diff_eq!(bounds_b.upper_corner().x, 0.0, epsilon = 1e-10);
        assert_abs_diff_eq!(bounds_b.lower_corner().y, -1.0, epsilon = 1e-10);
        assert_abs_diff_eq!(bounds_b.upper_corner().y, 1.0, epsilon = 1e-10);
        assert_abs_diff_eq!(bounds_b.lower_corner().z, -1.0, epsilon = 1e-10);
        assert_abs_diff_eq!(bounds_b.upper_corner().z, 1.0, epsilon = 1e-10);
    }

    #[test]
    fn compute_box_intersection_bounds_with_one_box_inside_other_works() {
        // Box A: large box
        let box_a = AxisAlignedBox::new(point![-2.0, -2.0, -2.0], point![2.0, 2.0, 2.0]);

        // Box B: small box inside A
        let box_b = OrientedBox::new(
            point![0.0, 0.0, 0.0],
            UnitQuaternion::identity(),
            0.5,
            0.5,
            0.5,
        );

        let (bounds_a, bounds_b) = compute_box_intersection_bounds(&box_a, &box_b).unwrap();

        // bounds_a should encompass the entire small box
        assert_abs_diff_eq!(
            bounds_a.lower_corner(),
            &point![-0.5, -0.5, -0.5],
            epsilon = 1e-10
        );
        assert_abs_diff_eq!(
            bounds_a.upper_corner(),
            &point![0.5, 0.5, 0.5],
            epsilon = 1e-10
        );

        // bounds_b should encompass the entire box B (since it's completely inside)
        assert_abs_diff_eq!(
            bounds_b.lower_corner(),
            &point![-0.5, -0.5, -0.5],
            epsilon = 1e-10
        );
        assert_abs_diff_eq!(
            bounds_b.upper_corner(),
            &point![0.5, 0.5, 0.5],
            epsilon = 1e-10
        );
    }

    #[test]
    fn compute_box_intersection_bounds_with_rotated_box_works() {
        // Box A: axis-aligned box
        let box_a = AxisAlignedBox::new(point![-1.0, -1.0, -1.0], point![1.0, 1.0, 1.0]);

        // Box B: rotated 45 degrees around Z axis
        let rotation = UnitQuaternion::from_axis_angle(&Vector3::z_axis(), PI / 4.0);
        let box_b = OrientedBox::new(point![0.0, 0.0, 0.0], rotation, 1.0, 1.0, 1.0);

        let (bounds_a, bounds_b) = compute_box_intersection_bounds(&box_a, &box_b).unwrap();

        // Both bounds should be within their respective boxes
        assert!(bounds_a.lower_corner().x >= -1.0);
        assert!(bounds_a.lower_corner().y >= -1.0);
        assert!(bounds_a.lower_corner().z >= -1.0);
        assert!(bounds_a.upper_corner().x <= 1.0);
        assert!(bounds_a.upper_corner().y <= 1.0);
        assert!(bounds_a.upper_corner().z <= 1.0);

        assert!(bounds_b.lower_corner().x >= -1.0);
        assert!(bounds_b.lower_corner().y >= -1.0);
        assert!(bounds_b.lower_corner().z >= -1.0);
        assert!(bounds_b.upper_corner().x <= 1.0);
        assert!(bounds_b.upper_corner().y <= 1.0);
        assert!(bounds_b.upper_corner().z <= 1.0);
    }

    #[test]
    fn compute_box_intersection_bounds_with_touching_boxes_works() {
        // Box A: centered at origin
        let box_a = AxisAlignedBox::new(point![-1.0, -1.0, -1.0], point![1.0, 1.0, 1.0]);

        // Box B: touching on the +X face
        let box_b = OrientedBox::new(
            point![2.0, 0.0, 0.0],
            UnitQuaternion::identity(),
            1.0,
            1.0,
            1.0,
        );

        let (bounds_a, bounds_b) = compute_box_intersection_bounds(&box_a, &box_b).unwrap();

        // The touching face should result in bounds with zero width in X
        assert_abs_diff_eq!(bounds_a.lower_corner().x, 1.0, epsilon = 1e-10);
        assert_abs_diff_eq!(bounds_a.upper_corner().x, 1.0, epsilon = 1e-10);
        assert_abs_diff_eq!(bounds_a.lower_corner().y, -1.0, epsilon = 1e-10);
        assert_abs_diff_eq!(bounds_a.upper_corner().y, 1.0, epsilon = 1e-10);
        assert_abs_diff_eq!(bounds_a.lower_corner().z, -1.0, epsilon = 1e-10);
        assert_abs_diff_eq!(bounds_a.upper_corner().z, 1.0, epsilon = 1e-10);

        assert_abs_diff_eq!(bounds_b.lower_corner().x, -1.0, epsilon = 1e-10);
        assert_abs_diff_eq!(bounds_b.upper_corner().x, -1.0, epsilon = 1e-10);
        assert_abs_diff_eq!(bounds_b.lower_corner().y, -1.0, epsilon = 1e-10);
        assert_abs_diff_eq!(bounds_b.upper_corner().y, 1.0, epsilon = 1e-10);
        assert_abs_diff_eq!(bounds_b.lower_corner().z, -1.0, epsilon = 1e-10);
        assert_abs_diff_eq!(bounds_b.upper_corner().z, 1.0, epsilon = 1e-10);
    }

    #[test]
    fn compute_box_intersection_bounds_with_corner_intersection_works() {
        // Box A: at origin
        let box_a = AxisAlignedBox::new(point![-1.0, -1.0, -1.0], point![1.0, 1.0, 1.0]);

        // Box B: positioned so only corners intersect
        let box_b = OrientedBox::new(
            point![1.5, 1.5, 1.5],
            UnitQuaternion::identity(),
            1.0,
            1.0,
            1.0,
        );

        let (bounds_a, bounds_b) = compute_box_intersection_bounds(&box_a, &box_b).unwrap();

        // The intersection should be the corner region [0.5, 1.0] in all dimensions
        assert_abs_diff_eq!(bounds_a.lower_corner().x, 0.5, epsilon = 1e-10);
        assert_abs_diff_eq!(bounds_a.upper_corner().x, 1.0, epsilon = 1e-10);
        assert_abs_diff_eq!(bounds_a.lower_corner().y, 0.5, epsilon = 1e-10);
        assert_abs_diff_eq!(bounds_a.upper_corner().y, 1.0, epsilon = 1e-10);
        assert_abs_diff_eq!(bounds_a.lower_corner().z, 0.5, epsilon = 1e-10);
        assert_abs_diff_eq!(bounds_a.upper_corner().z, 1.0, epsilon = 1e-10);

        assert_abs_diff_eq!(bounds_b.lower_corner().x, -1.0, epsilon = 1e-10);
        assert_abs_diff_eq!(bounds_b.upper_corner().x, -0.5, epsilon = 1e-10);
        assert_abs_diff_eq!(bounds_b.lower_corner().y, -1.0, epsilon = 1e-10);
        assert_abs_diff_eq!(bounds_b.upper_corner().y, -0.5, epsilon = 1e-10);
        assert_abs_diff_eq!(bounds_b.lower_corner().z, -1.0, epsilon = 1e-10);
        assert_abs_diff_eq!(bounds_b.upper_corner().z, -0.5, epsilon = 1e-10);
    }

    #[test]
    fn compute_box_intersection_bounds_with_different_sized_boxes_works() {
        // Box A: small box
        let box_a = AxisAlignedBox::new(point![-0.5, -0.5, -0.5], point![0.5, 0.5, 0.5]);

        // Box B: large box overlapping A
        let box_b = OrientedBox::new(
            point![0.0, 0.0, 0.0],
            UnitQuaternion::identity(),
            2.0,
            2.0,
            2.0,
        );

        let (bounds_a, bounds_b) = compute_box_intersection_bounds(&box_a, &box_b).unwrap();

        // bounds_a should encompass the entire small box A
        assert_abs_diff_eq!(
            bounds_a.lower_corner(),
            &point![-0.5, -0.5, -0.5],
            epsilon = 1e-10
        );
        assert_abs_diff_eq!(
            bounds_a.upper_corner(),
            &point![0.5, 0.5, 0.5],
            epsilon = 1e-10
        );

        // bounds_b should be the central region of box B
        assert_abs_diff_eq!(
            bounds_b.lower_corner(),
            &point![-0.5, -0.5, -0.5],
            epsilon = 1e-10
        );
        assert_abs_diff_eq!(
            bounds_b.upper_corner(),
            &point![0.5, 0.5, 0.5],
            epsilon = 1e-10
        );
    }

    #[test]
    fn compute_box_intersection_bounds_with_translated_and_rotated_box_works() {
        // Box A: axis-aligned at origin
        let box_a = AxisAlignedBox::new(point![-1.0, -1.0, -1.0], point![1.0, 1.0, 1.0]);

        // Box B: translated and rotated
        let rotation = UnitQuaternion::from_axis_angle(&Vector3::y_axis(), PI / 6.0);
        let box_b = OrientedBox::new(point![0.5, 0.5, 0.0], rotation, 1.0, 1.0, 1.0);

        let (bounds_a, _bounds_b) = compute_box_intersection_bounds(&box_a, &box_b).unwrap();

        // Verify bounds are within their respective coordinate systems
        assert!(bounds_a.lower_corner().x >= -1.0);
        assert!(bounds_a.lower_corner().y >= -1.0);
        assert!(bounds_a.lower_corner().z >= -1.0);
        assert!(bounds_a.upper_corner().x <= 1.0);
        assert!(bounds_a.upper_corner().y <= 1.0);
        assert!(bounds_a.upper_corner().z <= 1.0);

        // The intersection should have positive volume
        let volume_a = (bounds_a.upper_corner() - bounds_a.lower_corner()).product();
        assert!(volume_a > 0.0);
    }

    #[test]
    fn compute_box_intersection_bounds_edge_case_just_separated_returns_none() {
        // Box A: at origin
        let box_a = AxisAlignedBox::new(point![-1.0, -1.0, -1.0], point![1.0, 1.0, 1.0]);

        // Box B: just barely separated (should not intersect)
        let box_b = OrientedBox::new(
            point![2.001, 0.0, 0.0],
            UnitQuaternion::identity(),
            1.0,
            1.0,
            1.0,
        );

        assert!(compute_box_intersection_bounds(&box_a, &box_b).is_none());
    }
}
