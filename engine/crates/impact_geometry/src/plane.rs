//! Representation of planes.

use crate::Sphere;
use approx::AbsDiffEq;
use bytemuck::{Pod, Zeroable};
use impact_math::transform::{Isometry3, Similarity3};
use nalgebra::{Point3, UnitQuaternion, UnitVector3, Vector3};
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
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct Plane {
    unit_normal: UnitVector3<f32>,
    displacement: f32,
}

/// How a sphere is positioned relative to a plane.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SphereRelationToPlane {
    /// The center of the sphere lies strictly in the negative
    /// halfspace of the plane.
    CenterInNegativeHalfspace(IntersectsPlane),
    /// The center of the sphere lies in or on the boundary of
    /// the positive halfspace of the plane.
    CenterInPositiveHalfspace(IntersectsPlane),
}

/// Whether any part of a sphere intersects a plane.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum IntersectsPlane {
    /// Some part of the sphere intersects the plane.
    Yes,
    /// No part of the sphere intersects the plane.
    No,
}

impl Plane {
    /// The xy-coordinate plane, with the positive halfspace being the space of
    /// positive z-coordinates.
    pub const XY_PLANE: Self =
        Self::new(UnitVector3::new_unchecked(Vector3::new(0.0, 0.0, 1.0)), 0.0);

    /// The yz-coordinate plane, with the positive halfspace being the space of
    /// positive x-coordinates.
    pub const YZ_PLANE: Self =
        Self::new(UnitVector3::new_unchecked(Vector3::new(1.0, 0.0, 0.0)), 0.0);

    /// The xz-coordinate plane, with the positive halfspace being the space of
    /// positive y-coordinates.
    pub const XZ_PLANE: Self =
        Self::new(UnitVector3::new_unchecked(Vector3::new(0.0, 1.0, 0.0)), 0.0);

    /// Creates a new plane defined by the given unit normal
    /// vector and displacement.
    pub const fn new(unit_normal: UnitVector3<f32>, displacement: f32) -> Self {
        Self {
            unit_normal,
            displacement,
        }
    }

    /// Creates a new plane defined by the given unit normal
    /// vector and point in the plane.
    pub fn from_normal_and_point(
        unit_normal: UnitVector3<f32>,
        point_in_plane: &Point3<f32>,
    ) -> Self {
        Self::new(
            unit_normal,
            Self::calculate_displacement(&unit_normal, point_in_plane),
        )
    }

    /// Returns the unit normal vector of the plane.
    pub fn unit_normal(&self) -> &UnitVector3<f32> {
        &self.unit_normal
    }

    /// Returns the displacement of the plane.
    pub fn displacement(&self) -> f32 {
        self.displacement
    }

    /// Computes the signed distance from the plane to the given
    /// point. If the signed distance is negative, the point lies
    /// in the negative halfspace of the plane.
    pub fn compute_signed_distance(&self, point: &Point3<f32>) -> f32 {
        self.unit_normal().dot(&point.coords) - self.displacement
    }

    /// Whether the given point is strictly in the positive
    /// halfspace of the plane.
    pub fn point_lies_in_positive_halfspace(&self, point: &Point3<f32>) -> bool {
        self.compute_signed_distance(point) > 0.0
    }

    /// Whether the given point is strictly in the negative
    /// halfspace of the plane.
    pub fn point_lies_in_negative_halfspace(&self, point: &Point3<f32>) -> bool {
        self.compute_signed_distance(point) < 0.0
    }

    /// Returns the projection of the given point onto this plane.
    pub fn project_point_onto_plane(&self, point: &Point3<f32>) -> Point3<f32> {
        let signed_distance = self.compute_signed_distance(point);
        point - self.unit_normal.scale(signed_distance)
    }

