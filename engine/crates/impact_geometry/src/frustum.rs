//! Representation of frustums.

use crate::{AxisAlignedBox, Plane, PlaneC, Sphere};
use approx::AbsDiffEq;
use impact_math::{
    bounds::{Bounds, UpperExclusiveBounds},
    matrix::{Matrix4, Matrix4C},
    point::Point3,
    transform::{Projective3, Similarity3},
    vector::{UnitVector3, Vector3},
};

/// A frustum, which in general is a pyramid truncated at the top. It is here
/// represented by the six planes making up the faces of the truncated pyramid.
///
/// The planes are created in such a way that their negative halfspaces
/// correspond to the space outside the frustum.
///
/// The planes and transform matrices are stored in 128-bit SIMD registers for
/// efficient computation. That leads to an extra 102 bytes in size (16 for each
/// plane and 6 due to additional padding) and 16-byte alignment. For
/// cache-friendly storage, prefer the compact 4-byte aligned [`FrustumC`].
#[derive(Clone, Debug, PartialEq)]
pub struct Frustum {
    planes: [Plane; 6],
    largest_signed_dist_aab_corner_indices_for_planes: [u8; 6],
    inverse_transform_matrix: Matrix4,
}

/// A frustum, which in general is a pyramid truncated at the top. It is here
/// represented by the six planes making up the faces of the truncated pyramid.
/// This is the "compact" version.
///
/// The planes are created in such a way that their negative halfspaces
/// correspond to the space outside the frustum.
///
/// This type only supports a few basic operations, as is primarily intended for
/// compact storage inside other types and collections. For computations, prefer
/// the SIMD-friendly 16-byte aligned [`Frustum`].
#[derive(Clone, Debug, PartialEq)]
pub struct FrustumC {
    planes: [PlaneC; 6],
    largest_signed_dist_aab_corner_indices_for_planes: [u8; 6],
    inverse_transform_matrix: Matrix4C,
}

impl Frustum {
    /// Creates the frustum representing the clip space of the
    /// given transform.
    ///
    /// This function uses the method of Gribb and Hartmann (2001)
    /// "Fast Extraction of Viewing Frustum Planes from the
    /// World-View-Projection Matrix".
    pub fn from_transform(transform: &Projective3) -> Self {
        let planes = Self::planes_from_transform_matrix(transform.matrix());

        let largest_signed_dist_aab_corner_indices_for_planes =
            Self::determine_largest_signed_dist_aab_corner_indices_for_all_planes(&planes);

        Self {
            planes,
            largest_signed_dist_aab_corner_indices_for_planes,
            inverse_transform_matrix: *transform.inverted().matrix(),
        }
    }

    /// Creates the frustum representing the clip space of the given transform
    /// matrix, using the given matrix inverse rather than computing it.
    pub fn from_transform_matrix_with_inverse(
        transform_matrix: Matrix4,
        inverse_transform_matrix: Matrix4,
    ) -> Self {
        let planes = Self::planes_from_transform_matrix(&transform_matrix);

        let largest_signed_dist_aab_corner_indices_for_planes =
            Self::determine_largest_signed_dist_aab_corner_indices_for_all_planes(&planes);

        Self {
            planes,
            largest_signed_dist_aab_corner_indices_for_planes,
            inverse_transform_matrix,
        }
    }

    /// Returns the planes defining the faces of the frustum.
    #[inline]
    pub const fn planes(&self) -> &[Plane; 6] {
        &self.planes
    }

    /// Returns the plane defining the left face of the frustum.
    #[inline]
    pub const fn left_plane(&self) -> &Plane {
        &self.planes[0]
    }

    /// Returns the plane defining the right face of the frustum.
    #[inline]
    pub const fn right_plane(&self) -> &Plane {
        &self.planes[1]
    }

    /// Returns the plane defining the bottom face of the frustum.
    #[inline]
    pub const fn bottom_plane(&self) -> &Plane {
        &self.planes[2]
    }

    /// Returns the plane defining the top face of the frustum.
    #[inline]
    pub const fn top_plane(&self) -> &Plane {
        &self.planes[3]
    }

    /// Returns the near plane of the frustum.
    #[inline]
    pub const fn near_plane(&self) -> &Plane {
        &self.planes[4]
    }

    /// Returns the far plane of the frustum.
    #[inline]
    pub const fn far_plane(&self) -> &Plane {
        &self.planes[5]
    }

