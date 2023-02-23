//! Representation of frustums.

use crate::{
    geometry::{
        plane::{IntersectsPlane, SphereRelationToPlane},
        AxisAlignedBox, Plane, Sphere,
    },
    num::Float,
};
use approx::AbsDiffEq;
use nalgebra::{
    self as na, point, vector, Matrix4, Point3, Projective3, Similarity3, UnitQuaternion,
    UnitVector3, Vector3,
};

/// A frustum, which in general is a pyramid truncated at the
/// top. It is here represented by the six planes making up
/// the faces of the truncated pyramid.
///
/// The planes are created in such a way that their negative
/// halfspaces correspond to the space outside the frustum.
#[derive(Clone, Debug, PartialEq)]
pub struct Frustum<F: Float> {
    planes: [Plane<F>; 6],
    transform_matrix: Matrix4<F>,
    inverse_transform_matrix: Matrix4<F>,
}

impl<F: Float> Frustum<F> {
    /// Creates the frustum representing the clip space of the
    /// given transform.
    ///
    /// This function uses the method of Gribb and Hartmann (2001)
    /// "Fast Extraction of Viewing Frustum Planes from the
    /// World-View-Projection Matrix".
    pub fn from_transform(transform: &Projective3<F>) -> Self {
        Self {
            planes: Self::planes_from_transform_matrix(transform.matrix()),
            transform_matrix: transform.to_homogeneous(),
            inverse_transform_matrix: transform.inverse().to_homogeneous(),
        }
    }

    /// Creates the frustum representing the clip space of the identity
    /// transform, which is the cube spanning from -1 to 1 in each dimension.
    pub fn for_identity_transform() -> Self {
        Self {
            planes: [
                Plane::new(Vector3::x_axis(), F::ONE),
                Plane::new(-Vector3::x_axis(), F::ONE),
                Plane::new(Vector3::y_axis(), F::ONE),
                Plane::new(-Vector3::y_axis(), F::ONE),
                Plane::new(Vector3::z_axis(), F::ONE),
                Plane::new(-Vector3::z_axis(), F::ONE),
            ],
            transform_matrix: Matrix4::identity(),
            inverse_transform_matrix: Matrix4::identity(),
        }
    }

    /// Returns the plane defining the left face of the frustum.
    pub fn left_plane(&self) -> &Plane<F> {
        &self.planes[0]
    }

    /// Returns the plane defining the right face of the frustum.
    pub fn right_plane(&self) -> &Plane<F> {
        &self.planes[1]
    }

    /// Returns the plane defining the bottom face of the frustum.
    pub fn bottom_plane(&self) -> &Plane<F> {
        &self.planes[2]
    }

    /// Returns the plane defining the top face of the frustum.
    pub fn top_plane(&self) -> &Plane<F> {
        &self.planes[3]
    }

    /// Returns the near plane of the frustum.
    pub fn near_plane(&self) -> &Plane<F> {
        &self.planes[4]
    }

    /// Returns the far plane of the frustum.
    pub fn far_plane(&self) -> &Plane<F> {
        &self.planes[5]
    }

    /// Returns the matrix of the transform into the clip space
    /// that this frustum represents.
    pub fn transform_matrix(&self) -> &Matrix4<F> {
        &self.transform_matrix
    }

    /// Whether the given point is strictly inside the frustum.
    pub fn contains_point(&self, point: &Point3<F>) -> bool {
        self.planes
            .iter()
            .all(|plane| plane.point_lies_in_positive_halfspace(point))
    }

