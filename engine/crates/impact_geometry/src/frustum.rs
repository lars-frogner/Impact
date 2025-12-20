//! Representation of frustums.

use crate::{AxisAlignedBox, Plane, Sphere};
use approx::AbsDiffEq;
use impact_math::{
    bounds::{Bounds, UpperExclusiveBounds},
    transform::{Projective3, Similarity3},
};
use nalgebra::{Matrix4, Point3, UnitQuaternion, UnitVector3, Vector3};

/// A frustum, which in general is a pyramid truncated at the
/// top. It is here represented by the six planes making up
/// the faces of the truncated pyramid.
///
/// The planes are created in such a way that their negative
/// halfspaces correspond to the space outside the frustum.
#[derive(Clone, Debug, PartialEq)]
pub struct Frustum {
    planes: [Plane; 6],
    largest_signed_dist_aab_corner_indices_for_planes: [usize; 6],
    transform_matrix: Matrix4<f32>,
    inverse_transform_matrix: Matrix4<f32>,
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
            Self::determine_largest_signed_dist_aab_corner_indices_for_all_planes(planes);

        Self {
            planes,
            largest_signed_dist_aab_corner_indices_for_planes,
            transform_matrix: *transform.matrix(),
            inverse_transform_matrix: *transform.inverse().matrix(),
        }
    }

    /// Creates the frustum representing the clip space of the given transform
    /// matrix, using the given matrix inverse rather than computing it.
    pub fn from_transform_matrix_with_inverse(
        transform_matrix: Matrix4<f32>,
        inverse_transform_matrix: Matrix4<f32>,
    ) -> Self {
        let planes = Self::planes_from_transform_matrix(&transform_matrix);

        let largest_signed_dist_aab_corner_indices_for_planes =
            Self::determine_largest_signed_dist_aab_corner_indices_for_all_planes(planes);

        Self {
            planes,
            largest_signed_dist_aab_corner_indices_for_planes,
            transform_matrix,
            inverse_transform_matrix,
        }
    }

    /// Returns the planes defining the faces of the frustum.
    pub fn planes(&self) -> &[Plane; 6] {
        &self.planes
    }

    /// Returns the plane defining the left face of the frustum.
    pub fn left_plane(&self) -> &Plane {
        &self.planes[0]
    }

    /// Returns the plane defining the right face of the frustum.
    pub fn right_plane(&self) -> &Plane {
        &self.planes[1]
    }

    /// Returns the plane defining the bottom face of the frustum.
    pub fn bottom_plane(&self) -> &Plane {
        &self.planes[2]
    }

    /// Returns the plane defining the top face of the frustum.
    pub fn top_plane(&self) -> &Plane {
        &self.planes[3]
    }

    /// Returns the near plane of the frustum.
    pub fn near_plane(&self) -> &Plane {
        &self.planes[4]
    }

    /// Returns the far plane of the frustum.
    pub fn far_plane(&self) -> &Plane {
        &self.planes[5]
    }

    /// Returns the matrix of the transform into the clip space
    /// that this frustum represents.
    pub fn transform_matrix(&self) -> &Matrix4<f32> {
        &self.transform_matrix
    }

    /// Returns the distance from the frustum apex to the near plane.
    pub fn near_distance(&self) -> f32 {
        self.near_plane().displacement()
    }

    /// Returns the distance from the frustum apex to the far plane.
    pub fn far_distance(&self) -> f32 {
        -self.far_plane().displacement()
    }

    /// Computes the vertical height of the frustum at the given distance from
    /// the apex towards the far plane.
    pub fn height_at_distance(&self, distance: f32) -> f32 {
        let clip_space_depth = self.convert_view_distance_to_clip_space_depth(distance);

        let top_point =
            self.inverse_transform_matrix
                .transform_point(&Point3::new(0.0, 1.0, clip_space_depth));
        let bottom_point = self.inverse_transform_matrix.transform_point(&Point3::new(
            0.0,
            -1.0,
            clip_space_depth,
        ));

        (top_point.y - bottom_point.y).abs()
    }

    /// Whether the given point is strictly inside the frustum.
    pub fn contains_point(&self, point: &Point3<f32>) -> bool {
        self.planes
            .iter()
            .all(|plane| plane.point_lies_in_positive_halfspace(point))
    }

    /// Whether any part of the given sphere could be inside the frustum. If the
    /// sphere lies close to an edge or a corner, this method may return `true`
    /// even if the sphere is really outside. However, this method is will
    /// always return `true` if the sphere is really inside. If the boundaries
    /// exactly touch each other, the sphere is considered inside.
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
                    &axis_aligned_box.corner(largest_signed_dist_corner_idx),
                ) >= 0.0
            })
    }

    /// Computes the 8 corners of the frustum.
    pub fn compute_corners(&self) -> [Point3<f32>; 8] {
        [
            self.inverse_transform_matrix
                .transform_point(&Point3::new(-1.0, -1.0, 0.0)),
            self.inverse_transform_matrix
                .transform_point(&Point3::new(-1.0, -1.0, 1.0)),
            self.inverse_transform_matrix
                .transform_point(&Point3::new(-1.0, 1.0, 0.0)),
            self.inverse_transform_matrix
                .transform_point(&Point3::new(-1.0, 1.0, 1.0)),
            self.inverse_transform_matrix
                .transform_point(&Point3::new(1.0, -1.0, 0.0)),
            self.inverse_transform_matrix
                .transform_point(&Point3::new(1.0, -1.0, 1.0)),
            self.inverse_transform_matrix
                .transform_point(&Point3::new(1.0, 1.0, 0.0)),
            self.inverse_transform_matrix
                .transform_point(&Point3::new(1.0, 1.0, 1.0)),
        ]
    }

    /// Computes the 8 corners of the part of the frustum lying between the
    /// given linear (as opposed to clip space-) depths.
    pub fn compute_corners_of_subfrustum(
        &self,
        clip_space_depth_limits: UpperExclusiveBounds<f32>,
    ) -> [Point3<f32>; 8] {
        let (lower_linear_depth, upper_linear_depth) = clip_space_depth_limits.bounds();
        let lower_clip_space_depth =
            self.convert_linear_depth_to_clip_space_depth(lower_linear_depth);
        let upper_clip_space_depth =
            self.convert_linear_depth_to_clip_space_depth(upper_linear_depth);
        [
            self.inverse_transform_matrix.transform_point(&Point3::new(
                -1.0,
                -1.0,
                lower_clip_space_depth,
            )),
            self.inverse_transform_matrix.transform_point(&Point3::new(
                -1.0,
                -1.0,
                upper_clip_space_depth,
            )),
            self.inverse_transform_matrix.transform_point(&Point3::new(
                -1.0,
                1.0,
                lower_clip_space_depth,
            )),
            self.inverse_transform_matrix.transform_point(&Point3::new(
                -1.0,
                1.0,
                upper_clip_space_depth,
            )),
            self.inverse_transform_matrix.transform_point(&Point3::new(
                1.0,
                -1.0,
                lower_clip_space_depth,
            )),
            self.inverse_transform_matrix.transform_point(&Point3::new(
                1.0,
                -1.0,
                upper_clip_space_depth,
            )),
            self.inverse_transform_matrix.transform_point(&Point3::new(
                1.0,
                1.0,
                lower_clip_space_depth,
            )),
            self.inverse_transform_matrix.transform_point(&Point3::new(
                1.0,
                1.0,
                upper_clip_space_depth,
            )),
        ]
    }

    /// Computes the clip space depth corresponding to the given distance from
    /// the frustum apex along the view direction.
    pub fn convert_view_distance_to_clip_space_depth(&self, distance: f32) -> f32 {
        self.transform_matrix
            .transform_point(&Point3::from(
                self.near_plane().unit_normal().as_ref() * distance,
            ))
            .z
    }

    /// Computes the view distance corresponding to the given clip space depth.
    pub fn convert_clip_space_depth_to_view_distance(&self, clip_space_depth: f32) -> f32 {
        self.inverse_transform_matrix
            .transform_point(&Point3::new(0.0, 0.0, clip_space_depth))
            .z
    }

    /// Computes the linear depth, which increases linearly with distance from 0
    /// at the frustum apex to 1 at the base, corresponding to the given
    /// distance from the frustum apex along the view direction.
    pub fn convert_view_distance_to_linear_depth(&self, distance: f32) -> f32 {
        distance / self.far_distance()
    }

    /// Computes the distance from the frustum apex along the view direction
    /// corresponding to the given linear depth (which increases linearly with
    /// distance from 0 at the frustum apex to 1 at the base).
    pub fn convert_linear_depth_to_view_distance(&self, linear_depth: f32) -> f32 {
        linear_depth * self.far_distance()
    }

    /// Computes the clip space depth corresponding to the given linear depth.
    pub fn convert_linear_depth_to_clip_space_depth(&self, linear_depth: f32) -> f32 {
        self.convert_view_distance_to_clip_space_depth(
            self.convert_linear_depth_to_view_distance(linear_depth),
        )
    }

    /// Computes the center point of the frustum.
    pub fn compute_center(&self) -> Point3<f32> {
        let corners = self.compute_corners();
        let n_corners = corners.len();

        corners
            .into_iter()
            .reduce(|accum, point| accum + point.coords)
            .unwrap()
            / (n_corners as f32)
    }

    /// Computes the frustum's axis-aligned bounding box.
    pub fn compute_aabb(&self) -> AxisAlignedBox {
        AxisAlignedBox::aabb_for_point_array(&self.compute_corners())
    }

    /// Computes the axis-aligned bounding box for the part of the frustum lying
    /// between the given linear (as opposed to clip space-) depths.
    pub fn compute_aabb_for_subfrustum(
        &self,
        linear_depth_limits: UpperExclusiveBounds<f32>,
    ) -> AxisAlignedBox {
        AxisAlignedBox::aabb_for_point_array(
            &self.compute_corners_of_subfrustum(linear_depth_limits),
        )
    }

    /// Computes the frustum resulting from rotating this frustum with the given
    /// rotation quaternion.
    pub fn rotated(&self, rotation: &UnitQuaternion<f32>) -> Self {
        let rotated_planes = [
            self.planes[0].rotated(rotation),
            self.planes[1].rotated(rotation),
            self.planes[2].rotated(rotation),
            self.planes[3].rotated(rotation),
            self.planes[4].rotated(rotation),
            self.planes[5].rotated(rotation),
        ];

        let largest_signed_dist_aab_corner_indices_for_planes =
            Self::determine_largest_signed_dist_aab_corner_indices_for_all_planes(rotated_planes);

        let rotated_inverse_transform_matrix =
            rotation.to_homogeneous() * self.inverse_transform_matrix;

        let inverse_of_rotated_inverse_transform_matrix =
            self.transform_matrix * rotation.inverse().to_homogeneous();

        Self {
            planes: rotated_planes,
            largest_signed_dist_aab_corner_indices_for_planes,
            transform_matrix: inverse_of_rotated_inverse_transform_matrix,
            inverse_transform_matrix: rotated_inverse_transform_matrix,
        }
    }

    /// Computes the frustum resulting from transforming this frustum with the
    /// given similarity transform.
    pub fn transformed(&self, transformation: &Similarity3) -> Self {
        let transformed_planes = self.transformed_planes(transformation);

        let largest_signed_dist_aab_corner_indices_for_planes =
            Self::determine_largest_signed_dist_aab_corner_indices_for_all_planes(
                transformed_planes,
            );

        let transformed_inverse_transform_matrix =
            transformation.to_matrix() * self.inverse_transform_matrix;

        let inverse_of_transformed_inverse_transform_matrix =
            self.transform_matrix * transformation.inverse().to_matrix();

        Self {
            planes: transformed_planes,
            largest_signed_dist_aab_corner_indices_for_planes,
            transform_matrix: inverse_of_transformed_inverse_transform_matrix,
            inverse_transform_matrix: transformed_inverse_transform_matrix,
        }
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

    fn planes_from_transform_matrix(transform_matrix: &Matrix4<f32>) -> [Plane; 6] {
        let m = transform_matrix;

        let left = Self::plane_from_unnormalized_coefficients(
            m.m41 + m.m11,
            m.m42 + m.m12,
            m.m43 + m.m13,
            -(m.m44 + m.m14),
        );
        let right = Self::plane_from_unnormalized_coefficients(
            m.m41 - m.m11,
            m.m42 - m.m12,
            m.m43 - m.m13,
            -(m.m44 - m.m14),
        );

        let bottom = Self::plane_from_unnormalized_coefficients(
            m.m41 + m.m21,
            m.m42 + m.m22,
            m.m43 + m.m23,
            -(m.m44 + m.m24),
        );
        let top = Self::plane_from_unnormalized_coefficients(
            m.m41 - m.m21,
            m.m42 - m.m22,
            m.m43 - m.m23,
            -(m.m44 - m.m24),
        );

        let near = Self::plane_from_unnormalized_coefficients(m.m31, m.m32, m.m33, -m.m34);
        let far = Self::plane_from_unnormalized_coefficients(
            m.m41 - m.m31,
            m.m42 - m.m32,
            m.m43 - m.m33,
            -(m.m44 - m.m34),
        );

        [left, right, bottom, top, near, far]
    }

    fn plane_from_unnormalized_coefficients(
        normal_x: f32,
        normal_y: f32,
        normal_z: f32,
        displacement: f32,
    ) -> Plane {
        let (unit_normal, magnitude) =
            UnitVector3::new_and_get(Vector3::new(normal_x, normal_y, normal_z));

        Plane::new(unit_normal, displacement / magnitude)
    }

    #[cfg(test)]
    fn from_transform_matrix(transform_matrix: Matrix4<f32>) -> Self {
        let planes = Self::planes_from_transform_matrix(&transform_matrix);

        let largest_signed_dist_aab_corner_indices_for_planes =
            Self::determine_largest_signed_dist_aab_corner_indices_for_all_planes(planes);

        Self {
            planes,
            largest_signed_dist_aab_corner_indices_for_planes,
            transform_matrix,
            inverse_transform_matrix: transform_matrix.try_inverse().unwrap(),
        }
    }

    /// Determines the corner of any axis-aligned bounding box that will have
    /// the largest signed distance in the space of the given plane. The corner
    /// is represented by an index following the convention of
    /// [`AxisAlignedBox::corner`].
    pub fn determine_largest_signed_dist_aab_corner_index_for_plane(plane: &Plane) -> usize {
        let normal = plane.unit_normal();
        match (
            normal.x.is_sign_negative(),
            normal.y.is_sign_negative(),
            normal.z.is_sign_negative(),
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

    fn determine_largest_signed_dist_aab_corner_indices_for_all_planes(
        planes: [Plane; 6],
    ) -> [usize; 6] {
        planes.map(|plane| Self::determine_largest_signed_dist_aab_corner_index_for_plane(&plane))
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
                .transform_matrix
                .abs_diff_eq(&other.transform_matrix, epsilon)
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
    use nalgebra::{Rotation3, Vector3};

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
    fn corners_of_transformed_frustum_equal_transformed_corners_of_original_frustum() {
        let frustum = Frustum::from_transform(
            PerspectiveTransform::new(1.0, Degrees(56.0), UpperExclusiveBounds::new(0.21, 160.2))
                .as_projective(),
        );

        let transformation = Similarity3::from_parts(
            Vector3::new(2.1, -5.9, 0.01),
            Rotation3::from_euler_angles(0.1, 0.2, 180.0).into(),
            7.0,
        );

        let transformed_frustum = frustum.transformed(&transformation);

        for (corner, corner_of_transformed) in frustum
            .compute_corners()
            .iter()
            .zip(transformed_frustum.compute_corners())
        {
            let transformed_corner = transformation.transform_point(corner);
            assert_abs_diff_eq!(transformed_corner, corner_of_transformed, epsilon = 1e-3);
        }
    }

    #[test]
    fn transforming_frustum_and_then_transforming_with_inverse_gives_original_frustum() {
        let frustum = Frustum::from_transform(
            PerspectiveTransform::new(1.0, Degrees(56.0), UpperExclusiveBounds::new(0.21, 160.2))
                .as_projective(),
        );

        let transformation = Similarity3::from_parts(
            Vector3::new(2.1, -5.9, 0.01),
            Rotation3::from_euler_angles(0.1, 0.2, 180.0).into(),
            7.0,
        );

        let transformed_frustum = frustum.transformed(&transformation);

        let untransformed_frustum = transformed_frustum.transformed(&transformation.inverse());

        assert_abs_diff_eq!(frustum, untransformed_frustum, epsilon = 1e-4);
    }

    #[test]
    fn creating_frustum_for_transform_of_transformed_frustum_gives_transformed_frustum() {
        let frustum = Frustum::from_transform(
            PerspectiveTransform::new(1.0, Degrees(56.0), UpperExclusiveBounds::new(0.21, 160.2))
                .as_projective(),
        );

        let transformation = Similarity3::from_parts(
            Vector3::new(2.1, -5.9, 0.01),
            Rotation3::from_euler_angles(0.1, 0.2, 0.3).into(),
            7.0,
        );

        let transformed_frustum = frustum.transformed(&transformation);

        let frustum_from_transformed =
            Frustum::from_transform_matrix(*transformed_frustum.transform_matrix());

        assert_abs_diff_eq!(
            transformed_frustum,
            frustum_from_transformed,
            epsilon = 1e-1
        );
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

        let corners = frustum.compute_corners_of_subfrustum(UpperExclusiveBounds::new(
            new_near_linear_depth,
            new_far_linear_depth,
        ));

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
    fn computing_orthographic_subfrustum_aabb_works() {
        let (left, right, bottom, top, near, far) = (0.1, 1.2, 2.3, 3.4, 4.5, 5.6);
        let frustum = Frustum::from_transform(
            OrthographicTransform::new(left, right, bottom, top, near, far).as_projective(),
        );

        let (new_near, new_far) = (4.9, 5.2);

        let new_near_linear_depth = frustum.convert_view_distance_to_linear_depth(new_near);
        let new_far_linear_depth = frustum.convert_view_distance_to_linear_depth(new_far);

        let aabb = frustum.compute_aabb_for_subfrustum(UpperExclusiveBounds::new(
            new_near_linear_depth,
            new_far_linear_depth,
        ));

        assert_abs_diff_eq!(
            aabb.lower_corner(),
            &Point3::new(left, bottom, new_near),
            epsilon = 1e-5
        );
        assert_abs_diff_eq!(
            aabb.upper_corner(),
            &Point3::new(right, top, new_far),
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
            let largest_signed_dist =
                plane.compute_signed_distance(&aab.corner(largest_signed_dist_aab_corner_idx));
            for corner_idx in 0..8 {
                assert!(
                    plane.compute_signed_distance(&aab.corner(corner_idx)) <= largest_signed_dist
                );
            }
        }
    }

    #[test]
    fn computing_frustum_height_at_distance_works() {
        // Test with perspective frustum with 90 degree FOV
        let frustum = Frustum::from_transform(
            PerspectiveTransform::new(1.0, Degrees(90.0), UpperExclusiveBounds::new(1.0, 10.0))
                .as_projective(),
        );

        // At distance 1.0 (near plane), the height should be 2.0 for 90 degree FOV
        // Since tan(45°) = 1, and height = 2 * distance * tan(half_fov)
        let height_at_near = frustum.height_at_distance(1.0);
        assert_abs_diff_eq!(height_at_near, 2.0, epsilon = 1e-9);

        // At distance 2.0, the height should be 4.0
        let height_at_double_distance = frustum.height_at_distance(2.0);
        assert_abs_diff_eq!(height_at_double_distance, 4.0, epsilon = 1e-9);

        // Test with orthographic frustum - height should be constant
        let (left, right, bottom, top, near, far) = (-1.0, 1.0, -2.0, 2.0, 1.0, 10.0);
        let ortho_frustum = Frustum::from_transform(
            OrthographicTransform::new(left, right, bottom, top, near, far).as_projective(),
        );

        let expected_height = top - bottom; // 4.0
        let height_at_near_ortho = ortho_frustum.height_at_distance(near);
        let height_at_mid_ortho = ortho_frustum.height_at_distance((near + far) / 2.0);
        let height_at_far_ortho = ortho_frustum.height_at_distance(far);

        assert_abs_diff_eq!(height_at_near_ortho, expected_height, epsilon = 1e-9);
        assert_abs_diff_eq!(height_at_mid_ortho, expected_height, epsilon = 1e-9);
        assert_abs_diff_eq!(height_at_far_ortho, expected_height, epsilon = 1e-9);
    }
}