    /// Returns the distance from the frustum apex to the near plane.
    #[inline]
    pub fn near_distance(&self) -> f32 {
        self.near_plane().displacement()
    }

    /// Returns the distance from the frustum apex to the far plane.
    #[inline]
    pub fn far_distance(&self) -> f32 {
        -self.far_plane().displacement()
    }

    /// Whether the given point is strictly inside the frustum.
    #[inline]
    pub fn contains_point(&self, point: &Point3) -> bool {
        self.planes
            .iter()
            .all(|plane| plane.point_lies_in_positive_halfspace(point))
    }

    /// Whether any part of the given sphere could be inside the frustum. If the
    /// sphere lies close to an edge or a corner, this method may return `true`
    /// even if the sphere is really outside. However, this method is will
    /// always return `true` if the sphere is really inside. If the boundaries
    /// exactly touch each other, the sphere is considered inside.
    #[inline]
    pub fn could_contain_part_of_sphere(&self, sphere: &Sphere) -> bool {
        self.planes
            .iter()
            .all(|plane| plane.compute_signed_distance(sphere.center()) >= -sphere.radius())
    }

    /// Whether any part of the given axis-aligned box could be inside the
    /// frustum. If the box lies close to an edge or a corner, this method may
    /// return `true` even if the box is really outside. However, this method is
    /// will always return `true` if the box is really inside. If the boundaries
    /// exactly touch each other, the box is considered inside.
    #[inline]
    pub fn could_contain_part_of_axis_aligned_box(
        &self,
        axis_aligned_box: &AxisAlignedBox,
    ) -> bool {
        self.planes
            .iter()
            .zip(
                self.largest_signed_dist_aab_corner_indices_for_planes
                    .iter(),
            )
            .all(|(plane, &largest_signed_dist_corner_idx)| {
                plane.compute_signed_distance(
                    &axis_aligned_box.corner(largest_signed_dist_corner_idx as usize),
                ) >= 0.0
            })
    }

    /// Computes the 8 corners of the frustum.
    pub fn compute_corners(&self) -> [Point3; 8] {
        [
            self.inverse_transform_matrix
                .project_point(&Point3::new(-1.0, -1.0, 0.0)),
            self.inverse_transform_matrix
                .project_point(&Point3::new(-1.0, -1.0, 1.0)),
            self.inverse_transform_matrix
                .project_point(&Point3::new(-1.0, 1.0, 0.0)),
            self.inverse_transform_matrix
                .project_point(&Point3::new(-1.0, 1.0, 1.0)),
            self.inverse_transform_matrix
                .project_point(&Point3::new(1.0, -1.0, 0.0)),
            self.inverse_transform_matrix
                .project_point(&Point3::new(1.0, -1.0, 1.0)),
            self.inverse_transform_matrix
                .project_point(&Point3::new(1.0, 1.0, 0.0)),
            self.inverse_transform_matrix
                .project_point(&Point3::new(1.0, 1.0, 1.0)),
        ]
    }

    /// Computes the linear depth, which increases linearly with distance from 0
    /// at the frustum apex to 1 at the base, corresponding to the given
    /// distance from the frustum apex along the view direction.
    #[inline]
    pub fn convert_view_distance_to_linear_depth(&self, distance: f32) -> f32 {
        distance / self.far_distance()
    }

    /// Computes the distance from the frustum apex along the view direction
    /// corresponding to the given linear depth (which increases linearly with
    /// distance from 0 at the frustum apex to 1 at the base).
    #[inline]
    pub fn convert_linear_depth_to_view_distance(&self, linear_depth: f32) -> f32 {
        linear_depth * self.far_distance()
    }

    /// Computes the center point of the frustum.
    #[inline]
    pub fn compute_center(&self) -> Point3 {
        let corners = self.compute_corners();
        let n_corners = corners.len();

        corners
            .into_iter()
            .reduce(|accum, point| accum + point.as_vector())
            .unwrap()
            / (n_corners as f32)
    }

    /// Computes the frustum's axis-aligned bounding box.
    #[inline]
    pub fn compute_aabb(&self) -> AxisAlignedBox {
        AxisAlignedBox::aabb_for_point_array(&self.compute_corners())
    }

