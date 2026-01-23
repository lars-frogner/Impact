//! Representation of capsules.

use crate::{AxisAlignedBox, Sphere};
use approx::AbsDiffEq;
use bytemuck::{Pod, Zeroable};
use impact_math::{
    point::{Point3, Point3C},
    transform::{Isometry3, Similarity3},
    vector::{Vector3, Vector3C},
};

/// A capsule represented by the starting point and displacement vector of the
/// segment making up the central axis of the cylinder between the caps, as well
/// as a radius.
///
/// The segment start and segment vector are stored in 128-bit SIMD registers
/// for efficient computation. That leads to an extra 20 bytes in size (4 each
/// due to the padded point and vector and 12 due to padding after the radius)
/// and 16-byte alignment. For cache-friendly storage, prefer the compact 4-byte
/// aligned [`CapsuleC`].
#[derive(Clone, Debug, PartialEq)]
pub struct Capsule {
    segment_start: Point3,
    segment_vector: Vector3,
    radius: f32,
}

/// Helper for testing whether a capsule contains a point. Useful for
/// efficiently testing many points without unneccesary recomputation of
/// intermediate quantities.
#[derive(Clone, Debug)]
pub struct CapsulePointContainmentTester {
    segment_start: Point3,
    segment_vector: Vector3,
    segment_vector_over_length_squared: Vector3,
    radius_squared: f32,
}

/// A capsule represented by the starting point and displacement vector of the
/// segment making up the central axis of the cylinder between the caps, as well
/// as a radius. This is the "compact" version.
///
/// This type only supports a few basic operations, as is primarily intended for
/// compact storage inside other types and collections. For computations, prefer
/// the SIMD-friendly 16-byte aligned [`Capsule`].
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct CapsuleC {
    segment_start: Point3C,
    segment_vector: Vector3C,
    radius: f32,
}

impl Capsule {
    /// Creates a new capsule with the given segment starting point, segment
    /// vector and radius.
    ///
    /// # Panics
    /// If `radius` is negative.
    #[inline]
    pub fn new(segment_start: Point3, segment_vector: Vector3, radius: f32) -> Self {
        assert!(radius >= 0.0);
        Self {
            segment_start,
            segment_vector,
            radius,
        }
    }

    /// Returns the starting point of the line segment making up the central
    /// axis of the cylinder between the caps.
    #[inline]
    pub fn segment_start(&self) -> &Point3 {
        &self.segment_start
    }

    /// Returns the end point of the line segment making up the central axis of
    /// the cylinder between the caps.
    #[inline]
    pub fn segment_end(&self) -> Point3 {
        self.segment_start + self.segment_vector
    }

    /// Returns the displacement vector between the end points of the line
    /// segment making up the central axis of the cylinder between the caps.
    #[inline]
    pub fn segment_vector(&self) -> &Vector3 {
        &self.segment_vector
    }

    /// Returns the radius of the capsule.
    #[inline]
    pub fn radius(&self) -> f32 {
        self.radius
    }

    /// Computes the capsule resulting from scaling this capsule with the given
    /// uniform scale factor.
    #[inline]
    pub fn scaled(&self, scale: f32) -> Self {
        Self::new(
            scale * self.segment_start,
            scale * self.segment_vector,
            scale * self.radius,
        )
    }

    /// Computes the capsule resulting from transforming this capsule with the
    /// given similarity transform.
    #[inline]
    pub fn transformed(&self, transform: &Similarity3) -> Self {
        Self::new(
            transform.transform_point(self.segment_start()),
            transform.transform_vector(self.segment_vector()),
            transform.scaling() * self.radius(),
        )
    }

    /// Computes the capsule resulting from transforming this capsule with the
    /// given isometry transform.
    #[inline]
    pub fn iso_transformed(&self, transform: &Isometry3) -> Self {
        Self::new(
            transform.transform_point(self.segment_start()),
            transform.transform_vector(self.segment_vector()),
            self.radius(),
        )
    }