    /// Whether all of the given sphere is strictly outside the frustum.
    pub fn sphere_lies_outside(&self, sphere: &Sphere<F>) -> bool {
        let mut intersects_from_negative_halfspace = [false, false, false];

        // For every plane that the sphere intersects and its
        // center lies in the negative halfspace, we will set
        // the corresponding coordinate in this point to the
        // offset of that plane along its normal in normalized
        // device coordinates, giving us a way to quickly find
        // the point on the frustum closest to the sphere
        let mut closest_point_ndc = Point3::origin();

        for (plane, (plane_axis, plane_offset_ndc)) in
            self.planes.iter().zip(Self::CUBE_PLANES_NDC.into_iter())
        {
            // If we already know that the sphere center lies in the
            // negative halfspace of the opposite plane, there is no
            // reason to test the sphere against this plane
            if intersects_from_negative_halfspace[plane_axis] {
                continue;
            }
            match plane.determine_sphere_relation(sphere) {
                // If all of the sphere lies in the negative halfspace
                // of any frustum plane is is sure to be outside
                SphereRelationToPlane::CenterInNegativeHalfspace(IntersectsPlane::No) => {
                    return true
                }
                SphereRelationToPlane::CenterInNegativeHalfspace(IntersectsPlane::Yes) => {
                    intersects_from_negative_halfspace[plane_axis] = true;
                    closest_point_ndc[plane_axis] = plane_offset_ndc;
                }
                SphereRelationToPlane::CenterInPositiveHalfspace(_) => {}
            }
        }

        let negative_halfspace_intersection_count = intersects_from_negative_halfspace
            .into_iter()
            .filter(|intersects| *intersects)
            .count();

        // If the sphere intersects none or one plane with its center in
        // the negative halfspace, it mut be at least partially inside
        // the frustum
        if negative_halfspace_intersection_count <= 1 {
            false
        }
        // If the sphere intersects two or three planes with its center
        // in the negative halfspace, it lies on the outside along an
        // edge or near a corner of the frustum, respectively
        else {
            // If the sphere lies along an edge, the coordinate along the
            // edge of the point closest to the sphere is as yet un-
            // determined (that coordinate will not have been updated from
            // `0.0` in `closest_point_ndc`) but must correspond to the
            // coordinate along the edge of the center of the sphere
            if negative_halfspace_intersection_count == 2 {
                let sphere_center_ndc = self.transform_matrix.transform_point(sphere.center());
                for (idx, intersects) in intersects_from_negative_halfspace.into_iter().enumerate()
                {
                    if !intersects {
                        closest_point_ndc[idx] = sphere_center_ndc[idx];
                        break;
                    }
                }
            }

            // We have found the normalized device coordinates of the closest
            // point on the frustum, so we transform that into the space of
            // the sphere using the stored inverse transform
            let closest_point = self
                .inverse_transform_matrix
                .transform_point(&closest_point_ndc);

            // Finally we can determine whether the sphere is fully outside
            // the frustum by checking the distance from the sphere center to
            // the closest point
            na::distance_squared(sphere.center(), &closest_point) >= sphere.radius_squared()
        }
    }

    /// Computes the frustum's axis-aligned bounding box.
    pub fn compute_aabb(&self) -> AxisAlignedBox<F> {
        let corners = [
            self.inverse_transform_matrix
                .transform_point(&point![-F::ONE, -F::ONE, -F::ONE]),
            self.inverse_transform_matrix
                .transform_point(&point![-F::ONE, -F::ONE, F::ONE]),
            self.inverse_transform_matrix
                .transform_point(&point![-F::ONE, F::ONE, -F::ONE]),
            self.inverse_transform_matrix
                .transform_point(&point![-F::ONE, F::ONE, F::ONE]),
            self.inverse_transform_matrix
                .transform_point(&point![F::ONE, -F::ONE, -F::ONE]),
            self.inverse_transform_matrix
                .transform_point(&point![F::ONE, -F::ONE, F::ONE]),
            self.inverse_transform_matrix
                .transform_point(&point![F::ONE, F::ONE, -F::ONE]),
            self.inverse_transform_matrix
                .transform_point(&point![F::ONE, F::ONE, F::ONE]),
        ];

        AxisAlignedBox::aabb_for_point_array(&corners)
    }

