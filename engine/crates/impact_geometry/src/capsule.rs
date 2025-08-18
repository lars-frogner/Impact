//! Representation of capsules.

use crate::{AxisAlignedBox, Sphere};
use impact_math::Float;
use nalgebra::{Isometry3, Point3, Similarity3, Vector3};

/// A capsule represented by the starting point and displacement vector of the
/// segment making up the central axis of the cylinder between the caps, as well
/// as a radius.
#[derive(Clone, Debug)]
pub struct Capsule<F: Float> {
    segment_start: Point3<F>,
    segment_vector: Vector3<F>,
    radius: F,
}

/// Helper for testing whether a capsule contains a point. Useful for
/// efficiently testing many points without unneccesary recomputation of
/// intermediate quantities.
#[derive(Clone, Debug)]
pub struct CapsulePointContainmentTester<F: Float> {
    segment_start: Point3<F>,
    segment_vector: Vector3<F>,
    segment_vector_over_length_squared: Vector3<F>,
    radius_squared: F,
}

impl<F: Float> Capsule<F> {
    /// Creates a new capsule with the given segment starting point, segment
    /// vector and radius.
    ///
    /// # Panics
    /// If `radius` is negative.
    pub fn new(segment_start: Point3<F>, segment_vector: Vector3<F>, radius: F) -> Self {
        assert!(radius >= F::zero());
        Self {
            segment_start,
            segment_vector,
            radius,
        }
    }

    /// Returns the starting point of the line segment making up the central
    /// axis of the cylinder between the caps.
    pub fn segment_start(&self) -> &Point3<F> {
        &self.segment_start
    }

    /// Returns the end point of the line segment making up the central axis of
    /// the cylinder between the caps.
    pub fn segment_end(&self) -> Point3<F> {
        self.segment_start + self.segment_vector
    }

    /// Returns the displacement vector between the end points of the line
    /// segment making up the central axis of the cylinder between the caps.
    pub fn segment_vector(&self) -> &Vector3<F> {
        &self.segment_vector
    }

    /// Returns the radius of the capsule.
    pub fn radius(&self) -> F {
        self.radius
    }

    /// Computes the capsule resulting from scaling this capsule with the given
    /// uniform scale factor.
    pub fn scaled(&self, scale: F) -> Self {
        Self::new(
            self.segment_start.coords.scale(scale).into(),
            self.segment_vector.scale(scale),
            self.radius * scale,
        )
    }

    /// Computes the capsule resulting from transforming this capsule with the
    /// given similarity transform.
    pub fn transformed(&self, transform: &Similarity3<F>) -> Self {
        Self::new(
            transform.transform_point(self.segment_start()),
            transform.transform_vector(self.segment_vector()),
            transform.scaling() * self.radius(),
        )
    }

    /// Computes the capsule resulting from transforming this capsule with the
    /// given isometry transform.
    pub fn translated_and_rotated(&self, transform: &Isometry3<F>) -> Self {
        Self::new(
            transform.transform_point(self.segment_start()),
            transform.transform_vector(self.segment_vector()),
            self.radius(),
        )
    }

    /// Computes the capsule's axis-aligned bounding box.
    pub fn compute_aabb(&self) -> AxisAlignedBox<F> {
        AxisAlignedBox::aabb_from_pair(
            &Sphere::new(*self.segment_start(), self.radius).compute_aabb(),
            &Sphere::new(self.segment_end(), self.radius).compute_aabb(),
        )
    }

    /// Computes the capsule obtained by clamping this capsule's segment to the
    /// bounds of the given axis-aligned box. Returns [`None`] if the segment
    /// lies completely outside the box.
    pub fn with_segment_clamped_to_aab(&self, aab: &AxisAlignedBox<F>) -> Option<Self> {
        let (t_min, t_max) =
            aab.find_contained_subsegment(self.segment_start, self.segment_vector)?;
        let clamped_segment_start = self.segment_start + self.segment_vector * t_min;
        let clamped_segment_vector = self.segment_vector * (t_max - t_min);
        Some(Self::new(
            clamped_segment_start,
            clamped_segment_vector,
            self.radius,
        ))
    }

    /// Returns a new point containment tester for the capsule.
    pub fn create_point_containment_tester(&self) -> CapsulePointContainmentTester<F> {
        CapsulePointContainmentTester {
            segment_start: self.segment_start,
            segment_vector: self.segment_vector,
            segment_vector_over_length_squared: self.segment_vector
                / self.segment_vector.norm_squared(),
            radius_squared: self.radius.powi(2),
        }
    }
}

impl<F: Float> CapsulePointContainmentTester<F> {
    /// Whether the capsule contains the given point. Returns `true` if the
    /// point lies exactly on the capsule boundary.
    pub fn contains_point(&self, point: &Point3<F>) -> bool {
        self.shortest_squared_distance_from_point_to_segment_if_contained(point)
            .is_some()
    }

    /// Returns an option containing the square of the shortest distance between
    /// the given point and the capsule's segment if the point lines within or
    /// on the boundary of the capsule, or [`None`] if the point is outside the
    /// capsule.
    pub fn shortest_squared_distance_from_point_to_segment_if_contained(
        &self,
        point: &Point3<F>,
    ) -> Option<F> {
        let shortest_squared_distance = self.shortest_squared_distance_from_point_to_segment(point);
        if shortest_squared_distance <= self.radius_squared {
            Some(shortest_squared_distance)
        } else {
            None
        }
    }

