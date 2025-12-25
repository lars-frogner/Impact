//! Projection transformations.

use crate::{AxisAlignedBox, FrustumA};
use approx::assert_abs_diff_ne;
use bytemuck::{Pod, Zeroable};
use impact_math::{
    angle::{Angle, Radians},
    bounds::{Bounds, UpperExclusiveBounds},
    matrix::Matrix4A,
    point::{Point2, Point3A},
    quaternion::{QuaternionA, UnitQuaternionA},
    transform::{Projective3A, Similarity3A},
    vector::{Vector3, Vector3A},
};
use std::{f32::consts::FRAC_1_SQRT_2, fmt::Debug};

/// A perspective transformation that maps points in a view frustum pointing
/// along the negative z-axis into the cube spanning from -1 to 1 in x and y and
/// from 0 to 1 in z in normalized device coordinates.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
pub struct PerspectiveTransform {
    matrix: Matrix4A,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
pub struct OrthographicTransform {
    matrix: Matrix4A,
}

/// Projects 3D points onto a face of a cubemap.
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct CubeMapper {
    /// Rotations bringing points that lie in front of each cube face to the
    /// same relative locations with respect to the positive z face.
    pub rotations_to_positive_z_face: [UnitQuaternionA; 6],
}

/// One of the six faces of a cubemap. The enum value corresponds to the
/// conventional index of the face.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CubemapFace {
    PositiveX = 0,
    NegativeX = 1,
    PositiveY = 2,
    NegativeY = 3,
    PositiveZ = 4,
    NegativeZ = 5,
}

impl PerspectiveTransform {
    /// Creates a new perspective transformation.
    ///
    /// # Note
    /// `aspect_ratio` is the ratio of width to height of the view plane.
    ///
    /// # Panics
    /// If `aspect_ratio`, `vertical_field_of_view` or the near distance is
    /// zero.
    pub fn new<A: Angle<f32>>(
        aspect_ratio: f32,
        vertical_field_of_view: A,
        near_and_far_distance: UpperExclusiveBounds<f32>,
    ) -> Self {
        let mut transform = Self {
            matrix: Matrix4A::identity(),
        };

        transform.set_vertical_field_of_view(vertical_field_of_view);
        transform.set_aspect_ratio(aspect_ratio);
        transform.set_near_and_far_distance(near_and_far_distance);

        *transform.matrix.element_mut(3, 2) = -1.0;
        *transform.matrix.element_mut(3, 3) = 0.0;

        transform
    }

    /// Returns a reference to perspective transformation seen as a
    /// [`Projective3A`].
    #[inline]
    pub fn as_projective(&self) -> &Projective3A {
        bytemuck::cast_ref(self)
    }

    /// Returns the perspective transformation as a [`Projective3A`].
    #[inline]
    pub fn to_projective(self) -> Projective3A {
        Projective3A::from_matrix_unchecked(self.matrix)
    }

    /// Returns the ratio of width to height of the view frustum.
    #[inline]
    pub fn aspect_ratio(&self) -> f32 {
        self.matrix.element(1, 1) / self.matrix.element(0, 0)
    }

    /// Returns the vertical field of view angle in radians.
    #[inline]
    pub fn vertical_field_of_view(&self) -> Radians<f32> {
        Radians(2.0 * (1.0 / self.matrix.element(1, 1)).atan())
    }

    /// Returns the near distance of the view frustum.
    #[inline]
    pub fn near_distance(&self) -> f32 {
        self.matrix.element(2, 3) / self.matrix.element(2, 2)
    }

    /// Returns the far distance of the view frustum.
    #[inline]
    pub fn far_distance(&self) -> f32 {
        self.matrix.element(2, 3) / (1.0 + self.matrix.element(2, 2))
    }

    #[inline]
    pub fn project_point(&self, point: &Point3A) -> Point3A {
        self.matrix.project_point(point)
    }

    /// Sets the ratio of width to height of the view frustum.
    ///
    /// # Panics
    /// If `aspect_ratio` is zero.
    #[inline]
    pub fn set_aspect_ratio(&mut self, aspect_ratio: f32) {
        assert_abs_diff_ne!(aspect_ratio, 0.0);
        *self.matrix.element_mut(0, 0) = self.matrix.element(1, 1) / aspect_ratio;
    }

    /// Sets the vertical field of view angle.
    ///
    /// # Panics
    /// If `fov` is zero.
    #[inline]
    pub fn set_vertical_field_of_view<A: Angle<f32>>(&mut self, vertical_field_of_view: A) {
        let vertical_field_of_view = vertical_field_of_view.radians();
        assert_abs_diff_ne!(vertical_field_of_view, 0.0);

        let old_m22 = self.matrix.element(1, 1);
        let new_m22 = 1.0 / (0.5 * vertical_field_of_view).tan();
        *self.matrix.element_mut(1, 1) = new_m22;
        *self.matrix.element_mut(0, 0) *= new_m22 / old_m22;
    }