    /// Determines how the given sphere is positioned relative
    /// to the plane.
    pub fn determine_sphere_relation(&self, sphere: &Sphere) -> SphereRelationToPlane {
        let signed_distance = self.compute_signed_distance(sphere.center());

        let intersects_plane = if signed_distance.abs() < sphere.radius() {
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

    /// Computes the plane resulting from scaling this plane with the given
    /// uniform scale factor.
    pub fn scaled(&self, scale: f32) -> Self {
        Self::new(self.unit_normal, self.displacement * scale)
    }

    /// Computes the plane resulting from rotating this plane with the given
    /// rotation quaternion.
    pub fn rotated(&self, rotation: &UnitQuaternion<f32>) -> Self {
        let rotated_unit_normal =
            UnitVector3::new_unchecked(rotation.transform_vector(&self.unit_normal));
        Self::new(rotated_unit_normal, self.displacement)
    }

    /// Computes the plane resulting from transforming this plane with the given
    /// similarity transform.
    pub fn transformed(&self, transform: &Similarity3) -> Self {
        let point_in_plane = Point3::from(self.unit_normal.as_ref() * self.displacement);
        let transformed_point_in_plane = transform.transform_point(&point_in_plane);
        let transformed_unit_normal =
            UnitVector3::new_unchecked(transform.rotation().transform_vector(&self.unit_normal));
        Self::from_normal_and_point(transformed_unit_normal, &transformed_point_in_plane)
    }

    /// Computes the plane resulting from transforming this plane with the given
    /// isometry transform.
    pub fn translated_and_rotated(&self, transform: &Isometry3) -> Self {
        let point_in_plane = Point3::from(self.unit_normal.as_ref() * self.displacement);
        let transformed_point_in_plane = transform.transform_point(&point_in_plane);
        let transformed_unit_normal =
            UnitVector3::new_unchecked(transform.rotation().transform_vector(&self.unit_normal));
        Self::from_normal_and_point(transformed_unit_normal, &transformed_point_in_plane)
    }

    /// Deconstructs the plane into its unit normal and displacement.
    pub fn into_normal_and_displacement(self) -> (UnitVector3<f32>, f32) {
        (self.unit_normal, self.displacement)
    }

    fn calculate_displacement(unit_normal: &UnitVector3<f32>, point_in_plane: &Point3<f32>) -> f32 {
        unit_normal.dot(&point_in_plane.coords)
    }
}

impl AbsDiffEq for Plane {
    type Epsilon = f32;

    fn default_epsilon() -> f32 {
        f32::default_epsilon()
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: f32) -> bool {
        self.unit_normal.abs_diff_eq(&other.unit_normal, epsilon)
            && self.displacement.abs_diff_eq(&other.displacement, epsilon)
    }
}

roc_integration::impl_roc_for_library_provided_primitives! {
//  Type            Pkg   Parents  Module   Roc name  Postfix      Precision
    Plane =>        core, None,    Plane,   Plane,    None,        PrecisionIrrelevant,
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_abs_diff_eq;
    use impact_math::consts::f32::SQRT_2;
    use nalgebra::Vector3;

    #[test]
    fn creating_plane_through_origin_gives_zero_displacement() {
        let plane = Plane::from_normal_and_point(
            UnitVector3::new_normalize(Vector3::new(1.2, -0.1, 2.7)),
            &Point3::origin(),
        );
        assert_abs_diff_eq!(plane.displacement(), 0.0);
    }

    #[test]
    fn signed_distance_is_correct() {
        let plane = Plane::from_normal_and_point(Vector3::y_axis(), &Point3::new(1.0, 2.0, 0.0));
        assert_abs_diff_eq!(
            plane.compute_signed_distance(&Point3::new(-1.2, 0.0, 42.4)),
            -2.0
        );
        assert_abs_diff_eq!(
            plane.compute_signed_distance(&Point3::new(-2.1, 10.0, 4.42)),
            8.0
        );

        let plane = Plane::from_normal_and_point(
            UnitVector3::new_normalize(Vector3::new(1.0, 0.0, 1.0)),
            &Point3::origin(),
        );
        assert_abs_diff_eq!(
            plane.compute_signed_distance(&Point3::new(8.0, 0.0, 8.0)),
            SQRT_2 * 8.0,
            epsilon = 1e-6
        );
        assert_abs_diff_eq!(
            plane.compute_signed_distance(&Point3::new(0.0, 8.0, 0.0)),
            0.0
        );
    }

    #[test]
    fn transforming_plane_with_identity_gives_same_plane() {
        let plane = Plane::new(
            UnitVector3::new_normalize(Vector3::new(1.2, -0.1, 2.7)),
            -3.4,
        );
        let transformed_plane = plane.transformed(&Similarity3::identity());

        assert_abs_diff_eq!(transformed_plane, plane, epsilon = 1e-9);
    }

    #[test]
    fn projecting_point_on_plane_returns_same_point() {
        let plane = Plane::from_normal_and_point(Vector3::y_axis(), &Point3::new(1.0, 2.0, 0.0));
        let point_on_plane = Point3::new(5.0, 2.0, -3.0);
        let projected_point = plane.project_point_onto_plane(&point_on_plane);

        assert_abs_diff_eq!(projected_point, point_on_plane, epsilon = 1e-9);
    }

    #[test]
    fn projecting_point_off_plane_moves_it_to_plane() {
        let plane = Plane::from_normal_and_point(Vector3::y_axis(), &Point3::new(0.0, 5.0, 0.0));
        let point_off_plane = Point3::new(2.0, 8.0, -1.0);
        let projected_point = plane.project_point_onto_plane(&point_off_plane);

        // The projected point should be on the plane (y = 5.0)
        assert_abs_diff_eq!(projected_point, Point3::new(2.0, 5.0, -1.0), epsilon = 1e-9);

        // Verify the projected point is actually on the plane
        assert_abs_diff_eq!(
            plane.compute_signed_distance(&projected_point),
            0.0,
            epsilon = 1e-9
        );
    }
}