    /// Computes the frustum resulting from rotating this frustum with the given
    /// rotation quaternion.
    pub fn rotated(&self, rotation: &UnitQuaternion<F>) -> Self {
        let rotated_planes = [
            self.planes[0].rotated(rotation),
            self.planes[1].rotated(rotation),
            self.planes[2].rotated(rotation),
            self.planes[3].rotated(rotation),
            self.planes[4].rotated(rotation),
            self.planes[5].rotated(rotation),
        ];

        let rotated_inverse_transform_matrix =
            rotation.to_homogeneous() * self.inverse_transform_matrix;

        let inverse_of_rotated_inverse_transform_matrix =
            self.transform_matrix * rotation.inverse().to_homogeneous();

        Self {
            planes: rotated_planes,
            transform_matrix: inverse_of_rotated_inverse_transform_matrix,
            inverse_transform_matrix: rotated_inverse_transform_matrix,
        }
    }

    /// Computes the frustum resulting from transforming this frustum with the
    /// given similarity transform.
    pub fn transformed(&self, transformation: &Similarity3<F>) -> Self {
        let transformed_planes = [
            self.planes[0].transformed(transformation),
            self.planes[1].transformed(transformation),
            self.planes[2].transformed(transformation),
            self.planes[3].transformed(transformation),
            self.planes[4].transformed(transformation),
            self.planes[5].transformed(transformation),
        ];

        let transformed_inverse_transform_matrix =
            transformation.to_homogeneous() * self.inverse_transform_matrix;

        let inverse_of_transformed_inverse_transform_matrix =
            self.transform_matrix * transformation.inverse().to_homogeneous();

        Self {
            planes: transformed_planes,
            transform_matrix: inverse_of_transformed_inverse_transform_matrix,
            inverse_transform_matrix: transformed_inverse_transform_matrix,
        }
    }

    /// Each element represents the plane making up a face
    /// of the frustum cube in normalized device coordinates,
    /// with the first and second tuple element representing
    /// the axis of the plane normal (0 => x, 1 => y, 2 => z)
    /// and the offset of the plane along that axis, respectively.
    const CUBE_PLANES_NDC: [(usize, F); 6] = [
        (0, F::NEG_ONE),
        (0, F::ONE),
        (1, F::NEG_ONE),
        (1, F::ONE),
        (2, F::NEG_ONE),
        (2, F::ONE),
    ];

    fn planes_from_transform_matrix(transform_matrix: &Matrix4<F>) -> [Plane<F>; 6] {
        let m = transform_matrix;

        let left = Self::plane_from_unnormalized_coefficients(
            m[(3, 0)] + m[(0, 0)],
            m[(3, 1)] + m[(0, 1)],
            m[(3, 2)] + m[(0, 2)],
            m[(3, 3)] + m[(0, 3)],
        );
        let right = Self::plane_from_unnormalized_coefficients(
            m[(3, 0)] - m[(0, 0)],
            m[(3, 1)] - m[(0, 1)],
            m[(3, 2)] - m[(0, 2)],
            m[(3, 3)] - m[(0, 3)],
        );
        let bottom = Self::plane_from_unnormalized_coefficients(
            m[(3, 0)] + m[(1, 0)],
            m[(3, 1)] + m[(1, 1)],
            m[(3, 2)] + m[(1, 2)],
            m[(3, 3)] + m[(1, 3)],
        );
        let top = Self::plane_from_unnormalized_coefficients(
            m[(3, 0)] - m[(1, 0)],
            m[(3, 1)] - m[(1, 1)],
            m[(3, 2)] - m[(1, 2)],
            m[(3, 3)] - m[(1, 3)],
        );
        let near = Self::plane_from_unnormalized_coefficients(
            m[(3, 0)] + m[(2, 0)],
            m[(3, 1)] + m[(2, 1)],
            m[(3, 2)] + m[(2, 2)],
            m[(3, 3)] + m[(2, 3)],
        );
        let far = Self::plane_from_unnormalized_coefficients(
            m[(3, 0)] - m[(2, 0)],
            m[(3, 1)] - m[(2, 1)],
            m[(3, 2)] - m[(2, 2)],
            m[(3, 3)] - m[(2, 3)],
        );

        [left, right, bottom, top, near, far]
    }