    #[inline]
    pub fn set_near_and_far_distance(&mut self, near_and_far_distance: UpperExclusiveBounds<f32>) {
        let (near_distance, far_distance) = near_and_far_distance.bounds();
        assert_abs_diff_ne!(near_distance, 0.0);

        *self.matrix.element_mut(2, 2) = -far_distance / (far_distance - near_distance);
        *self.matrix.element_mut(2, 3) = self.matrix.element(2, 2) * near_distance;
    }
}

impl OrthographicTransform {
    /// Creates a new orthographic transformation.
    ///
    /// # Panics
    /// If the extent of the view box along any axis is zero.
    pub fn new(left: f32, right: f32, bottom: f32, top: f32, near: f32, far: f32) -> Self {
        let mut transform = Self {
            matrix: Matrix4A::identity(),
        };

        transform.set_left_and_right(left, right);
        transform.set_bottom_and_top(bottom, top);
        transform.set_near_and_far(near, far);

        transform
    }

    /// Creates a new orthographic transformation.
    ///
    /// # Note
    /// `aspect_ratio` is the ratio of width to height of the view plane.
    ///
    /// # Panics
    /// If `aspect_ratio` or `vertical_field_of_view` is zero
    #[inline]
    pub fn with_field_of_view<A: Angle<f32>>(
        aspect_ratio: f32,
        vertical_field_of_view: A,
        near_and_far_distance: UpperExclusiveBounds<f32>,
    ) -> Self {
        let vertical_field_of_view = vertical_field_of_view.radians();
        assert_abs_diff_ne!(vertical_field_of_view, 0.0);
        assert_abs_diff_ne!(aspect_ratio, 0.0);

        let (near_distance, far_distance) = near_and_far_distance.bounds();
        let half_height = far_distance * vertical_field_of_view.tan();
        let half_width = half_height / aspect_ratio;

        Self::new(
            -half_width,
            half_width,
            -half_height,
            half_height,
            near_distance,
            far_distance,
        )
    }

    /// Creates a new orthographic transformation with the given axis-aligned
    /// box as the view box.
    #[inline]
    pub fn from_axis_aligned_box(axis_aligned_box: &AxisAlignedBox) -> Self {
        let lower = axis_aligned_box.lower_corner();
        let upper = axis_aligned_box.upper_corner();
        Self::new(
            lower.x(),
            upper.x(),
            lower.y(),
            upper.y(),
            lower.z(),
            upper.z(),
        )
    }

    /// Computes the translation and nonuniform scaling representing the
    /// orthographic transformation. Applying the translation followed by the
    /// scaling corresponds to applying the orthograpic transformation.
    #[inline]
    pub fn compute_orthographic_translation_and_scaling(
        left: f32,
        right: f32,
        bottom: f32,
        top: f32,
        near: f32,
        far: f32,
    ) -> (Vector3A, [f32; 3]) {
        (
            Vector3A::new(
                Self::compute_translation_x(left, right),
                Self::compute_translation_y(bottom, top),
                Self::compute_translation_z(near, far),
            ),
            [
                Self::compute_scaling_x(left, right),
                Self::compute_scaling_y(bottom, top),
                Self::compute_scaling_z(near, far),
            ],
        )
    }

    /// Computes the center and half extents of the orthographic view frustum
    /// represented by the given translation and nonuniform scaling.
    #[inline]
    pub fn compute_center_and_half_extents_from_translation_and_scaling(
        translation: &Vector3A,
        &[sx, sy, sz]: &[f32; 3],
    ) -> (Point3A, Vector3A) {
        (
            Point3A::new(
                -translation.x(),
                -translation.y(),
                0.5 * (1.0 / sz - 2.0 * translation.z()),
            ),
            Vector3A::new(1.0 / sx, 1.0 / sy, -0.5 / sz),
        )
    }

    /// Returns a reference to orthographic transformation seen as a
    /// [`Projective3A`].
    #[inline]
    pub fn as_projective(&self) -> &Projective3A {
        bytemuck::cast_ref(self)
    }

    /// Returns the orthographic transformation as a [`Projective3A`].
    #[inline]
    pub fn to_projective(self) -> Projective3A {
        Projective3A::from_matrix_unchecked(self.matrix)
    }

    #[inline]
    pub fn project_point(&self, point: &Point3A) -> Point3A {
        self.matrix.project_point(point)
    }

    #[inline]
    pub fn set_left_and_right(&mut self, left: f32, right: f32) {
        assert_abs_diff_ne!(left, right);
        let translation_x = Self::compute_translation_x(left, right);
        let scaling_x = Self::compute_scaling_x(left, right);
        *self.matrix.element_mut(0, 0) = scaling_x;
        *self.matrix.element_mut(0, 3) = scaling_x * translation_x;
    }

