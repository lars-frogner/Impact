//! Representation of spheres.

use crate::{AxisAlignedBox, Point};
use approx::abs_diff_eq;
use bytemuck::{Pod, Zeroable};
use impact_math::Float;
use na::{Similarity3, UnitQuaternion, vector};
use nalgebra::{self as na, Isometry3, Point3};

/// A sphere represented by the center point and the radius.
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Sphere<F: Float> {
    center: Point3<F>,
    radius: F,
}

// WARNING: Do not change `Sphere` without ensuring that `Sphere<f32>` and
// `Sphere<f64>` are still `Pod`
unsafe impl Zeroable for Sphere<f32> {}
unsafe impl Zeroable for Sphere<f64> {}
unsafe impl Pod for Sphere<f32> {}
unsafe impl Pod for Sphere<f64> {}

impl<F: Float> Sphere<F> {
    /// Creates a new sphere with the given center and radius.
    ///
    /// # Panics
    /// If `radius` is negative.
    pub fn new(center: Point3<F>, radius: F) -> Self {
        assert!(radius >= F::zero());
        Self { center, radius }
    }

    /// Finds the smallest sphere that fully encloses the two
    /// given spheres.
    pub fn bounding_sphere_from_pair(sphere_1: &Self, sphere_2: &Self) -> Self {
        let center_displacement = sphere_2.center() - sphere_1.center();
        let distance_between_centra = center_displacement.magnitude();

        if Self::first_sphere_encloses_second_sphere(
            sphere_1.radius(),
            sphere_2.radius(),
            distance_between_centra,
        ) {
            return *sphere_1;
        } else if Self::first_sphere_encloses_second_sphere(
            sphere_2.radius(),
            sphere_1.radius(),
            distance_between_centra,
        ) {
            return *sphere_2;
        }

        let bounding_radius =
            F::ONE_HALF * (distance_between_centra + sphere_1.radius() + sphere_2.radius());

        let mean_center = na::center(sphere_1.center(), sphere_2.center());

        let bounding_center = if abs_diff_eq!(distance_between_centra, F::zero()) {
            mean_center
        } else {
            mean_center
                + center_displacement
                    * (F::ONE_HALF * (sphere_2.radius() - sphere_1.radius())
                        / distance_between_centra)
        };

        // Increase radius enough to guarantee that both spheres
        // will test as being fully inside regardless of rounding errors
        let bounding_radius = bounding_radius + F::default_epsilon();

        Self::new(bounding_center, bounding_radius)
    }

    /// Finds the smallest sphere enclosing the given axis-aligned bounding box.
    pub fn bounding_sphere_from_aabb(aabb: &AxisAlignedBox<F>) -> Self {
        let center = aabb.center();
        let radius = F::ONE_HALF * na::distance(aabb.lower_corner(), aabb.upper_corner());
        Self::new(center, radius)
    }

    /// Finds a sphere enclosing the given points.
    ///
    /// # Panics
    /// If the point slice is empty.
    pub fn bounding_sphere_for_points(points: &[impl Point<F>]) -> Self {
        assert!(
            !points.is_empty(),
            "Tried to create bounding sphere for empty point slice"
        );

        let one_over_count = F::from_usize(points.len()).unwrap().recip();

        let first_point = points[0].point().coords;

        let centroid: Point3<F> = points
            .iter()
            .skip(1)
            .fold(first_point, |sum, point| sum + point.point().coords)
            .scale(one_over_count)
            .into();

        let max_squared_dist_from_centroid =
            points.iter().fold(F::ZERO, |max_squared_dist, point| {
                na::distance_squared(point.point(), &centroid).max(max_squared_dist)
            });

        Self::new(centroid, max_squared_dist_from_centroid.sqrt())
    }

    /// Returns the center point of the sphere.
    pub fn center(&self) -> &Point3<F> {
        &self.center
    }

    /// Returns the radius of the sphere.
    pub fn radius(&self) -> F {
        self.radius
    }

    /// Returns the square of the radius of the sphere.
    pub fn radius_squared(&self) -> F {
        self.radius.powi(2)
    }