    /// Computes the capsule's axis-aligned bounding box.
    #[inline]
    pub fn compute_aabb(&self) -> AxisAlignedBox {
        AxisAlignedBox::aabb_from_pair(
            &Sphere::new(*self.segment_start(), self.radius).compute_aabb(),
            &Sphere::new(self.segment_end(), self.radius).compute_aabb(),
        )
    }

    /// Trims off most of this capsule's segment for the parts of the capsule
    /// that do not touch the given axis-aligned box. If the capsule does not
    /// intersect the box, returns [`None`]. Otherwise, the returned capsule is
    /// guaranteed to cover all parts of the box that the un-trimmed capsule
    /// covers, but parts of the trimmed capsule may still be outside the box.
    pub fn trim_segment_outside_aab(&self, aab: &AxisAlignedBox) -> Option<Self> {
        // If we expand the box by the capsule radius on all sides, we can trim
        // away the parts of the segment outside the expanded box. If the
        // segment does not intersect the expanded box, the capsule does not
        // touch the un-expanded box. Otherwise, we retain all the parts of the
        // capsule touching the box, but we may retain a bit more than necessary
        // at the ends due to the expansion.
        let expanded_aab = aab.expanded_about_center(self.radius);

        let (t_min, t_max) =
            expanded_aab.find_contained_subsegment(&self.segment_start, &self.segment_vector)?;

        let trimmed_segment_start = self.segment_start + self.segment_vector * t_min;
        let trimmed_segment_vector = self.segment_vector * (t_max - t_min);

        Some(Self::new(
            trimmed_segment_start,
            trimmed_segment_vector,
            self.radius,
        ))
    }

    /// Returns a new point containment tester for the capsule.
    #[inline]
    pub fn create_point_containment_tester(&self) -> CapsulePointContainmentTester {
        CapsulePointContainmentTester {
            segment_start: self.segment_start,
            segment_vector: self.segment_vector,
            segment_vector_over_length_squared: self.segment_vector
                / self.segment_vector.norm_squared(),
            radius_squared: self.radius.powi(2),
        }
    }

    /// Converts the capsule to the 4-byte aligned cache-friendly [`CapsuleC`].
    #[inline]
    pub fn compact(&self) -> CapsuleC {
        CapsuleC::new(
            self.segment_start.compact(),
            self.segment_vector.compact(),
            self.radius,
        )
    }
}

impl AbsDiffEq for Capsule {
    type Epsilon = f32;

    fn default_epsilon() -> f32 {
        f32::default_epsilon()
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: f32) -> bool {
        self.segment_start
            .abs_diff_eq(&other.segment_start, epsilon)
            && self
                .segment_vector
                .abs_diff_eq(&other.segment_vector, epsilon)
            && self.radius.abs_diff_eq(&other.radius, epsilon)
    }
}

impl CapsulePointContainmentTester {
    /// Whether the capsule contains the given point. Returns `true` if the
    /// point lies exactly on the capsule boundary.
    #[inline]
    pub fn contains_point(&self, point: &Point3) -> bool {
        self.shortest_squared_distance_from_point_to_segment_if_contained(point)
            .is_some()
    }

    /// Returns an option containing the square of the shortest distance between
    /// the given point and the capsule's segment if the point lines within or
    /// on the boundary of the capsule, or [`None`] if the point is outside the
    /// capsule.
    #[inline]
    pub fn shortest_squared_distance_from_point_to_segment_if_contained(
        &self,
        point: &Point3,
    ) -> Option<f32> {
        let shortest_squared_distance = self.shortest_squared_distance_from_point_to_segment(point);
        if shortest_squared_distance <= self.radius_squared {
            Some(shortest_squared_distance)
        } else {
            None
        }
    }

    /// Returns the square of the shortest distance between the given point and
    /// the capsule's segment.
    #[inline]
    pub fn shortest_squared_distance_from_point_to_segment(&self, point: &Point3) -> f32 {
        let segment_start_to_point = point - self.segment_start;

        let t = segment_start_to_point
            .dot(&self.segment_vector_over_length_squared)
            .clamp(0.0, 1.0);

        let closest_point_on_segment = self.segment_start + self.segment_vector * t;

        (point - closest_point_on_segment).norm_squared()
    }
}