    #[inline]
    pub fn set_bottom_and_top(&mut self, bottom: f32, top: f32) {
        assert_abs_diff_ne!(bottom, top);
        let translation_y = Self::compute_translation_y(bottom, top);
        let scaling_y = Self::compute_scaling_y(bottom, top);
        *self.matrix.element_mut(1, 1) = scaling_y;
        *self.matrix.element_mut(1, 3) = scaling_y * translation_y;
    }

    #[inline]
    pub fn set_near_and_far(&mut self, near: f32, far: f32) {
        assert_abs_diff_ne!(near, far);
        let translation_z = Self::compute_translation_z(near, far);
        let scaling_z = Self::compute_scaling_z(near, far);
        *self.matrix.element_mut(2, 2) = scaling_z;
        *self.matrix.element_mut(2, 3) = scaling_z * translation_z;
    }

    #[inline]
    fn compute_translation_x(left: f32, right: f32) -> f32 {
        -0.5 * (left + right)
    }

    #[inline]
    fn compute_translation_y(bottom: f32, top: f32) -> f32 {
        -0.5 * (bottom + top)
    }

    #[inline]
    fn compute_translation_z(near: f32, _far: f32) -> f32 {
        -near
    }

    #[inline]
    fn compute_scaling_x(left: f32, right: f32) -> f32 {
        2.0 / (right - left)
    }

    #[inline]
    fn compute_scaling_y(bottom: f32, top: f32) -> f32 {
        2.0 / (top - bottom)
    }

    #[inline]
    fn compute_scaling_z(near: f32, far: f32) -> f32 {
        1.0 / (far - near)
    }
}

impl CubeMapper {
    /// Quaternions representing the rotation from each of the six cube faces to
    /// the positive z face. That is, a point with a certain texture coordinate
    /// within a cube face would, after being rotated with the corresponding
    /// rotation here, have the same texture coordinate within the positive z
    /// face.
    const ROTATIONS_TO_POSITIVE_Z_FACE: [UnitQuaternionA; 6] = [
        // From positive x face:
        // UnitQuaternion::from_axis_angle(&UnitVector3::unit_y(), -0.5 * PI)
        UnitQuaternionA::unchecked_from(QuaternionA::from_parts(
            Vector3::new(0.0, -FRAC_1_SQRT_2, 0.0),
            FRAC_1_SQRT_2,
        )),
        // From negative x face:
        // UnitQuaternion::from_axis_angle(&UnitVector3::unit_y(), 0.5 * PI)
        UnitQuaternionA::unchecked_from(QuaternionA::from_parts(
            Vector3::new(0.0, FRAC_1_SQRT_2, 0.0),
            FRAC_1_SQRT_2,
        )),
        // From positive y face:
        // UnitQuaternion::from_axis_angle(&UnitVector3::unit_x(), 0.5 * PI)
        UnitQuaternionA::unchecked_from(QuaternionA::from_parts(
            Vector3::new(FRAC_1_SQRT_2, 0.0, 0.0),
            FRAC_1_SQRT_2,
        )),
        // From negative y face:
        // UnitQuaternion::from_axis_angle(&UnitVector3::unit_x(), -0.5 * PI)
        UnitQuaternionA::unchecked_from(QuaternionA::from_parts(
            Vector3::new(-FRAC_1_SQRT_2, 0.0, 0.0),
            FRAC_1_SQRT_2,
        )),
        // From positive z face:
        // UnitQuaternion::identity()
        UnitQuaternionA::unchecked_from(QuaternionA::from_parts(Vector3::new(0.0, 0.0, 0.0), 1.0)),
        // From negative z face:
        // UnitQuaternion::from_axis_angle(&UnitVector3::unit_y(), PI)
        UnitQuaternionA::unchecked_from(QuaternionA::from_parts(Vector3::new(0.0, 1.0, 0.0), 0.0)),
    ];

    /// Returns a quaternion representing the rotation from the given cube face
    /// to the positive z face. That is, a point with a certain texture
    /// coordinate within the given cube face would, after being rotated with
    /// the returned rotation, have the same texture coordinate within the
    /// positive z face.
    #[inline]
    pub const fn rotation_to_positive_z_face_from_face(face: CubemapFace) -> UnitQuaternionA {
        Self::ROTATIONS_TO_POSITIVE_Z_FACE[face.as_idx_usize()]
    }

    /// Computes the cubemap-space frustum for the positive z cubemap face,
    /// using the given near and far distance.
    pub fn compute_frustum_for_positive_z_face(near_distance: f32, far_distance: f32) -> FrustumA {
        let (projection_matrix, inverse_projection_matrix) =
            Self::create_projection_matrix_and_inverse_for_positive_z_face(
                near_distance,
                far_distance,
            );

        FrustumA::from_transform_matrix_with_inverse(projection_matrix, inverse_projection_matrix)
    }