    /// Whether the given sphere is fully inside this sphere.
    /// A sphere is considered to enclose itself.
    pub fn encloses_sphere(&self, sphere: &Self) -> bool {
        Self::first_sphere_encloses_second_sphere(
            self.radius(),
            sphere.radius(),
            na::distance(self.center(), sphere.center()),
        )
    }

    /// Whether the given point is inside this sphere. A point
    /// exactly on the surface of the sphere is considered
    /// inside.
    pub fn contains_point(&self, point: &Point3<F>) -> bool {
        na::distance_squared(self.center(), point) <= self.radius_squared()
    }

    /// Whether all of the sphere is strictly outside the given axis-aligned
    /// box. The sphere is considered inside if the boundaries exactly touch
    /// each other.
    pub fn is_outside_axis_aligned_box(&self, axis_aligned_box: &AxisAlignedBox<F>) -> bool {
        let lower_corner = axis_aligned_box.lower_corner();
        let upper_corner = axis_aligned_box.upper_corner();

        let mut min_squared_distance_from_center = F::ZERO;
        for idx in 0..3 {
            if upper_corner[idx] < self.center[idx] {
                min_squared_distance_from_center += (self.center[idx] - upper_corner[idx]).powi(2);
            } else if lower_corner[idx] > self.center[idx] {
                min_squared_distance_from_center += (lower_corner[idx] - self.center[idx]).powi(2);
            }
        }

        min_squared_distance_from_center > self.radius_squared()
    }

    /// Whether all of the the given axis-aligned box is inside the sphere. The
    /// box is considered inside if the boundaries exactly touch each other.
    pub fn contains_axis_aligned_box(&self, axis_aligned_box: &AxisAlignedBox<F>) -> bool {
        let lower_corner = axis_aligned_box.lower_corner();
        let upper_corner = axis_aligned_box.upper_corner();

        let mut max_squared_distance_from_center = F::ZERO;
        for idx in 0..3 {
            max_squared_distance_from_center += F::max(
                (lower_corner[idx] - self.center[idx]).powi(2),
                (upper_corner[idx] - self.center[idx]).powi(2),
            );
        }

        max_squared_distance_from_center <= self.radius_squared()
    }

    /// Computes the sphere resulting from scaling this sphere with the given
    /// uniform scale factor.
    pub fn scaled(&self, scale: F) -> Self {
        Self::new(self.center.coords.scale(scale).into(), self.radius * scale)
    }

    /// Computes the sphere resulting from rotating this sphere with the given
    /// rotation quaternion.
    pub fn rotated(&self, rotation: &UnitQuaternion<F>) -> Self {
        Self::new(rotation.transform_point(self.center()), self.radius())
    }

    /// Computes the sphere resulting from transforming this
    /// sphere with the given similarity transform.
    pub fn transformed(&self, transform: &Similarity3<F>) -> Self {
        Self::new(
            transform.transform_point(self.center()),
            transform.scaling() * self.radius(),
        )
    }

    /// Computes the sphere resulting from transforming this
    /// sphere with the given isometry transform.
    pub fn translated_and_rotated(&self, transform: &Isometry3<F>) -> Self {
        Self::new(transform.transform_point(self.center()), self.radius())
    }

    /// Finds the smallest sphere that fully encloses this and
    /// all the given spheres.
    pub fn bounding_sphere_with<I>(self, spheres: impl IntoIterator<Item = I>) -> Self
    where
        I: std::borrow::Borrow<Self>,
    {
        spheres.into_iter().fold(self, |bounding_sphere, sphere| {
            Self::bounding_sphere_from_pair(&bounding_sphere, sphere.borrow())
        })
    }

    /// Computes the circle's axis-aligned bounding box.
    pub fn compute_aabb(&self) -> AxisAlignedBox<F> {
        let radius_vector = vector![self.radius, self.radius, self.radius];
        AxisAlignedBox::new(self.center - radius_vector, self.center + radius_vector)
    }

    fn first_sphere_encloses_second_sphere(
        sphere_1_radius: F,
        sphere_2_radius: F,
        distance_between_centra: F,
    ) -> bool {
        sphere_2_radius + distance_between_centra <= sphere_1_radius
    }
}