    /// Computes the planes of the frustum resulting from transforming this
    /// frustum with the given similarity transform.
    pub fn transformed_planes(&self, transformation: &Similarity3) -> [Plane; 6] {
        [
            self.planes[0].transformed(transformation),
            self.planes[1].transformed(transformation),
            self.planes[2].transformed(transformation),
            self.planes[3].transformed(transformation),
            self.planes[4].transformed(transformation),
            self.planes[5].transformed(transformation),
        ]
    }

    /// Converts the frustum to the 4-byte aligned cache-friendly [`FrustumC`].
    #[inline]
    pub fn compact(&self) -> FrustumC {
        FrustumC {
            planes: self.planes.clone().map(|plane| plane.compact()),
            largest_signed_dist_aab_corner_indices_for_planes: self
                .largest_signed_dist_aab_corner_indices_for_planes,
            inverse_transform_matrix: self.inverse_transform_matrix.compact(),
        }
    }

    /// Computes the 8 corners of the part of the frustum lying between the
    /// given linear (as opposed to clip space-) depths.
    ///
    /// This methods takes the corners from [`Self::compute_corners`] to allow
    /// them to be cached rather than recomputed for every call.
    pub fn compute_corners_of_subfrustum(
        full_frustum_corners: &[Point3; 8],
        frustum_near_distance: f32,
        frustum_far_distance: f32,
        linear_depth_limits: UpperExclusiveBounds<f32>,
    ) -> [Point3; 8] {
        let lower = linear_depth_limits.lower();
        let upper = linear_depth_limits.upper();

        // To interpolate between near and far corners, we need an interpolation
        // factor that is 0 at the near plane and 1 at the far plane. Linear
        // depth is zero at the apex and 1 at the far plane.

        let near_plane_linear_depth = frustum_near_distance / frustum_far_distance;
        let far_plane_linear_depth = 1.0;

        let scaling = 1.0 / (far_plane_linear_depth - near_plane_linear_depth);

        let lower_fraction = (lower - near_plane_linear_depth) * scaling;
        let upper_fraction = (upper - near_plane_linear_depth) * scaling;

        let mut corners = [Point3::origin(); 8];

        for i in 0..4 {
            let near_corner = full_frustum_corners[2 * i];
            let far_corner = full_frustum_corners[2 * i + 1];

            let displacement = far_corner - near_corner;

            corners[2 * i] = near_corner + lower_fraction * displacement;
            corners[2 * i + 1] = near_corner + upper_fraction * displacement;
        }

        corners
    }

    fn planes_from_transform_matrix(transform_matrix: &Matrix4) -> [Plane; 6] {
        let c1 = transform_matrix.column_1();
        let c2 = transform_matrix.column_2();
        let c3 = transform_matrix.column_3();
        let c4 = transform_matrix.column_4();

        let left = Self::plane_from_unnormalized_coefficients(
            c1.w() + c1.x(),
            c2.w() + c2.x(),
            c3.w() + c3.x(),
            -(c4.w() + c4.x()),
        );
        let right = Self::plane_from_unnormalized_coefficients(
            c1.w() - c1.x(),
            c2.w() - c2.x(),
            c3.w() - c3.x(),
            -(c4.w() - c4.x()),
        );

        let bottom = Self::plane_from_unnormalized_coefficients(
            c1.w() + c1.y(),
            c2.w() + c2.y(),
            c3.w() + c3.y(),
            -(c4.w() + c4.y()),
        );
        let top = Self::plane_from_unnormalized_coefficients(
            c1.w() - c1.y(),
            c2.w() - c2.y(),
            c3.w() - c3.y(),
            -(c4.w() - c4.y()),
        );

        let near = Self::plane_from_unnormalized_coefficients(c1.z(), c2.z(), c3.z(), -c4.z());
        let far = Self::plane_from_unnormalized_coefficients(
            c1.w() - c1.z(),
            c2.w() - c2.z(),
            c3.w() - c3.z(),
            -(c4.w() - c4.z()),
        );

        [left, right, bottom, top, near, far]
    }

    #[inline]
    fn plane_from_unnormalized_coefficients(
        normal_x: f32,
        normal_y: f32,
        normal_z: f32,
        displacement: f32,
    ) -> Plane {
        let (unit_normal, norm) =
            UnitVector3::normalized_from_and_norm(Vector3::new(normal_x, normal_y, normal_z));

        Plane::new(unit_normal, displacement / norm)
    }

