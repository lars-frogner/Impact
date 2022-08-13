//! Representation of planes.

use super::Sphere;
use crate::num::Float;
use nalgebra::{Point3, UnitVector3};
use num_traits::Signed;

/// A plane in 3D, represented by a unit normal and
/// a displacement.
///
/// The displacement `d` can be determined from the
/// normal `n` and any point `p` lying on the plane
/// as `d = -n.dot(p)`. By storing the displacement
/// instead of the point, we remove redundate degrees
/// of freedom.
///
/// The plane divides space into two halfspaces, the
/// positive and negative halfspace. The positive one
/// is defined as the halfspace the unit normal is
/// pointing into.
#[derive(Clone, Debug)]
pub struct Plane<F: Float> {
    unit_normal: UnitVector3<F>,
    displacement: F,
}

/// How a sphere is positioned relative to a plane.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum SphereRelationToPlane {
    /// The center of the sphere lies strictly in the negative
    /// halfspace of the plane.
    CenterInNegativeHalfspace(IntersectsPlane),
    /// The center of the sphere lies in or on the boundary of
    /// the positive halfspace of the plane.
    CenterInPositiveHalfspace(IntersectsPlane),
}

/// Whether any part of a sphere intersects a plane.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum IntersectsPlane {
    /// Some part of the sphere intersects the plane.
    Yes,
    /// No part of the sphere intersects the plane.
    No,
}

impl<F: Float> Plane<F> {
    /// Creates a new plane defined by the given unit normal
    /// vector and displacement.
    pub fn new(unit_normal: UnitVector3<F>, displacement: F) -> Self {
        Self {
            unit_normal,
            displacement,
        }
    }

    /// Creates a new plane defined by the given unit normal
    /// vector and point in the plane.
    pub fn from_normal_and_point(unit_normal: UnitVector3<F>, point_in_plane: &Point3<F>) -> Self {
        Self::new(
            unit_normal,
            Self::calculate_displacement(&unit_normal, point_in_plane),
        )
    }

    /// Returns the unit normal vector of the plane.
    pub fn unit_normal(&self) -> &UnitVector3<F> {
        &self.unit_normal
    }

    /// Returns the displacement of the plane.
    pub fn displacement(&self) -> F {
        self.displacement
    }

    /// Computes the signed distance from the plane to the given
    /// point. If the signed distance is negative, the point lies
    /// in the negative halfspace of the plane.
    pub fn compute_signed_distance(&self, point: &Point3<F>) -> F {
        self.unit_normal().dot(&point.coords) + self.displacement
    }

    /// Whether the given point is strictly in the positive
    /// halfspace of the plane.
    pub fn point_lies_in_positive_halfspace(&self, point: &Point3<F>) -> bool {
        self.compute_signed_distance(point) > F::zero()
    }

    /// Whether the given point is strictly in the negative
    /// halfspace of the plane.
    pub fn point_lies_in_negative_halfspace(&self, point: &Point3<F>) -> bool {
        self.compute_signed_distance(point) < F::zero()
    }

    /// Determines how the given sphere is positioned relative
    /// to the plane.
    pub fn determine_sphere_relation(&self, sphere: &Sphere<F>) -> SphereRelationToPlane {
        let signed_distance = self.compute_signed_distance(sphere.center());

        let intersects_plane = if <F as Signed>::abs(&signed_distance) < sphere.radius() {
            IntersectsPlane::Yes
        } else {
            IntersectsPlane::No
        };

        if signed_distance.is_negative() {
            SphereRelationToPlane::CenterInNegativeHalfspace(intersects_plane)
        } else {
            SphereRelationToPlane::CenterInPositiveHalfspace(intersects_plane)
        }
    }

    fn calculate_displacement(unit_normal: &UnitVector3<F>, point_in_plane: &Point3<F>) -> F {
        -unit_normal.dot(&point_in_plane.coords)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use approx::assert_abs_diff_eq;
    use nalgebra::{point, vector, Vector3};

    #[test]
    fn creating_plane_through_origin_gives_zero_displacement() {
        let plane = Plane::from_normal_and_point(
            UnitVector3::new_normalize(vector![1.2, -0.1, 2.7]),
            &Point3::origin(),
        );
        assert_abs_diff_eq!(plane.displacement(), 0.0);
    }

    #[test]
    fn signed_distance_is_correct() {
        let plane = Plane::from_normal_and_point(Vector3::y_axis(), &point![1.0, 2.0, 0.0]);
        assert_abs_diff_eq!(
            plane.compute_signed_distance(&point![-1.2, 0.0, 42.4]),
            -2.0
        );
        assert_abs_diff_eq!(
            plane.compute_signed_distance(&point![-2.1, 10.0, 4.42]),
            8.0
        );

        let plane = Plane::from_normal_and_point(
            UnitVector3::new_normalize(vector![1.0, 0.0, 1.0]),
            &Point3::origin(),
        );
        assert_abs_diff_eq!(
            plane.compute_signed_distance(&point![8.0, 0.0, 8.0]),
            std::f64::consts::SQRT_2 * 8.0,
            epsilon = 1e-9
        );
        assert_abs_diff_eq!(plane.compute_signed_distance(&point![0.0, 8.0, 0.0]), 0.0);
    }
}