roc_integration::impl_roc_for_library_provided_primitives! {
//  Type            Pkg   Parents  Module   Roc name  Postfix      Precision
    Sphere<f32> =>  core, None,    Sphere,  Sphere,   Some("_32"), SinglePrecision,
    Sphere<f64> =>  core, None,    Sphere,  Sphere,   Some("_64"), DoublePrecision,
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_abs_diff_eq;
    use na::vector;
    use nalgebra::point;

    macro_rules! test_bounding_sphere_from_pair {
        (
            sphere_1 = ($center_1:expr, $radius_1:expr),
            sphere_2 = ($center_2:expr, $radius_2:expr),
            bounding_sphere = ($center_bounding:expr, $radius_bounding:expr)
        ) => {{
            let sphere_1 = Sphere::new($center_1, $radius_1);
            let sphere_2 = Sphere::new($center_2, $radius_2);
            let bounding_sphere = Sphere::bounding_sphere_from_pair(&sphere_1, &sphere_2);
            assert_abs_diff_eq!(bounding_sphere.center(), &$center_bounding);
            assert_abs_diff_eq!(bounding_sphere.radius(), $radius_bounding);
            assert!(bounding_sphere.encloses_sphere(&sphere_1));
            assert!(bounding_sphere.encloses_sphere(&sphere_2));
        }};
    }

    #[test]
    fn creating_sphere_works() {
        let center = point![-0.1, 0.0, 123.5];
        let radius = 42.0;
        let sphere = Sphere::new(center, radius);
        assert_eq!(sphere.center(), &center);
        assert_eq!(sphere.radius(), radius);
    }

    #[test]
    #[should_panic]
    fn creating_sphere_with_negative_radius_fails() {
        Sphere::new(point![1.0, 2.0, 3.0], -0.1);
    }

    #[test]
    fn computing_bounding_sphere_works() {
        test_bounding_sphere_from_pair!(
            sphere_1 = (Point3::origin(), 42.0),
            sphere_2 = (Point3::origin(), 42.0),
            bounding_sphere = (Point3::origin(), 42.0)
        );
        test_bounding_sphere_from_pair!(
            sphere_1 = (Point3::origin(), 0.5),
            sphere_2 = (Point3::origin(), 2.0),
            bounding_sphere = (Point3::origin(), 2.0)
        );
        test_bounding_sphere_from_pair!(
            sphere_1 = (point![3.0, 4.0, 0.0], 0.0),
            sphere_2 = (Point3::origin(), 2.0),
            bounding_sphere = (
                point![(3.0 - 6.0 / 5.0) / 2.0, (4.0 - 8.0 / 5.0) / 2.0, 0.0],
                3.5
            )
        );
        test_bounding_sphere_from_pair!(
            sphere_1 = (Point3::origin(), 1.5),
            sphere_2 = (point![1.0, 0.0, 0.0], 2.0),
            bounding_sphere = (point![0.75, 0.0, 0.0], 2.25)
        );
    }

    #[test]
    fn computing_bounding_sphere_from_aabb_works() {
        let lower_corner = point![0.1, 0.2, 0.3];
        let upper_corner = point![2.1, 3.2, 4.3];
        let small_displacement = vector![1e-9, 1e-9, 1e-9];

        let bounding_sphere =
            Sphere::bounding_sphere_from_aabb(&AxisAlignedBox::new(lower_corner, upper_corner));

        assert!(bounding_sphere.contains_point(&(lower_corner + small_displacement)));
        assert!(bounding_sphere.contains_point(&(upper_corner - small_displacement)));
        assert!(!bounding_sphere.contains_point(&(lower_corner - small_displacement)));
        assert!(!bounding_sphere.contains_point(&(upper_corner + small_displacement)));
    }

    #[test]
    fn bounding_sphere_for_single_point_is_correct() {
        let point = point![0.1, 0.2, 0.3];
        let bounding_sphere = Sphere::bounding_sphere_for_points(&[point]);
        assert_abs_diff_eq!(bounding_sphere.center(), &point);
        assert_abs_diff_eq!(bounding_sphere.radius(), 0.0);
    }

    #[test]
    fn bounding_sphere_for_two_points_is_correct() {
        let points = [point![0.1, 0.2, 0.3], point![-0.3, 0.6, 0.7]];
        let bounding_sphere = Sphere::bounding_sphere_for_points(&points);
        assert_abs_diff_eq!(
            bounding_sphere.center(),
            &(points[0].coords + points[1].coords).unscale(2.0).into()
        );
        assert_abs_diff_eq!(
            bounding_sphere.radius(),
            0.5 * na::distance(&points[0], &points[1])
        );
    }

    #[test]
    fn sphere_encloses_itself() {
        let sphere = Sphere::new(point![3.0, 4.3, -0.1], 42.42);
        assert!(sphere.encloses_sphere(&sphere));
    }

    #[test]
    fn bigger_sphere_encloses_smaller_sphere_with_same_center() {
        let center = point![3.0, 4.3, -0.1];
        let smaller_sphere = Sphere::new(center, 1.0);
        let bigger_sphere = Sphere::new(center, 1.0 + f64::EPSILON);
        assert!(bigger_sphere.encloses_sphere(&smaller_sphere));
        assert!(!smaller_sphere.encloses_sphere(&bigger_sphere));
    }

    #[test]
    fn sphere_contains_point_on_surface() {
        let sphere = Sphere::new(Point3::origin(), 3.1);
        let point = point![0.0, 3.1, 0.0];
        assert!(sphere.contains_point(&point));
    }

    #[test]
    fn sphere_contains_point_inside() {
        let sphere = Sphere::new(point![2.14, 0.0, -1.3], 1.0 + f64::EPSILON);
        let point = point![2.14, 1.0, -1.3];
        assert!(sphere.contains_point(&point));
    }

    #[test]
    fn sphere_does_not_contain_point_outside() {
        let sphere = Sphere::new(point![2.14, 0.0, -1.3], 1.0);
        let point = point![2.14, 1.0 + f64::EPSILON, -1.3];
        assert!(!sphere.contains_point(&point));
    }

    #[test]
    fn sphere_outside_aligned_bounding_box_is_outside() {
        let sphere = Sphere::new(Point3::origin(), 1.0);
        let axis_aligned_box = AxisAlignedBox::new(point![2.0, 2.0, 2.0], point![3.0, 4.0, 5.0]);
        assert!(sphere.is_outside_axis_aligned_box(&axis_aligned_box));
    }

    #[test]
    fn sphere_partially_inside_aligned_bounding_box_is_not_outside() {
        let sphere = Sphere::new(point![4.0, 2.0, 2.1], 2.0);
        let axis_aligned_box = AxisAlignedBox::new(point![1.1, 1.2, 1.3], point![3.2, 3.1, 3.0]);
        assert!(!sphere.is_outside_axis_aligned_box(&axis_aligned_box));
    }

    #[test]
    fn sphere_completely_inside_aligned_bounding_box_is_not_outside() {
        let sphere = Sphere::new(point![3.01, 3.06, 3.02], 0.9);
        let axis_aligned_box = AxisAlignedBox::new(point![1.7, 1.6, 1.9], point![4.0, 5.0, 6.0]);
        assert!(!sphere.is_outside_axis_aligned_box(&axis_aligned_box));
    }

    #[test]
    fn sphere_touching_aligned_bounding_box_edges_from_inside_is_not_outside() {
        let sphere = Sphere::new(point![4.0, 4.0, 4.0], 1.0);
        let axis_aligned_box = AxisAlignedBox::new(point![3.0, 3.0, 3.0], point![5.0, 5.0, 5.0]);
        assert!(!sphere.is_outside_axis_aligned_box(&axis_aligned_box));
    }

    #[test]
    fn sphere_touching_aligned_bounding_box_edge_from_outside_is_not_outside() {
        let sphere = Sphere::new(point![3.0, 5.0, 5.0], 1.0);
        let axis_aligned_box = AxisAlignedBox::new(point![4.0, 4.0, 4.0], point![6.0, 6.0, 6.0]);
        assert!(!sphere.is_outside_axis_aligned_box(&axis_aligned_box));
    }

    #[test]
    fn sphere_fully_enclosing_aligned_bounding_box_is_not_outside() {
        let sphere = Sphere::new(point![3.0, 3.0, 3.0], 3.0);
        let axis_aligned_box =
            AxisAlignedBox::new(point![2.2, 2.1, 2.04], point![4.04, 4.06, 4.03]);
        assert!(!sphere.is_outside_axis_aligned_box(&axis_aligned_box));
    }

    #[test]
    fn sphere_enclosing_degenerate_aligned_bounding_box_is_not_outside() {
        let sphere = Sphere::new(point![5.0, 5.0, 5.0], 1.0);
        let axis_aligned_box = AxisAlignedBox::new(point![5.0, 5.0, 5.0], point![5.0, 5.0, 5.0]);
        assert!(!sphere.is_outside_axis_aligned_box(&axis_aligned_box));
    }

    #[test]
    fn degenerate_sphere_inside_aligned_bounding_box_is_not_outside() {
        let sphere = Sphere::new(point![5.0, 5.0, 5.0], 0.0);
        let axis_aligned_box = AxisAlignedBox::new(point![4.0, 4.0, 4.0], point![6.0, 6.0, 6.0]);
        assert!(!sphere.is_outside_axis_aligned_box(&axis_aligned_box));
    }

    #[test]
    fn degenerate_sphere_outside_aligned_bounding_box_is_outside() {
        let sphere = Sphere::new(point![3.0, 3.0, 3.0], 0.0);
        let axis_aligned_box = AxisAlignedBox::new(point![4.0, 4.0, 4.0], point![6.0, 6.0, 6.0]);
        assert!(sphere.is_outside_axis_aligned_box(&axis_aligned_box));
    }

    #[test]
    fn aligned_bounding_box_completely_outside_sphere_is_not_contained() {
        let sphere = Sphere::new(Point3::origin(), 1.0);
        let axis_aligned_box = AxisAlignedBox::new(point![2.0, 2.0, 2.0], point![3.0, 4.0, 5.0]);
        assert!(!sphere.contains_axis_aligned_box(&axis_aligned_box));
    }

    #[test]
    fn aligned_bounding_box_partially_inside_sphere_is_not_contained() {
        let sphere = Sphere::new(point![4.0, 2.0, 2.1], 2.0);
        let axis_aligned_box = AxisAlignedBox::new(point![1.1, 1.2, 1.3], point![3.2, 3.1, 3.0]);
        assert!(!sphere.contains_axis_aligned_box(&axis_aligned_box));
    }

    #[test]
    fn aligned_bounding_box_encompassing_sphere_is_not_contained() {
        let sphere = Sphere::new(point![3.01, 3.06, 3.02], 0.9);
        let axis_aligned_box = AxisAlignedBox::new(point![1.7, 1.6, 1.9], point![4.0, 5.0, 6.0]);
        assert!(!sphere.contains_axis_aligned_box(&axis_aligned_box));
    }

    #[test]
    fn aligned_bounding_box_completely_inside_sphere_is_contained() {
        let sphere = Sphere::new(point![1.0, 1.0, 1.0], 2.0);
        let axis_aligned_box = AxisAlignedBox::new(point![0.0, 0.0, 0.0], point![2.0, 2.0, 2.0]);
        assert!(sphere.contains_axis_aligned_box(&axis_aligned_box));
    }

    #[test]
    fn aligned_bounding_box_barely_inside_sphere_is_contained() {
        let sphere = Sphere::new(point![1.0, 1.0, 1.0], f64::sqrt(3.0) + 1e-9);
        let axis_aligned_box = AxisAlignedBox::new(point![0.0, 0.0, 0.0], point![2.0, 2.0, 2.0]);
        assert!(sphere.contains_axis_aligned_box(&axis_aligned_box));
    }

    #[test]
    fn aligned_bounding_box_barely_outside_sphere_is_not_contained() {
        let sphere = Sphere::new(point![1.0, 1.0, 1.0], f64::sqrt(3.0) - 1e-9);
        let axis_aligned_box = AxisAlignedBox::new(point![0.0, 0.0, 0.0], point![2.0, 2.0, 2.0]);
        assert!(!sphere.contains_axis_aligned_box(&axis_aligned_box));
    }
}