    /// Returns the square of the shortest distance between the given point and
    /// the capsule's segment.
    pub fn shortest_squared_distance_from_point_to_segment(&self, point: &Point3<F>) -> F {
        let segment_start_to_point = point - self.segment_start;

        let t = segment_start_to_point
            .dot(&self.segment_vector_over_length_squared)
            .clamp(F::ZERO, F::ONE);

        let closest_point_on_segment = self.segment_start + self.segment_vector * t;

        (point - closest_point_on_segment).norm_squared()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AxisAlignedBox;
    use approx::assert_abs_diff_eq;
    use nalgebra::{Translation3, UnitQuaternion, Vector3};

    #[test]
    #[should_panic]
    fn creating_capsule_with_negative_radius_fails() {
        let segment_start = Point3::origin();
        let segment_vector = Vector3::x();
        Capsule::new(segment_start, segment_vector, -1.0);
    }

    #[test]
    fn computes_correct_segment_end() {
        let segment_start = Point3::new(0.0, 0.0, 0.0);
        let segment_vector = Vector3::x() * 5.0;
        let radius = 1.0;

        let capsule = Capsule::new(segment_start, segment_vector, radius);

        let expected_segment_end = Point3::new(5.0, 0.0, 0.0);
        assert_abs_diff_eq!(capsule.segment_end(), expected_segment_end);
    }

    #[test]
    fn translating_capsule_works() {
        let segment_start = Point3::origin();
        let segment_vector = Vector3::z() * 3.0;
        let radius = 0.5;

        let capsule = Capsule::new(segment_start, segment_vector, radius);

        let translation = Translation3::new(2.0, 3.0, 4.0);
        let transform = Similarity3::from_parts(translation, UnitQuaternion::identity(), 1.0);

        let transformed_capsule = capsule.transformed(&transform);

        let expected_segment_start = Point3::new(2.0, 3.0, 4.0);
        let expected_segment_vector = Vector3::z() * 3.0;

        assert_abs_diff_eq!(*transformed_capsule.segment_start(), expected_segment_start);
        assert_eq!(
            transformed_capsule.segment_vector(),
            &expected_segment_vector
        );
        assert_eq!(transformed_capsule.radius(), radius);
    }

    #[test]
    fn rotating_capsule_works() {
        let segment_start = Point3::origin();
        let segment_vector = Vector3::x() * 5.0;
        let radius = 1.0;

        let capsule = Capsule::new(segment_start, segment_vector, radius);

        let rotation =
            UnitQuaternion::from_axis_angle(&Vector3::z_axis(), std::f32::consts::FRAC_PI_2);
        let transform = Similarity3::from_parts(Translation3::identity(), rotation, 1.0);

        let transformed_capsule = capsule.transformed(&transform);

        let expected_segment_start = Point3::origin();
        let expected_segment_end = Point3::new(0.0, 5.0, 0.0);
        let expected_segment_vector = Vector3::y() * 5.0;

        assert_abs_diff_eq!(
            *transformed_capsule.segment_start(),
            expected_segment_start,
            epsilon = 1e-6,
        );
        assert_abs_diff_eq!(
            transformed_capsule.segment_end(),
            expected_segment_end,
            epsilon = 1e-6,
        );
        assert_abs_diff_eq!(
            transformed_capsule.segment_vector(),
            &expected_segment_vector,
            epsilon = 1e-6,
        );
    }

    #[test]
    fn computes_correct_aabb_for_vertical_capsule() {
        let segment_start = Point3::origin();
        let segment_vector = Vector3::y() * 5.0;
        let radius = 2.0;

        let capsule = Capsule::new(segment_start, segment_vector, radius);

        let expected_min = Point3::new(-2.0, -2.0, -2.0);
        let expected_max = Point3::new(2.0, 7.0, 2.0);
        let expected_aabb = AxisAlignedBox::new(expected_min, expected_max);

        let computed_aabb = capsule.compute_aabb();
        assert_abs_diff_eq!(computed_aabb, expected_aabb);
    }

    #[test]
    fn computes_correct_aabb_for_diagonal_capsule() {
        let segment_start = Point3::origin();
        let segment_vector = Vector3::new(1.0, 1.0, 1.0);
        let radius = 0.5;

        let capsule = Capsule::new(segment_start, segment_vector, radius);

        let expected_min = Point3::new(-0.5, -0.5, -0.5);
        let expected_max = Point3::new(1.5, 1.5, 1.5);
        let expected_aabb = AxisAlignedBox::new(expected_min, expected_max);

        let computed_aabb = capsule.compute_aabb();
        assert_abs_diff_eq!(computed_aabb, expected_aabb);
    }

    #[test]
    fn computes_correct_aabb_for_zero_segment_length_capsule() {
        let segment_start = Point3::new(1.0, 1.0, 1.0);
        let segment_vector = Vector3::zeros();
        let radius = 1.0;

        let capsule = Capsule::new(segment_start, segment_vector, radius);

        assert_abs_diff_eq!(capsule.segment_start(), &capsule.segment_end());

        let expected_min = Point3::new(0.0, 0.0, 0.0);
        let expected_max = Point3::new(2.0, 2.0, 2.0);
        let expected_aabb = AxisAlignedBox::new(expected_min, expected_max);

        let computed_aabb = capsule.compute_aabb();
        assert_abs_diff_eq!(computed_aabb, expected_aabb);
    }
}