    fn plane_from_unnormalized_coefficients(
        normal_x: F,
        normal_y: F,
        normal_z: F,
        displacement: F,
    ) -> Plane<F> {
        let (unit_normal, magnitude) =
            UnitVector3::new_and_get(vector![normal_x, normal_y, normal_z]);

        Plane::new(unit_normal, displacement / magnitude)
    }

    #[cfg(test)]
    fn from_transform_matrix(transform_matrix: Matrix4<F>) -> Self {
        Self {
            planes: Self::planes_from_transform_matrix(&transform_matrix),
            transform_matrix,
            inverse_transform_matrix: transform_matrix.try_inverse().unwrap(),
        }
    }
}

impl<F: Float + AbsDiffEq> AbsDiffEq for Frustum<F>
where
    F::Epsilon: Copy,
{
    type Epsilon = F::Epsilon;

    fn default_epsilon() -> F::Epsilon {
        F::default_epsilon()
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: F::Epsilon) -> bool {
        self.planes[0].abs_diff_eq(&other.planes[0], epsilon)
            && self.planes[1].abs_diff_eq(&other.planes[1], epsilon)
            && self.planes[2].abs_diff_eq(&other.planes[2], epsilon)
            && self.planes[3].abs_diff_eq(&other.planes[3], epsilon)
            && self.planes[4].abs_diff_eq(&other.planes[4], epsilon)
            && self.planes[5].abs_diff_eq(&other.planes[5], epsilon)
            && self
                .transform_matrix
                .abs_diff_eq(&other.transform_matrix, epsilon)
            && self
                .inverse_transform_matrix
                .abs_diff_eq(&other.inverse_transform_matrix, epsilon)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use approx::assert_abs_diff_eq;
    use nalgebra::{point, Perspective3, Rotation3, Scale3, Translation3, Vector3};

    #[test]
    fn creating_frustum_for_identity_transform_works() {
        // Should give axis aligned cube with walls at +-1.0
        let frustum = Frustum::<f64>::from_transform_matrix(Matrix4::identity());

        assert_abs_diff_eq!(frustum.left_plane().unit_normal(), &Vector3::x_axis());
        assert_abs_diff_eq!(frustum.left_plane().displacement(), 1.0);

        assert_abs_diff_eq!(frustum.right_plane().unit_normal(), &(-Vector3::x_axis()));
        assert_abs_diff_eq!(frustum.right_plane().displacement(), 1.0);

        assert_abs_diff_eq!(frustum.bottom_plane().unit_normal(), &Vector3::y_axis());
        assert_abs_diff_eq!(frustum.bottom_plane().displacement(), 1.0);

        assert_abs_diff_eq!(frustum.top_plane().unit_normal(), &(-Vector3::y_axis()));
        assert_abs_diff_eq!(frustum.top_plane().displacement(), 1.0);

        assert_abs_diff_eq!(frustum.near_plane().unit_normal(), &Vector3::z_axis());
        assert_abs_diff_eq!(frustum.near_plane().displacement(), 1.0);

        assert_abs_diff_eq!(frustum.far_plane().unit_normal(), &(-Vector3::z_axis()));
        assert_abs_diff_eq!(frustum.far_plane().displacement(), 1.0);
    }

    #[test]
    fn creating_frustum_for_translation_transform_works() {
        let dx = 1.2;
        let dy = 5.3;
        let dz = -0.1;

        let frustum =
            Frustum::<f64>::from_transform_matrix(Translation3::new(dx, dy, dz).to_homogeneous());

        assert_abs_diff_eq!(frustum.left_plane().unit_normal(), &Vector3::x_axis());
        assert_abs_diff_eq!(frustum.left_plane().displacement(), 1.0 + dx);

        assert_abs_diff_eq!(frustum.right_plane().unit_normal(), &(-Vector3::x_axis()));
        assert_abs_diff_eq!(frustum.right_plane().displacement(), 1.0 - dx);

        assert_abs_diff_eq!(frustum.bottom_plane().unit_normal(), &Vector3::y_axis());
        assert_abs_diff_eq!(frustum.bottom_plane().displacement(), 1.0 + dy);

        assert_abs_diff_eq!(frustum.top_plane().unit_normal(), &(-Vector3::y_axis()));
        assert_abs_diff_eq!(frustum.top_plane().displacement(), 1.0 - dy);

        assert_abs_diff_eq!(frustum.near_plane().unit_normal(), &Vector3::z_axis());
        assert_abs_diff_eq!(frustum.near_plane().displacement(), 1.0 + dz);

        assert_abs_diff_eq!(frustum.far_plane().unit_normal(), &(-Vector3::z_axis()));
        assert_abs_diff_eq!(frustum.far_plane().displacement(), 1.0 - dz);
    }

    #[test]
    fn creating_frustum_for_nonuniform_scale_transform_works() {
        let sx = 1.2;
        let sy = 5.3;
        let sz = 9.1;

        let frustum =
            Frustum::<f64>::from_transform_matrix(Scale3::new(sx, sy, sz).to_homogeneous());

        assert_abs_diff_eq!(frustum.left_plane().unit_normal(), &Vector3::x_axis());
        assert_abs_diff_eq!(frustum.left_plane().displacement(), 1.0 / sx);

        assert_abs_diff_eq!(frustum.right_plane().unit_normal(), &(-Vector3::x_axis()));
        assert_abs_diff_eq!(frustum.right_plane().displacement(), 1.0 / sx);

        assert_abs_diff_eq!(frustum.bottom_plane().unit_normal(), &Vector3::y_axis());
        assert_abs_diff_eq!(frustum.bottom_plane().displacement(), 1.0 / sy);

        assert_abs_diff_eq!(frustum.top_plane().unit_normal(), &(-Vector3::y_axis()));
        assert_abs_diff_eq!(frustum.top_plane().displacement(), 1.0 / sy);

        assert_abs_diff_eq!(frustum.near_plane().unit_normal(), &Vector3::z_axis());
        assert_abs_diff_eq!(frustum.near_plane().displacement(), 1.0 / sz);

        assert_abs_diff_eq!(frustum.far_plane().unit_normal(), &(-Vector3::z_axis()));
        assert_abs_diff_eq!(frustum.far_plane().displacement(), 1.0 / sz);
    }

    #[test]
    fn creating_frustum_for_90_deg_rotation_transform_works() {
        let frustum = Frustum::<f64>::from_transform_matrix(
            Rotation3::from_axis_angle(&Vector3::z_axis(), std::f64::consts::FRAC_PI_2)
                .to_homogeneous(),
        );

        assert_abs_diff_eq!(frustum.left_plane().unit_normal(), &(-Vector3::y_axis()));
        assert_abs_diff_eq!(frustum.left_plane().displacement(), 1.0);

        assert_abs_diff_eq!(frustum.right_plane().unit_normal(), &Vector3::y_axis());
        assert_abs_diff_eq!(frustum.right_plane().displacement(), 1.0);

        assert_abs_diff_eq!(frustum.bottom_plane().unit_normal(), &Vector3::x_axis());
        assert_abs_diff_eq!(frustum.bottom_plane().displacement(), 1.0);

        assert_abs_diff_eq!(frustum.top_plane().unit_normal(), &(-Vector3::x_axis()));
        assert_abs_diff_eq!(frustum.top_plane().displacement(), 1.0);

        assert_abs_diff_eq!(frustum.near_plane().unit_normal(), &Vector3::z_axis());
        assert_abs_diff_eq!(frustum.near_plane().displacement(), 1.0);

        assert_abs_diff_eq!(frustum.far_plane().unit_normal(), &(-Vector3::z_axis()));
        assert_abs_diff_eq!(frustum.far_plane().displacement(), 1.0);
    }

    #[test]
    fn inside_points_are_reported_as_inside() {
        let frustum = Frustum::<f64>::from_transform_matrix(Matrix4::identity());
        for x in [-0.999, 0.999] {
            for y in [-0.999, 0.999] {
                for z in [-0.999, 0.999] {
                    assert!(frustum.contains_point(&point![x, y, z]));
                }
            }
        }
    }

    #[test]
    fn outside_points_are_reported_as_outside() {
        let frustum = Frustum::<f64>::from_transform_matrix(Matrix4::identity());
        for x in [-1.001, 1.001] {
            for y in [-1.001, 1.001] {
                for z in [-1.001, 1.001] {
                    assert!(!frustum.contains_point(&point![x, y, z]));
                }
            }
        }
    }

    #[test]
    fn outside_spheres_are_reported_as_outside() {
        let frustum = Frustum::<f64>::from_transform_matrix(Matrix4::identity());
        for x in [-2, 0, 2] {
            for y in [-2, 0, 2] {
                for z in [-2, 0, 2] {
                    let dist_to_frustum = match (x, y, z) {
                        (0, 0, 0) => {
                            continue;
                        }
                        (_, 0, 0) | (0, _, 0) | (0, 0, _) => 1.0,
                        (0, _, _) | (_, 0, _) | (_, _, 0) => f64::sqrt(2.0),
                        _ => f64::sqrt(3.0),
                    };
                    for dist_fraction in [0.999, 0.5, 0.1] {
                        let sphere = Sphere::new(
                            point![f64::from(x), f64::from(y), f64::from(z)],
                            dist_fraction * dist_to_frustum,
                        );
                        assert!(frustum.sphere_lies_outside(&sphere));
                    }
                }
            }
        }
    }

    #[test]
    fn barely_inside_spheres_are_reported_as_not_outside() {
        let frustum = Frustum::<f64>::from_transform_matrix(Matrix4::identity());
        for x in [-2, 0, 2] {
            for y in [-2, 0, 2] {
                for z in [-2, 0, 2] {
                    let dist_to_frustum = match (x, y, z) {
                        (0, 0, 0) => {
                            continue;
                        }
                        (_, 0, 0) | (0, _, 0) | (0, 0, _) => 1.0,
                        (0, _, _) | (_, 0, _) | (_, _, 0) => f64::sqrt(2.0),
                        _ => f64::sqrt(3.0),
                    };
                    let sphere = Sphere::new(
                        point![f64::from(x), f64::from(y), f64::from(z)],
                        1.001 * dist_to_frustum,
                    );
                    assert!(!frustum.sphere_lies_outside(&sphere));
                }
            }
        }
    }

    #[test]
    fn centered_spheres_are_reported_as_not_outside() {
        let frustum = Frustum::<f64>::from_transform_matrix(Matrix4::identity());
        for radius in [0.01, 0.999, 1.001, 2.0, 10.0] {
            let sphere = Sphere::new(Point3::origin(), radius);
            assert!(!frustum.sphere_lies_outside(&sphere));
        }
    }

    #[test]
    fn creating_frustum_for_transform_of_transformed_frustum_gives_transformed_frustum() {
        let frustum = Frustum::<f64>::from_transform_matrix(
            Perspective3::new(0.7, 2.3, 0.21, 160.2).to_homogeneous(),
        );

        let transformation = Similarity3::from_parts(
            Translation3::new(2.1, -5.9, 0.01),
            Rotation3::from_euler_angles(0.1, 0.2, 0.3).into(),
            7.0,
        );

        let transformed_frustum = frustum.transformed(&transformation);

        let frustum_from_transformed =
            Frustum::<f64>::from_transform_matrix(*transformed_frustum.transform_matrix());

        assert_abs_diff_eq!(
            transformed_frustum,
            frustum_from_transformed,
            epsilon = 1e-9
        );
    }
}