    /// Computes the frustum for the given cubemap face, using the given
    /// transform to cubemap space (defining the position and orientation of the
    /// full cubemap in the parent space) and the given near and far distance.
    pub fn compute_transformed_frustum_for_face(
        face: CubemapFace,
        transform_to_cube_space: &Similarity3A,
        near_distance: f32,
        far_distance: f32,
    ) -> FrustumA {
        let (view_projection_matrix, inverse_view_projection_matrix) =
            Self::compute_view_projection_matrix_and_inverse_for_face(
                face,
                transform_to_cube_space,
                near_distance,
                far_distance,
            );

        FrustumA::from_transform_matrix_with_inverse(
            view_projection_matrix,
            inverse_view_projection_matrix,
        )
    }

    /// Creates a new mapper for 3D points onto a cubemap.
    ///
    /// The given rotation to cube space will be applied to each point prior to
    /// projection onto a cubemap face.
    #[inline]
    pub fn new(rotation_to_cube_space: UnitQuaternionA) -> Self {
        let rotations_to_positive_z_face = [
            Self::ROTATIONS_TO_POSITIVE_Z_FACE[0] * rotation_to_cube_space,
            Self::ROTATIONS_TO_POSITIVE_Z_FACE[1] * rotation_to_cube_space,
            Self::ROTATIONS_TO_POSITIVE_Z_FACE[2] * rotation_to_cube_space,
            Self::ROTATIONS_TO_POSITIVE_Z_FACE[3] * rotation_to_cube_space,
            Self::ROTATIONS_TO_POSITIVE_Z_FACE[4] * rotation_to_cube_space,
            Self::ROTATIONS_TO_POSITIVE_Z_FACE[5] * rotation_to_cube_space,
        ];

        Self {
            rotations_to_positive_z_face,
        }
    }

    /// Creates a new mapper for 3D points onto a cubemap.
    ///
    /// Points to project must be specified in the coordinate system of the
    /// cubemap.
    #[inline]
    pub fn new_in_cube_space() -> Self {
        Self::new(UnitQuaternionA::identity())
    }

    /// Projects the given 3D point onto the given cubemap face, producing a 2D
    /// point whose x- and y-coordinates correspond to offsets from the face
    /// center in a plane parallel to the face, with the orientations of the
    /// axes following the cubemap conventions.
    ///
    /// If the x- or y-coordinate after projection lies outside the -1.0 to 1.0
    /// range, the point belongs to another face.
    pub fn map_point_onto_face(&self, face: CubemapFace, point: &Point3A) -> Point2 {
        let rotated_point =
            self.rotations_to_positive_z_face[face.as_idx_usize()].rotate_point(point);
        Self::map_point_to_positive_z_face(&rotated_point)
    }

    fn compute_view_projection_matrix_and_inverse_for_face(
        face: CubemapFace,
        view_transform: &Similarity3A,
        near_distance: f32,
        far_distance: f32,
    ) -> (Matrix4A, Matrix4A) {
        let (projection_matrix_for_positive_z_face, inverse_projection_matrix_for_positive_z_face) =
            Self::create_projection_matrix_and_inverse_for_positive_z_face(
                near_distance,
                far_distance,
            );

        let complete_view_transform =
            view_transform.rotated(&Self::ROTATIONS_TO_POSITIVE_Z_FACE[face.as_idx_usize()]);

        let view_projection_matrix =
            projection_matrix_for_positive_z_face * complete_view_transform.to_matrix();

        let inverse_view_projection_matrix = complete_view_transform.inverted().to_matrix()
            * inverse_projection_matrix_for_positive_z_face;

        (view_projection_matrix, inverse_view_projection_matrix)
    }

    fn create_projection_matrix_and_inverse_for_positive_z_face(
        near_distance: f32,
        far_distance: f32,
    ) -> (Matrix4A, Matrix4A) {
        let mut matrix = Matrix4A::identity();

        let inverse_distance_span = 1.0 / (far_distance - near_distance);

        *matrix.element_mut(2, 2) = far_distance * inverse_distance_span;
        *matrix.element_mut(2, 3) = -matrix.element(2, 2) * near_distance;

        *matrix.element_mut(3, 2) = 1.0;
        *matrix.element_mut(3, 3) = 0.0;

        let mut inverse_matrix = Matrix4A::identity();

        *inverse_matrix.element_mut(2, 2) = 0.0;
        *inverse_matrix.element_mut(2, 3) = 1.0;

        *inverse_matrix.element_mut(3, 2) = 1.0 / matrix.element(2, 3);
        *inverse_matrix.element_mut(3, 3) = -matrix.element(2, 2) * inverse_matrix.element(3, 2);

        (matrix, inverse_matrix)
    }