    /// Determines the corner of any axis-aligned bounding box that will have
    /// the largest signed distance in the space of the given plane. The corner
    /// is represented by an index following the convention of
    /// [`AxisAlignedBox::corner`].
    #[inline]
    pub fn determine_largest_signed_dist_aab_corner_index_for_plane(plane: &Plane) -> u8 {
        let normal = plane.unit_normal();
        match (
            normal.x().is_sign_negative(),
            normal.y().is_sign_negative(),
            normal.z().is_sign_negative(),
        ) {
            (true, true, true) => 0,
            (true, true, false) => 1,
            (true, false, true) => 2,
            (true, false, false) => 3,
            (false, true, true) => 4,
            (false, true, false) => 5,
            (false, false, true) => 6,
            (false, false, false) => 7,
        }
    }

    #[inline]
    fn determine_largest_signed_dist_aab_corner_indices_for_all_planes(
        planes: &[Plane; 6],
    ) -> [u8; 6] {
        [
            Self::determine_largest_signed_dist_aab_corner_index_for_plane(&planes[0]),
            Self::determine_largest_signed_dist_aab_corner_index_for_plane(&planes[1]),
            Self::determine_largest_signed_dist_aab_corner_index_for_plane(&planes[2]),
            Self::determine_largest_signed_dist_aab_corner_index_for_plane(&planes[3]),
            Self::determine_largest_signed_dist_aab_corner_index_for_plane(&planes[4]),
            Self::determine_largest_signed_dist_aab_corner_index_for_plane(&planes[5]),
        ]
    }
}

impl AbsDiffEq for Frustum {
    type Epsilon = f32;

    fn default_epsilon() -> f32 {
        f32::default_epsilon()
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: f32) -> bool {
        self.planes[0].abs_diff_eq(&other.planes[0], epsilon)
            && self.planes[1].abs_diff_eq(&other.planes[1], epsilon)
            && self.planes[2].abs_diff_eq(&other.planes[2], epsilon)
            && self.planes[3].abs_diff_eq(&other.planes[3], epsilon)
            && self.planes[4].abs_diff_eq(&other.planes[4], epsilon)
            && self.planes[5].abs_diff_eq(&other.planes[5], epsilon)
            && self
                .inverse_transform_matrix
                .abs_diff_eq(&other.inverse_transform_matrix, epsilon)
    }
}

impl FrustumC {
    /// Returns the planes defining the faces of the frustum.
    #[inline]
    pub const fn planes(&self) -> &[PlaneC; 6] {
        &self.planes
    }

    /// Returns the plane defining the left face of the frustum.
    #[inline]
    pub const fn left_plane(&self) -> &PlaneC {
        &self.planes[0]
    }

    /// Returns the plane defining the right face of the frustum.
    #[inline]
    pub const fn right_plane(&self) -> &PlaneC {
        &self.planes[1]
    }

    /// Returns the plane defining the bottom face of the frustum.
    #[inline]
    pub const fn bottom_plane(&self) -> &PlaneC {
        &self.planes[2]
    }

    /// Returns the plane defining the top face of the frustum.
    #[inline]
    pub const fn top_plane(&self) -> &PlaneC {
        &self.planes[3]
    }

    /// Returns the near plane of the frustum.
    #[inline]
    pub const fn near_plane(&self) -> &PlaneC {
        &self.planes[4]
    }

    /// Returns the far plane of the frustum.
    #[inline]
    pub const fn far_plane(&self) -> &PlaneC {
        &self.planes[5]
    }

    /// Returns the distance from the frustum apex to the near plane.
    #[inline]
    pub fn near_distance(&self) -> f32 {
        self.near_plane().displacement()
    }

    /// Returns the distance from the frustum apex to the far plane.
    #[inline]
    pub fn far_distance(&self) -> f32 {
        -self.far_plane().displacement()
    }

    /// Converts the frustum to the 16-byte aligned SIMD-friendly [`Frustum`].
    #[inline]
    pub fn aligned(&self) -> Frustum {
        Frustum {
            planes: self.planes.map(|plane| plane.aligned()),
            largest_signed_dist_aab_corner_indices_for_planes: self
                .largest_signed_dist_aab_corner_indices_for_planes,
            inverse_transform_matrix: self.inverse_transform_matrix.aligned(),
        }
    }
}

impl AbsDiffEq for FrustumC {
    type Epsilon = f32;