impl CapsuleC {
    /// Creates a new capsule with the given segment starting point, segment
    /// vector and radius.
    ///
    /// # Panics
    /// If `radius` is negative.
    #[inline]
    pub const fn new(segment_start: Point3C, segment_vector: Vector3C, radius: f32) -> Self {
        assert!(radius >= 0.0);
        Self {
            segment_start,
            segment_vector,
            radius,
        }
    }

    /// Returns the starting point of the line segment making up the central
    /// axis of the cylinder between the caps.
    #[inline]
    pub const fn segment_start(&self) -> &Point3C {
        &self.segment_start
    }

    /// Returns the end point of the line segment making up the central axis of
    /// the cylinder between the caps.
    #[inline]
    pub fn segment_end(&self) -> Point3C {
        self.segment_start + self.segment_vector
    }

    /// Returns the displacement vector between the end points of the line
    /// segment making up the central axis of the cylinder between the caps.
    #[inline]
    pub const fn segment_vector(&self) -> &Vector3C {
        &self.segment_vector
    }

    /// Returns the radius of the capsule.
    #[inline]
    pub const fn radius(&self) -> f32 {
        self.radius
    }

    /// Converts the capsule to the 16-byte aligned SIMD-friendly [`Capsule`].
    #[inline]
    pub fn aligned(&self) -> Capsule {
        Capsule::new(
            self.segment_start.aligned(),
            self.segment_vector.aligned(),
            self.radius,
        )
    }
}

impl AbsDiffEq for CapsuleC {
    type Epsilon = f32;

    fn default_epsilon() -> f32 {
        f32::default_epsilon()
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: f32) -> bool {
        self.segment_start
            .abs_diff_eq(&other.segment_start, epsilon)
            && self
                .segment_vector
                .abs_diff_eq(&other.segment_vector, epsilon)
            && self.radius.abs_diff_eq(&other.radius, epsilon)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AxisAlignedBox;
    use approx::assert_abs_diff_eq;
    use impact_math::{consts::f32::FRAC_PI_2, quaternion::UnitQuaternion, vector::UnitVector3};

    #[test]
    #[should_panic]
    fn creating_capsule_with_negative_radius_fails() {
        let segment_start = Point3::origin();
        let segment_vector = Vector3::unit_x();
        Capsule::new(segment_start, segment_vector, -1.0);
    }

    #[test]
    fn computes_correct_segment_end() {
        let segment_start = Point3::new(0.0, 0.0, 0.0);
        let segment_vector = Vector3::unit_x() * 5.0;
        let radius = 1.0;

        let capsule = Capsule::new(segment_start, segment_vector, radius);

        let expected_segment_end = Point3::new(5.0, 0.0, 0.0);
        assert_abs_diff_eq!(capsule.segment_end(), expected_segment_end);
    }

    #[test]
    fn translating_capsule_works() {
        let segment_start = Point3::origin();
        let segment_vector = Vector3::unit_z() * 3.0;
        let radius = 0.5;

        let capsule = Capsule::new(segment_start, segment_vector, radius);

        let translation = Vector3::new(2.0, 3.0, 4.0);
        let transform = Similarity3::from_translation(translation);

        let transformed_capsule = capsule.transformed(&transform);

        let expected_segment_start = Point3::new(2.0, 3.0, 4.0);
        let expected_segment_vector = Vector3::unit_z() * 3.0;

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
        let segment_vector = Vector3::unit_x() * 5.0;
        let radius = 1.0;

        let capsule = Capsule::new(segment_start, segment_vector, radius);

        let rotation = UnitQuaternion::from_axis_angle(&UnitVector3::unit_z(), FRAC_PI_2);
        let transform = Similarity3::from_rotation(rotation);

        let transformed_capsule = capsule.transformed(&transform);

        let expected_segment_start = Point3::origin();
        let expected_segment_end = Point3::new(0.0, 5.0, 0.0);
        let expected_segment_vector = Vector3::unit_y() * 5.0;

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
        let segment_vector = Vector3::unit_y() * 5.0;
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