    #[inline]
    fn map_point_to_positive_z_face(point: &Point3A) -> Point2 {
        let inverse_point_z = 1.0 / point.z();
        Point2::new(point.x() * inverse_point_z, point.y() * inverse_point_z)
    }
}

impl CubemapFace {
    /// Returns an array with each face in the conventional order.
    #[inline]
    pub const fn all() -> [Self; 6] {
        [
            Self::PositiveX,
            Self::NegativeX,
            Self::PositiveY,
            Self::NegativeY,
            Self::PositiveZ,
            Self::NegativeZ,
        ]
    }

    /// Returns the index of the face according the conventional ordering as a
    /// [`u32`].
    #[inline]
    pub const fn as_idx_u32(&self) -> u32 {
        *self as u32
    }

    /// Returns the index of the face according the conventional ordering as a
    /// [`usize`].
    #[inline]
    pub const fn as_idx_usize(&self) -> usize {
        *self as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_abs_diff_eq;
    use impact_math::angle::Degrees;

    #[test]
    #[should_panic]
    fn constructing_perspective_transform_with_zero_aspect_ratio() {
        PerspectiveTransform::new(0.0, Degrees(45.0), UpperExclusiveBounds::new(0.1, 100.0));
    }

    #[test]
    #[should_panic]
    fn constructing_perspective_transform_with_zero_vertical_fov() {
        PerspectiveTransform::new(1.0, Degrees(0.0), UpperExclusiveBounds::new(0.1, 100.0));
    }

    #[test]
    fn setting_perspective_transform_aspect_ratio_works() {
        let mut transform =
            PerspectiveTransform::new(1.0, Degrees(45.0), UpperExclusiveBounds::new(0.1, 100.0));
        assert_abs_diff_eq!(transform.aspect_ratio(), 1.0);
        transform.set_aspect_ratio(0.5);
        assert_abs_diff_eq!(transform.aspect_ratio(), 0.5);
    }

    #[test]
    fn setting_perspective_transform_vertical_field_of_view_works() {
        let mut transform =
            PerspectiveTransform::new(1.0, Degrees(45.0), UpperExclusiveBounds::new(0.1, 100.0));
        assert_abs_diff_eq!(transform.vertical_field_of_view(), Degrees(45.0));
        transform.set_vertical_field_of_view(Degrees(90.0));
        assert_abs_diff_eq!(transform.vertical_field_of_view(), Degrees(90.0));
    }

    #[test]
    fn setting_perspective_transform_near_and_far_distance_works() {
        let mut transform =
            PerspectiveTransform::new(1.0, Degrees(45.0), UpperExclusiveBounds::new(0.1, 100.0));
        assert_abs_diff_eq!(transform.near_distance(), 0.1);
        assert_abs_diff_eq!(transform.far_distance(), 100.0, epsilon = 1e-4);
        transform.set_near_and_far_distance(UpperExclusiveBounds::new(42.0, 256.0));
        assert_abs_diff_eq!(transform.near_distance(), 42.0);
        assert_abs_diff_eq!(transform.far_distance(), 256.0, epsilon = 1e-4);
    }

    #[test]
    fn transforming_point_with_perspective_transform_works() {
        let transform =
            PerspectiveTransform::new(1.0, Degrees(45.0), UpperExclusiveBounds::new(0.1, 100.0));

        let point = Point3A::new(1.2, 2.4, 1.8);

        assert_abs_diff_eq!(
            transform.project_point(&point),
            transform.as_projective().project_point(&point),
            epsilon = 1e-6
        );
    }

    #[test]
    fn perspective_transform_near_plane_maps_to_zero() {
        let near_distance = 0.01;
        let far_distance = 100.0;
        let transform = PerspectiveTransform::new(
            1.0,
            Degrees(45.0),
            UpperExclusiveBounds::new(near_distance, far_distance),
        );

        let point = Point3A::new(0.0, 0.0, -near_distance);
        assert_abs_diff_eq!(transform.project_point(&point).z(), 0.0);
    }

    #[test]
    fn perspective_transform_far_plane_maps_to_one() {
        let near_distance = 0.01;
        let far_distance = 100.0;
        let transform = PerspectiveTransform::new(
            1.0,
            Degrees(45.0),
            UpperExclusiveBounds::new(near_distance, far_distance),
        );

        let point = Point3A::new(0.0, 0.0, -far_distance);
        assert_abs_diff_eq!(transform.project_point(&point).z(), 1.0);
    }

    #[test]
    fn mapping_to_positive_x_cubemap_face_works() {
        let mapper = CubeMapper::new_in_cube_space();

        let near = 0.1;
        let far = 10.0;

        assert_abs_diff_eq!(
            mapper.map_point_onto_face(CubemapFace::PositiveX, &Point3A::new(far, far, far)),
            Point2::new(-1.0, 1.0),
            epsilon = 1e-5
        );
        assert_abs_diff_eq!(
            mapper.map_point_onto_face(CubemapFace::PositiveX, &Point3A::new(far, -far, -far)),
            Point2::new(1.0, -1.0),
            epsilon = 1e-5
        );
        assert_abs_diff_eq!(
            mapper.map_point_onto_face(CubemapFace::PositiveX, &Point3A::new(near, near, near)),
            Point2::new(-1.0, 1.0),
            epsilon = 1e-5
        );
        assert_abs_diff_eq!(
            mapper.map_point_onto_face(CubemapFace::PositiveX, &Point3A::new(near, -near, -near)),
            Point2::new(1.0, -1.0),
            epsilon = 1e-5
        );
    }

    #[test]
    fn mapping_to_negative_x_cubemap_face_works() {
        let mapper = CubeMapper::new_in_cube_space();

        let near = 0.1;
        let far = 10.0;

        assert_abs_diff_eq!(
            mapper.map_point_onto_face(CubemapFace::NegativeX, &Point3A::new(-far, far, far)),
            Point2::new(1.0, 1.0),
            epsilon = 1e-5
        );
        assert_abs_diff_eq!(
            mapper.map_point_onto_face(CubemapFace::NegativeX, &Point3A::new(-far, -far, -far)),
            Point2::new(-1.0, -1.0),
            epsilon = 1e-5
        );
        assert_abs_diff_eq!(
            mapper.map_point_onto_face(CubemapFace::NegativeX, &Point3A::new(-near, near, near)),
            Point2::new(1.0, 1.0),
            epsilon = 1e-5
        );
        assert_abs_diff_eq!(
            mapper.map_point_onto_face(CubemapFace::NegativeX, &Point3A::new(-near, -near, -near)),
            Point2::new(-1.0, -1.0),
            epsilon = 1e-5
        );
    }

    #[test]
    fn mapping_to_positive_y_cubemap_face_works() {
        let mapper = CubeMapper::new_in_cube_space();

        let near = 0.1;
        let far = 10.0;

        assert_abs_diff_eq!(
            mapper.map_point_onto_face(CubemapFace::PositiveY, &Point3A::new(far, far, far)),
            Point2::new(1.0, -1.0),
            epsilon = 1e-5
        );
        assert_abs_diff_eq!(
            mapper.map_point_onto_face(CubemapFace::PositiveY, &Point3A::new(-far, far, -far)),
            Point2::new(-1.0, 1.0),
            epsilon = 1e-5
        );
        assert_abs_diff_eq!(
            mapper.map_point_onto_face(CubemapFace::PositiveY, &Point3A::new(near, near, near)),
            Point2::new(1.0, -1.0),
            epsilon = 1e-5
        );
        assert_abs_diff_eq!(
            mapper.map_point_onto_face(CubemapFace::PositiveY, &Point3A::new(-near, near, -near)),
            Point2::new(-1.0, 1.0),
            epsilon = 1e-5
        );
    }

    #[test]
    fn mapping_to_negative_y_cubemap_face_works() {
        let mapper = CubeMapper::new_in_cube_space();

        let near = 0.1;
        let far = 10.0;

        assert_abs_diff_eq!(
            mapper.map_point_onto_face(CubemapFace::NegativeY, &Point3A::new(far, -far, far)),
            Point2::new(1.0, 1.0),
            epsilon = 1e-5
        );
        assert_abs_diff_eq!(
            mapper.map_point_onto_face(CubemapFace::NegativeY, &Point3A::new(-far, -far, -far)),
            Point2::new(-1.0, -1.0),
            epsilon = 1e-5
        );
        assert_abs_diff_eq!(
            mapper.map_point_onto_face(CubemapFace::NegativeY, &Point3A::new(near, -near, near)),
            Point2::new(1.0, 1.0),
            epsilon = 1e-5
        );
        assert_abs_diff_eq!(
            mapper.map_point_onto_face(CubemapFace::NegativeY, &Point3A::new(-near, -near, -near)),
            Point2::new(-1.0, -1.0),
            epsilon = 1e-5
        );
    }

    #[test]
    fn mapping_to_positive_z_cubemap_face_works() {
        let mapper = CubeMapper::new_in_cube_space();

        let near = 0.1;
        let far = 10.0;

        assert_abs_diff_eq!(
            mapper.map_point_onto_face(CubemapFace::PositiveZ, &Point3A::new(far, far, far)),
            Point2::new(1.0, 1.0),
            epsilon = 1e-5
        );
        assert_abs_diff_eq!(
            mapper.map_point_onto_face(CubemapFace::PositiveZ, &Point3A::new(-far, -far, far)),
            Point2::new(-1.0, -1.0),
            epsilon = 1e-5
        );
        assert_abs_diff_eq!(
            mapper.map_point_onto_face(CubemapFace::PositiveZ, &Point3A::new(near, near, near)),
            Point2::new(1.0, 1.0),
            epsilon = 1e-5
        );
        assert_abs_diff_eq!(
            mapper.map_point_onto_face(CubemapFace::PositiveZ, &Point3A::new(-near, -near, near)),
            Point2::new(-1.0, -1.0),
            epsilon = 1e-5
        );
    }

    #[test]
    fn mapping_to_negative_z_cubemap_face_works() {
        let mapper = CubeMapper::new_in_cube_space();

        let near = 0.1;
        let far = 10.0;

        assert_abs_diff_eq!(
            mapper.map_point_onto_face(CubemapFace::NegativeZ, &Point3A::new(far, far, -far)),
            Point2::new(-1.0, 1.0),
            epsilon = 1e-5
        );
        assert_abs_diff_eq!(
            mapper.map_point_onto_face(CubemapFace::NegativeZ, &Point3A::new(-far, -far, -far)),
            Point2::new(1.0, -1.0),
            epsilon = 1e-5
        );
        assert_abs_diff_eq!(
            mapper.map_point_onto_face(CubemapFace::NegativeZ, &Point3A::new(near, near, -near)),
            Point2::new(-1.0, 1.0),
            epsilon = 1e-5
        );
        assert_abs_diff_eq!(
            mapper.map_point_onto_face(CubemapFace::NegativeZ, &Point3A::new(-near, -near, -near)),
            Point2::new(1.0, -1.0),
            epsilon = 1e-5
        );
    }

    #[test]
    fn mapping_to_positive_x_cubemap_face_with_frustum_works() {
        let near = 0.1;
        let far = 10.0;

        let frustum = CubeMapper::compute_transformed_frustum_for_face(
            CubemapFace::PositiveX,
            &Similarity3A::identity(),
            near,
            far,
        );
        let projection = frustum.transform_matrix();

        assert_abs_diff_eq!(
            projection.project_point(&Point3A::new(far, far, far)).xy(),
            Point2::new(-1.0, 1.0),
            epsilon = 1e-5
        );
        assert_abs_diff_eq!(
            projection
                .project_point(&Point3A::new(far, -far, -far))
                .xy(),
            Point2::new(1.0, -1.0),
            epsilon = 1e-5
        );
        assert_abs_diff_eq!(
            projection
                .project_point(&Point3A::new(near, near, near))
                .xy(),
            Point2::new(-1.0, 1.0),
            epsilon = 1e-5
        );
        assert_abs_diff_eq!(
            projection
                .project_point(&Point3A::new(near, -near, -near))
                .xy(),
            Point2::new(1.0, -1.0),
            epsilon = 1e-5
        );
    }

    #[test]
    fn mapping_to_negative_x_cubemap_face_with_frustum_works() {
        let near = 0.1;
        let far = 10.0;

        let frustum = CubeMapper::compute_transformed_frustum_for_face(
            CubemapFace::NegativeX,
            &Similarity3A::identity(),
            near,
            far,
        );
        let projection = frustum.transform_matrix();

        assert_abs_diff_eq!(
            projection.project_point(&Point3A::new(-far, far, far)).xy(),
            Point2::new(1.0, 1.0),
            epsilon = 1e-5
        );
        assert_abs_diff_eq!(
            projection
                .project_point(&Point3A::new(-far, -far, -far))
                .xy(),
            Point2::new(-1.0, -1.0),
            epsilon = 1e-5
        );
        assert_abs_diff_eq!(
            projection
                .project_point(&Point3A::new(-near, near, near))
                .xy(),
            Point2::new(1.0, 1.0),
            epsilon = 1e-5
        );
        assert_abs_diff_eq!(
            projection
                .project_point(&Point3A::new(-near, -near, -near))
                .xy(),
            Point2::new(-1.0, -1.0),
            epsilon = 1e-5
        );
    }

    #[test]
    fn mapping_to_positive_y_cubemap_face_with_frustum_works() {
        let near = 0.1;
        let far = 10.0;

        let frustum = CubeMapper::compute_transformed_frustum_for_face(
            CubemapFace::PositiveY,
            &Similarity3A::identity(),
            near,
            far,
        );
        let projection = frustum.transform_matrix();

        assert_abs_diff_eq!(
            projection.project_point(&Point3A::new(far, far, far)).xy(),
            Point2::new(1.0, -1.0),
            epsilon = 1e-5
        );
        assert_abs_diff_eq!(
            projection
                .project_point(&Point3A::new(-far, far, -far))
                .xy(),
            Point2::new(-1.0, 1.0),
            epsilon = 1e-5
        );
        assert_abs_diff_eq!(
            projection
                .project_point(&Point3A::new(near, near, near))
                .xy(),
            Point2::new(1.0, -1.0),
            epsilon = 1e-5
        );
        assert_abs_diff_eq!(
            projection
                .project_point(&Point3A::new(-near, near, -near))
                .xy(),
            Point2::new(-1.0, 1.0),
            epsilon = 1e-5
        );
    }

    #[test]
    fn mapping_to_negative_y_cubemap_face_with_frustum_works() {
        let near = 0.1;
        let far = 10.0;

        let frustum = CubeMapper::compute_transformed_frustum_for_face(
            CubemapFace::NegativeY,
            &Similarity3A::identity(),
            near,
            far,
        );
        let projection = frustum.transform_matrix();

        assert_abs_diff_eq!(
            projection.project_point(&Point3A::new(far, -far, far)).xy(),
            Point2::new(1.0, 1.0),
            epsilon = 1e-5
        );
        assert_abs_diff_eq!(
            projection
                .project_point(&Point3A::new(-far, -far, -far))
                .xy(),
            Point2::new(-1.0, -1.0),
            epsilon = 1e-5
        );
        assert_abs_diff_eq!(
            projection
                .project_point(&Point3A::new(near, -near, near))
                .xy(),
            Point2::new(1.0, 1.0),
            epsilon = 1e-5
        );
        assert_abs_diff_eq!(
            projection
                .project_point(&Point3A::new(-near, -near, -near))
                .xy(),
            Point2::new(-1.0, -1.0),
            epsilon = 1e-5
        );
    }

    #[test]
    fn mapping_to_positive_z_cubemap_face_with_frustum_works() {
        let near = 0.1;
        let far = 10.0;

        let frustum = CubeMapper::compute_transformed_frustum_for_face(
            CubemapFace::PositiveZ,
            &Similarity3A::identity(),
            near,
            far,
        );
        let projection = frustum.transform_matrix();

        assert_abs_diff_eq!(
            projection.project_point(&Point3A::new(far, far, far)).xy(),
            Point2::new(1.0, 1.0),
            epsilon = 1e-5
        );
        assert_abs_diff_eq!(
            projection
                .project_point(&Point3A::new(-far, -far, far))
                .xy(),
            Point2::new(-1.0, -1.0),
            epsilon = 1e-5
        );
        assert_abs_diff_eq!(
            projection
                .project_point(&Point3A::new(near, near, near))
                .xy(),
            Point2::new(1.0, 1.0),
            epsilon = 1e-5
        );
        assert_abs_diff_eq!(
            projection
                .project_point(&Point3A::new(-near, -near, near))
                .xy(),
            Point2::new(-1.0, -1.0),
            epsilon = 1e-5
        );
    }

    #[test]
    fn mapping_to_negative_z_cubemap_face_with_frustum_works() {
        let near = 0.1;
        let far = 10.0;

        let frustum = CubeMapper::compute_transformed_frustum_for_face(
            CubemapFace::NegativeZ,
            &Similarity3A::identity(),
            near,
            far,
        );
        let projection = frustum.transform_matrix();

        assert_abs_diff_eq!(
            projection.project_point(&Point3A::new(far, far, -far)).xy(),
            Point2::new(-1.0, 1.0),
            epsilon = 1e-5
        );
        assert_abs_diff_eq!(
            projection
                .project_point(&Point3A::new(-far, -far, -far))
                .xy(),
            Point2::new(1.0, -1.0),
            epsilon = 1e-5
        );
        assert_abs_diff_eq!(
            projection
                .project_point(&Point3A::new(near, near, -near))
                .xy(),
            Point2::new(-1.0, 1.0),
            epsilon = 1e-5
        );
        assert_abs_diff_eq!(
            projection
                .project_point(&Point3A::new(-near, -near, -near))
                .xy(),
            Point2::new(1.0, -1.0),
            epsilon = 1e-5
        );
    }

    #[test]
    fn computed_frusta_for_different_cubemap_faces_are_consistent() {
        let positive_z_frustum = CubeMapper::compute_transformed_frustum_for_face(
            CubemapFace::PositiveZ,
            &Similarity3A::identity(),
            0.1,
            10.0,
        );

        for face in CubemapFace::all() {
            let frustum_rotated_to_positive_z = CubeMapper::compute_transformed_frustum_for_face(
                face,
                &Similarity3A::identity(),
                0.1,
                10.0,
            )
            .rotated(&CubeMapper::ROTATIONS_TO_POSITIVE_Z_FACE[face.as_idx_usize()]);

            assert_abs_diff_eq!(
                &frustum_rotated_to_positive_z,
                &positive_z_frustum,
                epsilon = 1e-4
            );
        }
    }
}