    fn default_epsilon() -> f32 {
        f32::default_epsilon()
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: f32) -> bool {
        self.planes[0].abs_diff_eq(&other.planes[0], epsilon)
            && self.planes[1].abs_diff_eq(&other.planes[1], epsilon)
            && self.planes[2].abs_diff_eq(&other.planes[2], epsilon)
            && self.planes[3].abs_diff_eq(&other.planes[3], epsilon)
            && self.planes[4].abs_diff_eq(&other.planes[4], epsilon)
            && self.planes[5].abs_diff_eq(&other.planes[5], epsilon)
            && self
                .inverse_transform_matrix
                .abs_diff_eq(&other.inverse_transform_matrix, epsilon)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::projection::{OrthographicTransform, PerspectiveTransform};
    use approx::assert_abs_diff_eq;
    use impact_math::angle::Degrees;

    #[test]
    fn computing_frustum_near_and_far_distance_works() {
        let near = 0.21;
        let far = 160.2;
        let frustum = Frustum::from_transform(
            PerspectiveTransform::new(1.0, Degrees(56.0), UpperExclusiveBounds::new(near, far))
                .as_projective(),
        );

        assert_abs_diff_eq!(frustum.near_distance(), near, epsilon = 1e-2);
        assert_abs_diff_eq!(frustum.far_distance(), far, epsilon = 1e-2);
    }

    #[test]
    fn inside_points_are_reported_as_inside() {
        let frustum = Frustum::from_transform(
            OrthographicTransform::new(-1.0, 1.0, -1.0, 1.0, -1.0, 1.0).as_projective(),
        );
        for x in [-0.999, 0.999] {
            for y in [-0.999, 0.999] {
                for z in [-0.999, 0.999] {
                    assert!(frustum.contains_point(&Point3::new(x, y, z)));
                }
            }
        }
    }

    #[test]
    fn outside_points_are_reported_as_outside() {
        let frustum = Frustum::from_transform(
            OrthographicTransform::new(-1.0, 1.0, -1.0, 1.0, -1.0, 1.0).as_projective(),
        );
        for x in [-1.001, 1.001] {
            for y in [-1.001, 1.001] {
                for z in [-1.001, 1.001] {
                    assert!(!frustum.contains_point(&Point3::new(x, y, z)));
                }
            }
        }
    }

    #[test]
    fn safely_outside_spheres_are_reported_as_outside() {
        let frustum = Frustum::from_transform(
            OrthographicTransform::new(-1.0, 1.0, -1.0, 1.0, -1.0, 1.0).as_projective(),
        );
        for x in [-2, 0, 2] {
            for y in [-2, 0, 2] {
                for z in [-2, 0, 2] {
                    let dist_to_frustum = match (x, y, z) {
                        (0, 0, 0) => {
                            continue;
                        }
                        (_, 0, 0) | (0, _, 0) | (0, 0, _) => 1.0,
                        (0, _, _) | (_, 0, _) | (_, _, 0) => f32::sqrt(2.0),
                        _ => f32::sqrt(3.0),
                    };
                    for dist_fraction in [0.5, 0.3, 0.1] {
                        let sphere = Sphere::new(
                            Point3::new(x as f32, y as f32, z as f32),
                            dist_fraction * dist_to_frustum,
                        );
                        assert!(!frustum.could_contain_part_of_sphere(&sphere));
                    }
                }
            }
        }
    }

    #[test]
    fn barely_inside_spheres_are_reported_as_not_outside() {
        let frustum = Frustum::from_transform(
            OrthographicTransform::new(-1.0, 1.0, -1.0, 1.0, -1.0, 1.0).as_projective(),
        );
        for x in [-2, 0, 2] {
            for y in [-2, 0, 2] {
                for z in [-2, 0, 2] {
                    let dist_to_frustum = match (x, y, z) {
                        (0, 0, 0) => {
                            continue;
                        }
                        (_, 0, 0) | (0, _, 0) | (0, 0, _) => 1.0,
                        (0, _, _) | (_, 0, _) | (_, _, 0) => f32::sqrt(2.0),
                        _ => f32::sqrt(3.0),
                    };
                    let sphere = Sphere::new(
                        Point3::new(x as f32, y as f32, z as f32),
                        1.001 * dist_to_frustum,
                    );
                    assert!(frustum.could_contain_part_of_sphere(&sphere));
                }
            }
        }
    }

    #[test]
    fn centered_spheres_are_reported_as_not_outside() {
        let frustum = Frustum::from_transform(
            OrthographicTransform::new(-1.0, 1.0, -1.0, 1.0, -1.0, 1.0).as_projective(),
        );
        for radius in [0.01, 0.999, 1.001, 2.0, 10.0, 0.0] {
            let sphere = Sphere::new(Point3::origin(), radius);
            assert!(frustum.could_contain_part_of_sphere(&sphere));
        }
    }

    #[test]
    fn inside_sphere_is_reported_as_not_outside() {
        let frustum = Frustum::from_transform(
            PerspectiveTransform::new(1.0, Degrees(90.0), UpperExclusiveBounds::new(1.0, 10.0))
                .as_projective(),
        );

        let sphere = Sphere::new(Point3::new(3.37632, -3.3647947, -2.6214356), 1.0);

        assert!(frustum.could_contain_part_of_sphere(&sphere));
    }

    #[test]
    fn outside_aabs_are_reported_as_outside() {
        let frustum = Frustum::from_transform(
            OrthographicTransform::new(-1.0, 1.0, -1.0, 1.0, -1.0, 1.0).as_projective(),
        );
        for x in [-2, 0, 2] {
            for y in [-2, 0, 2] {
                for z in [-2, 0, 2] {
                    for offset_fraction in [0.9, 0.7, 0.5] {
                        if x == 0 && y == 0 && z == 0 {
                            continue;
                        }
                        let center = Point3::new(x as f32, y as f32, z as f32);
                        let corner_offset =
                            Vector3::new(offset_fraction, offset_fraction, offset_fraction);
                        let aab =
                            AxisAlignedBox::new(center - corner_offset, center + corner_offset);
                        assert!(!frustum.could_contain_part_of_axis_aligned_box(&aab));
                    }
                }
            }
        }
    }

    #[test]
    fn barely_inside_aabs_are_reported_as_not_outside() {
        let frustum = Frustum::from_transform(
            OrthographicTransform::new(-1.0, 1.0, -1.0, 1.0, -1.0, 1.0).as_projective(),
        );
        for x in [-2, 0, 2] {
            for y in [-2, 0, 2] {
                for z in [-2, 0, 2] {
                    let center = Point3::new(x as f32, y as f32, z as f32);
                    let corner_offset = Vector3::new(1.0, 1.0, 1.0) * 1.001;
                    let aab = AxisAlignedBox::new(center - corner_offset, center + corner_offset);
                    assert!(frustum.could_contain_part_of_axis_aligned_box(&aab));
                }
            }
        }
    }

    #[test]
    fn centered_aabs_are_reported_as_not_outside() {
        let frustum = Frustum::from_transform(
            OrthographicTransform::new(-1.0, 1.0, -1.0, 1.0, -1.0, 1.0).as_projective(),
        );
        for half_extent in [0.01, 0.999, 1.001, 2.0, 10.0, 0.0] {
            let corner_offset = Vector3::new(1.0, 1.0, 1.0) * half_extent;
            let aab = AxisAlignedBox::new(
                Point3::origin() - corner_offset,
                Point3::origin() + corner_offset,
            );
            assert!(frustum.could_contain_part_of_axis_aligned_box(&aab));
        }
    }

    #[test]
    fn computing_orthographic_frustum_corners_works() {
        let (left, right, bottom, top, near, far) = (0.1, 1.2, 2.3, 3.4, 4.5, 5.6);
        let frustum = Frustum::from_transform(
            OrthographicTransform::new(left, right, bottom, top, near, far).as_projective(),
        );

        let corners = frustum.compute_corners();

        assert_abs_diff_eq!(corners[0], Point3::new(left, bottom, near), epsilon = 1e-5);
        assert_abs_diff_eq!(corners[1], Point3::new(left, bottom, far), epsilon = 1e-5);
        assert_abs_diff_eq!(corners[2], Point3::new(left, top, near), epsilon = 1e-5);
        assert_abs_diff_eq!(corners[3], Point3::new(left, top, far), epsilon = 1e-5);
        assert_abs_diff_eq!(corners[4], Point3::new(right, bottom, near), epsilon = 1e-5);
        assert_abs_diff_eq!(corners[5], Point3::new(right, bottom, far), epsilon = 1e-5);
        assert_abs_diff_eq!(corners[6], Point3::new(right, top, near), epsilon = 1e-5);
        assert_abs_diff_eq!(corners[7], Point3::new(right, top, far), epsilon = 1e-5);
    }

    #[test]
    fn computing_orthographic_subfrustum_corners_works() {
        let (left, right, bottom, top, near, far) = (0.1, 1.2, 2.3, 3.4, 4.5, 5.6);
        let frustum = Frustum::from_transform(
            OrthographicTransform::new(left, right, bottom, top, near, far).as_projective(),
        );

        let (new_near, new_far) = (4.9, 5.2);

        let new_near_linear_depth = frustum.convert_view_distance_to_linear_depth(new_near);
        let new_far_linear_depth = frustum.convert_view_distance_to_linear_depth(new_far);

        let corners = Frustum::compute_corners_of_subfrustum(
            &frustum.compute_corners(),
            frustum.near_distance(),
            frustum.far_distance(),
            UpperExclusiveBounds::new(new_near_linear_depth, new_far_linear_depth),
        );

        assert_abs_diff_eq!(
            corners[0],
            Point3::new(left, bottom, new_near),
            epsilon = 1e-5
        );
        assert_abs_diff_eq!(
            corners[1],
            Point3::new(left, bottom, new_far),
            epsilon = 1e-5
        );
        assert_abs_diff_eq!(corners[2], Point3::new(left, top, new_near), epsilon = 1e-5);
        assert_abs_diff_eq!(corners[3], Point3::new(left, top, new_far), epsilon = 1e-5);
        assert_abs_diff_eq!(
            corners[4],
            Point3::new(right, bottom, new_near),
            epsilon = 1e-5
        );
        assert_abs_diff_eq!(
            corners[5],
            Point3::new(right, bottom, new_far),
            epsilon = 1e-5
        );
        assert_abs_diff_eq!(
            corners[6],
            Point3::new(right, top, new_near),
            epsilon = 1e-5
        );
        assert_abs_diff_eq!(corners[7], Point3::new(right, top, new_far), epsilon = 1e-5);
    }

    #[test]
    fn computing_orthographic_frustum_center_works() {
        let (left, right, bottom, top, near, far) = (0.1, 1.2, 2.3, 3.4, 4.5, 5.6);
        let frustum = Frustum::from_transform(
            OrthographicTransform::new(left, right, bottom, top, near, far).as_projective(),
        );

        let center = frustum.compute_center();

        assert_abs_diff_eq!(
            center,
            Point3::new(
                0.5 * (left + right),
                0.5 * (bottom + top),
                0.5 * (near + far)
            ),
            epsilon = 1e-5
        );
    }

    #[test]
    fn computing_orthographic_frustum_aabb_works() {
        let (left, right, bottom, top, near, far) = (0.1, 1.2, 2.3, 3.4, 4.5, 5.6);
        let frustum = Frustum::from_transform(
            OrthographicTransform::new(left, right, bottom, top, near, far).as_projective(),
        );

        let aabb = frustum.compute_aabb();

        assert_abs_diff_eq!(
            aabb.lower_corner(),
            &Point3::new(left, bottom, near),
            epsilon = 1e-5
        );
        assert_abs_diff_eq!(
            aabb.upper_corner(),
            &Point3::new(right, top, far),
            epsilon = 1e-5
        );
    }

    #[test]
    fn should_determine_correct_largest_signed_dist_aab_corner_indices() {
        let frustum = Frustum::from_transform(
            PerspectiveTransform::new(1.0, Degrees(90.0), UpperExclusiveBounds::new(1.0, 10.0))
                .as_projective(),
        );

        let aab = AxisAlignedBox::new(Point3::origin(), Point3::new(1.0, 1.0, 1.0));

        for plane_idx in 0..6 {
            let plane = &frustum.planes[plane_idx];
            let largest_signed_dist_aab_corner_idx =
                frustum.largest_signed_dist_aab_corner_indices_for_planes[plane_idx];
            let largest_signed_dist = plane
                .compute_signed_distance(&aab.corner(largest_signed_dist_aab_corner_idx as usize));
            for corner_idx in 0..8 {
                assert!(
                    plane.compute_signed_distance(&aab.corner(corner_idx)) <= largest_signed_dist
                );
            }
        }
    }
}
