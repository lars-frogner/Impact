//! Projection transformations.

use crate::{
    geometry::{Angle, Bounds, Radians, UpperExclusiveBounds},
    num::Float,
};
use approx::assert_abs_diff_ne;
use bytemuck::{Pod, Zeroable};
use nalgebra::{
    vector, Matrix4, Point3, Projective3, Quaternion, Scale3, Similarity3, Translation3,
    UnitQuaternion, Vector3,
};
use std::fmt::Debug;

/// A perspective transformation that maps points in a view frustum pointing
/// along the negative z-axis into the cube spanning from -1 to 1 in x and y and
/// from 0 to 1 in z in normalized device coordinates, with a flipped x-axis.
///
/// The reason for flipping the x-axis is to make it so that points with
/// positive x-coordinates in view space gets projected to the left of the
/// screen and vice versa, which is the intuitive behavior for a camera looking
/// down the negative z-axis in view space.
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct PerspectiveTransform<F: Float> {
    matrix: Matrix4<F>,
}
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct OrthographicTransform<F: Float> {
    matrix: Matrix4<F>,
}

/// Projects 3D points onto a face of a cubemap.
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct CubeMapper<F: Float> {
    transform_to_positive_z_face: Similarity3<F>,
    z_scaling: F,
    z_translation: F,
}

/// One of the six faces of a cubemap. The enum value corresponds to the
/// conventional index of the face.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CubeMapFace {
    PositiveX = 0,
    NegativeX = 1,
    PositiveY = 2,
    NegativeY = 3,
    PositiveZ = 4,
    NegativeZ = 5,
}

impl<F: Float> PerspectiveTransform<F> {
    /// Creates a new perspective transformation.
    ///
    /// # Note
    /// `aspect_ratio` is the ratio of width to height of the view plane.
    ///
    /// # Panics
    /// If `aspect_ratio` or `vertical_field_of_view` is zero.
    pub fn new<A: Angle<F>>(
        aspect_ratio: F,
        vertical_field_of_view: A,
        near_and_far_distance: UpperExclusiveBounds<F>,
    ) -> Self {
        let mut transform = Self {
            matrix: Matrix4::identity(),
        };

        transform.set_vertical_field_of_view(vertical_field_of_view);
        transform.set_aspect_ratio(aspect_ratio);
        transform.set_near_and_far_distance(near_and_far_distance);

        transform.matrix.m43 = -F::ONE;
        transform.matrix.m44 = F::ZERO;

        transform
    }

    /// Returns a reference to perspective transformation seen as a
    /// [`Projective3`].
    pub fn as_projective(&self) -> &Projective3<F> {
        unsafe { &*(self as *const Self).cast::<Projective3<F>>() }
    }

    /// Returns the perspective transformation as a [`Projective3`].
    pub fn to_projective(self) -> Projective3<F> {
        Projective3::from_matrix_unchecked(self.matrix)
    }

    /// Returns the ratio of width to height of the view frustum.
    pub fn aspect_ratio(&self) -> F {
        -self.matrix.m22 / self.matrix.m11
    }

    /// Returns the vertical field of view angle in radians.
    pub fn vertical_field_of_view(&self) -> Radians<F> {
        Radians(F::TWO * F::atan(F::ONE / self.matrix.m22))
    }

    /// Returns the near distance of the view frustum.
    pub fn near_distance(&self) -> F {
        self.matrix.m34 / self.matrix.m33
    }

    /// Returns the far distance of the view frustum.
    pub fn far_distance(&self) -> F {
        self.matrix.m34 / (F::ONE + self.matrix.m33)
    }

    pub fn transform_point(&self, point: &Point3<F>) -> Point3<F> {
        let inverse_denom = -F::ONE / point.z;
        Point3::new(
            self.matrix.m11 * point.x * inverse_denom,
            self.matrix.m22 * point.y * inverse_denom,
            (self.matrix.m33 * point.z + self.matrix.m34) * inverse_denom,
        )
    }

    pub fn transform_vector(&self, vector: &Vector3<F>) -> Vector3<F> {
        let inverse_denom = -F::ONE / vector.z;
        Vector3::new(
            self.matrix.m11 * vector.x * inverse_denom,
            self.matrix.m22 * vector.y * inverse_denom,
            -self.matrix.m33,
        )
    }

    /// Sets the ratio of width to height of the view frustum.
    ///
    /// # Panics
    /// If `aspect_ratio` is zero.
    pub fn set_aspect_ratio(&mut self, aspect_ratio: F) {
        assert_abs_diff_ne!(aspect_ratio, F::zero());
        self.matrix.m11 = -self.matrix.m22 / aspect_ratio;
    }

    /// Sets the vertical field of view angle.
    ///
    /// # Panics
    /// If `fov` is zero.
    pub fn set_vertical_field_of_view<A: Angle<F>>(&mut self, vertical_field_of_view: A) {
        let vertical_field_of_view = vertical_field_of_view.radians();
        assert_abs_diff_ne!(vertical_field_of_view, F::ZERO);

        let old_m22 = self.matrix.m22;
        let new_m22 = F::ONE / F::tan(F::ONE_HALF * vertical_field_of_view);
        self.matrix.m22 = new_m22;
        self.matrix.m11 *= new_m22 / old_m22;
    }

    pub fn set_near_and_far_distance(&mut self, near_and_far_distance: UpperExclusiveBounds<F>) {
        let (near_distance, far_distance) = near_and_far_distance.bounds();

        let inverse_depth_span = F::ONE / (near_distance - far_distance);

        self.matrix.m33 =
            F::ONE_HALF * ((near_distance + far_distance) * inverse_depth_span - F::ONE);
        self.matrix.m34 = far_distance * near_distance * inverse_depth_span;
    }
}

impl<F: Float> OrthographicTransform<F> {
    /// Creates a new orthographic transformation.
    ///
    /// # Panics
    /// If the extent of the view box along any axis is zero.
    pub fn new(left: F, right: F, bottom: F, top: F, near: F, far: F) -> Self {
        let mut transform = Self {
            matrix: Matrix4::identity(),
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
    pub fn with_field_of_view<A: Angle<F>>(
        aspect_ratio: F,
        vertical_field_of_view: A,
        near_and_far_distance: UpperExclusiveBounds<F>,
    ) -> Self {
        let vertical_field_of_view = vertical_field_of_view.radians();
        assert_abs_diff_ne!(vertical_field_of_view, F::ZERO);
        assert_abs_diff_ne!(aspect_ratio, F::zero());

        let (near_distance, far_distance) = near_and_far_distance.bounds();
        let half_height = far_distance * F::tan(vertical_field_of_view);
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

    /// Computes the translation and nonuniform scaling representing the
    /// orthographic transformation. Applying the translation followed by the
    /// scaling corresponds to applying the orthograpic transformation.
    pub fn compute_orthographic_translation_and_scaling(
        left: F,
        right: F,
        bottom: F,
        top: F,
        near: F,
        far: F,
    ) -> (Translation3<F>, Scale3<F>) {
        (
            Translation3::new(
                Self::compute_translation_x(left, right),
                Self::compute_translation_y(bottom, top),
                Self::compute_translation_z(near, far),
            ),
            Scale3::new(
                Self::compute_scaling_x(left, right),
                Self::compute_scaling_y(bottom, top),
                Self::compute_scaling_z(near, far),
            ),
        )
    }

    /// Returns a reference to orthographic transformation seen as a
    /// [`Projective3`].
    pub fn as_projective(&self) -> &Projective3<F> {
        unsafe { &*(self as *const Self).cast::<Projective3<F>>() }
    }

    /// Returns the orthographic transformation as a [`Projective3`].
    pub fn to_projective(self) -> Projective3<F> {
        Projective3::from_matrix_unchecked(self.matrix)
    }

    pub fn transform_point(&self, point: &Point3<F>) -> Point3<F> {
        Point3::new(
            self.matrix.m11 * point.x + self.matrix.m14,
            self.matrix.m22 * point.y + self.matrix.m24,
            self.matrix.m33 * point.z + self.matrix.m34,
        )
    }

    pub fn transform_vector(&self, vector: &Vector3<F>) -> Vector3<F> {
        Vector3::new(
            self.matrix.m11 * vector.x,
            self.matrix.m22 * vector.y,
            self.matrix.m33 * vector.z,
        )
    }

    pub fn set_left_and_right(&mut self, left: F, right: F) {
        assert_abs_diff_ne!(left, right);
        let translation_x = Self::compute_translation_x(left, right);
        let scaling_x = Self::compute_scaling_x(left, right);
        self.matrix.m11 = scaling_x;
        self.matrix.m14 = scaling_x * translation_x;
    }

    pub fn set_bottom_and_top(&mut self, bottom: F, top: F) {
        assert_abs_diff_ne!(bottom, top);
        let translation_y = Self::compute_translation_y(bottom, top);
        let scaling_y = Self::compute_scaling_y(bottom, top);
        self.matrix.m22 = scaling_y;
        self.matrix.m24 = scaling_y * translation_y;
    }

    pub fn set_near_and_far(&mut self, near: F, far: F) {
        assert_abs_diff_ne!(near, far);
        let translation_z = Self::compute_translation_z(near, far);
        let scaling_z = Self::compute_scaling_z(near, far);
        self.matrix.m33 = scaling_z;
        self.matrix.m34 = scaling_z * translation_z;
    }

    fn compute_translation_x(left: F, right: F) -> F {
        -F::ONE_HALF * (left + right)
    }

    fn compute_translation_y(bottom: F, top: F) -> F {
        -F::ONE_HALF * (bottom + top)
    }

    fn compute_translation_z(near: F, _far: F) -> F {
        -near
    }

    fn compute_scaling_x(left: F, right: F) -> F {
        -F::TWO / (right - left)
    }

    fn compute_scaling_y(bottom: F, top: F) -> F {
        F::TWO / (top - bottom)
    }

    fn compute_scaling_z(near: F, far: F) -> F {
        F::ONE / (far - near)
    }
}

unsafe impl<F: Float> Zeroable for PerspectiveTransform<F> {}
unsafe impl<F: Float> Pod for PerspectiveTransform<F> {}

unsafe impl<F: Float> Zeroable for OrthographicTransform<F> {}
unsafe impl<F: Float> Pod for OrthographicTransform<F> {}

impl<F: Float> CubeMapper<F> {
    /// Quaternions representing the rotation from each of the six cube faces to
    /// the positive z face. That is, a point with a certain texture coordinate
    /// within a cube face would, after being rotated with the corresponding
    /// rotation here, have the same texture coordinate within the positive z
    /// face.
    const ROTATIONS_TO_POSITIVE_Z_FACE: [UnitQuaternion<F>; 6] = [
        // From positive x face:
        // UnitQuaternion::from_axis_angle(&Vector3::y_axis(), -F::ONE_HALF * F::PI())
        UnitQuaternion::new_unchecked(Quaternion::from_vector(vector![
            F::ZERO,
            F::NEG_FRAC_1_SQRT_2,
            F::ZERO,
            <F as Float>::FRAC_1_SQRT_2
        ])),
        // From negative x face:
        // UnitQuaternion::from_axis_angle(&Vector3::y_axis(), F::ONE_HALF * F::PI())
        UnitQuaternion::new_unchecked(Quaternion::from_vector(vector![
            F::ZERO,
            <F as Float>::FRAC_1_SQRT_2,
            F::ZERO,
            <F as Float>::FRAC_1_SQRT_2
        ])),
        // From positive y face:
        // UnitQuaternion::from_axis_angle(&Vector3::x_axis(), F::ONE_HALF * F::PI())
        UnitQuaternion::new_unchecked(Quaternion::from_vector(vector![
            <F as Float>::FRAC_1_SQRT_2,
            F::ZERO,
            F::ZERO,
            <F as Float>::FRAC_1_SQRT_2
        ])),
        // From negative y face:
        // UnitQuaternion::from_axis_angle(&Vector3::x_axis(), -F::ONE_HALF * F::PI())
        UnitQuaternion::new_unchecked(Quaternion::from_vector(vector![
            F::NEG_FRAC_1_SQRT_2,
            F::ZERO,
            F::ZERO,
            <F as Float>::FRAC_1_SQRT_2
        ])),
        // From positive z face:
        // UnitQuaternion::identity()
        UnitQuaternion::new_unchecked(Quaternion::from_vector(vector![
            F::ZERO,
            F::ZERO,
            F::ZERO,
            F::ONE
        ])),
        // From negative z face:
        // UnitQuaternion::from_axis_angle(&Vector3::y_axis(), F::PI())
        UnitQuaternion::new_unchecked(Quaternion::from_vector(vector![
            F::ZERO,
            F::ONE,
            F::ZERO,
            F::ZERO
        ])),
    ];

    /// Creates a new mapper for 3D points onto the given cubemap face.
    ///
    /// The given near and far distance refer to distances from the origin along
    /// the outward direction of the cube face. They do not affect the x and
    /// y-coordinates (or texture coordinates) of the projected point, but scale
    /// the projected z-coordinate so that points at the near plane are
    /// projected to z = 0.0 and points on the far plane are projected to z =
    /// 1.0.
    ///
    /// The given transformation to cube space will be applied to each point
    /// prior to projection onto the face.
    pub fn for_face(
        face: CubeMapFace,
        near_and_far_distance: UpperExclusiveBounds<F>,
        transform_to_cube_space: Similarity3<F>,
    ) -> Self {
        let transform_to_positive_z_face =
            Self::ROTATIONS_TO_POSITIVE_Z_FACE[face as usize] * transform_to_cube_space;

        let (z_scaling, z_translation) =
            Self::compute_z_scaling_and_translation(near_and_far_distance);

        Self {
            transform_to_positive_z_face,
            z_scaling,
            z_translation,
        }
    }

    /// Creates a new mapper for 3D points onto the given cubemap face.
    ///
    /// The given near and far distance refer to distances from the origin along
    /// the outward direction of the cube face. They do not affect the x- and
    /// y-coordinates (or texture coordinates) of the projected point, but scale
    /// the projected z-coordinate so that points at the near plane are
    /// projected to z = 0.0 and points on the far plane are projected to z =
    /// 1.0.
    ///
    /// Points to project must be specified in the coordinate system of the
    /// cubemap.
    pub fn in_cube_space_for_face(
        face: CubeMapFace,
        near_and_far_distance: UpperExclusiveBounds<F>,
    ) -> Self {
        Self::for_face(face, near_and_far_distance, Similarity3::identity())
    }

    /// Projects the given 3D point onto the cubemap face, producing a new 3D
    /// point whose x- and y-coordinates correspond to offsets from the face
    /// center in a plane parallel to the face, with the orientations of the
    /// axes following the cubemap conventions. The z-coordinate of the
    /// projected point is the offset perpendicular to the face, scaled to yield
    /// z = 0.0 at the near distance from the origin and z = 1.0 at the far
    /// distance.
    ///
    /// If the x- or y-coordinate after projection lies outside the -1.0 to 1.0
    /// range, or the z-coordinate is negative, the point belongs to another
    /// face.
    pub fn map_point(&self, point: &Point3<F>) -> Point3<F> {
        let rotated_point = self.transform_to_positive_z_face.transform_point(point);
        Self::map_point_to_positive_z_face(self.z_scaling, self.z_translation, &rotated_point)
    }

    fn compute_z_scaling_and_translation(near_and_far_distance: UpperExclusiveBounds<F>) -> (F, F) {
        let (near_distance, far_distance) = near_and_far_distance.bounds();

        let inverse_z_span = F::ONE / (near_distance - far_distance);

        let z_scaling = -F::ONE_HALF * ((near_distance + far_distance) * inverse_z_span - F::ONE);

        let z_translation = far_distance * near_distance * inverse_z_span;

        (z_scaling, z_translation)
    }

    fn map_point_to_positive_z_face(
        z_scaling: F,
        z_translation: F,
        point: &Point3<F>,
    ) -> Point3<F> {
        let inverse_point_z = F::ONE / point.z;
        Point3::new(
            point.x * inverse_point_z,
            point.y * inverse_point_z,
            (z_scaling * point.z + z_translation) * inverse_point_z,
        )
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::geometry::Degrees;
    use approx::assert_abs_diff_eq;
    use nalgebra::{point, vector};

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
        assert_abs_diff_eq!(transform.far_distance(), 100.0, epsilon = 1e-7);
        transform.set_near_and_far_distance(UpperExclusiveBounds::new(42.0, 256.0));
        assert_abs_diff_eq!(transform.near_distance(), 42.0);
        assert_abs_diff_eq!(transform.far_distance(), 256.0, epsilon = 1e-7);
    }

    #[test]
    fn transforming_point_with_perspective_transform_works() {
        let transform =
            PerspectiveTransform::new(1.0, Degrees(45.0), UpperExclusiveBounds::new(0.1, 100.0));

        let point = point![1.2, 2.4, 1.8];

        assert_abs_diff_eq!(
            transform.transform_point(&point),
            transform.as_projective().transform_point(&point),
            epsilon = 1e-9
        );
    }

    #[test]
    fn transforming_vector_with_perspective_transform_works() {
        let transform =
            PerspectiveTransform::new(1.0, Degrees(45.0), UpperExclusiveBounds::new(0.1, 100.0));

        let vector = vector![1.2, 2.4, 1.8];

        assert_abs_diff_eq!(
            transform.transform_vector(&vector),
            transform.as_projective().transform_vector(&vector),
            epsilon = 1e-9
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

        let point = point![0.0, 0.0, -near_distance];
        assert_abs_diff_eq!(transform.transform_point(&point).z, 0.0);
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

        let point = point![0.0, 0.0, -far_distance];
        assert_abs_diff_eq!(transform.transform_point(&point).z, 1.0);
    }

    #[test]
    fn mapping_to_positive_x_cubemap_face_works() {
        let near = 0.1;
        let far = 10.0;
        let mapper = CubeMapper::in_cube_space_for_face(
            CubeMapFace::PositiveX,
            UpperExclusiveBounds::new(near, far),
        );

        assert_abs_diff_eq!(
            mapper.map_point(&point![far, far, far]),
            point![-1.0, 1.0, 1.0],
            epsilon = 1e-9
        );
        assert_abs_diff_eq!(
            mapper.map_point(&point![far, -far, -far]),
            point![1.0, -1.0, 1.0],
            epsilon = 1e-9
        );
        assert_abs_diff_eq!(
            mapper.map_point(&point![near, near, near]),
            point![-1.0, 1.0, 0.0],
            epsilon = 1e-9
        );
        assert_abs_diff_eq!(
            mapper.map_point(&point![near, -near, -near]),
            point![1.0, -1.0, 0.0],
            epsilon = 1e-9
        );
    }

    #[test]
    fn mapping_to_negative_x_cubemap_face_works() {
        let near = 0.1;
        let far = 10.0;
        let mapper = CubeMapper::in_cube_space_for_face(
            CubeMapFace::NegativeX,
            UpperExclusiveBounds::new(near, far),
        );

        assert_abs_diff_eq!(
            mapper.map_point(&point![-far, far, far]),
            point![1.0, 1.0, 1.0],
            epsilon = 1e-9
        );
        assert_abs_diff_eq!(
            mapper.map_point(&point![-far, -far, -far]),
            point![-1.0, -1.0, 1.0],
            epsilon = 1e-9
        );
        assert_abs_diff_eq!(
            mapper.map_point(&point![-near, near, near]),
            point![1.0, 1.0, 0.0],
            epsilon = 1e-9
        );
        assert_abs_diff_eq!(
            mapper.map_point(&point![-near, -near, -near]),
            point![-1.0, -1.0, 0.0],
            epsilon = 1e-9
        );
    }

    #[test]
    fn mapping_to_positive_y_cubemap_face_works() {
        let near = 0.1;
        let far = 10.0;
        let mapper = CubeMapper::in_cube_space_for_face(
            CubeMapFace::PositiveY,
            UpperExclusiveBounds::new(near, far),
        );

        assert_abs_diff_eq!(
            mapper.map_point(&point![far, far, far]),
            point![1.0, -1.0, 1.0],
            epsilon = 1e-9
        );
        assert_abs_diff_eq!(
            mapper.map_point(&point![-far, far, -far]),
            point![-1.0, 1.0, 1.0],
            epsilon = 1e-9
        );
        assert_abs_diff_eq!(
            mapper.map_point(&point![near, near, near]),
            point![1.0, -1.0, 0.0],
            epsilon = 1e-9
        );
        assert_abs_diff_eq!(
            mapper.map_point(&point![-near, near, -near]),
            point![-1.0, 1.0, 0.0],
            epsilon = 1e-9
        );
    }

    #[test]
    fn mapping_to_negative_y_cubemap_face_works() {
        let near = 0.1;
        let far = 10.0;
        let mapper = CubeMapper::in_cube_space_for_face(
            CubeMapFace::NegativeY,
            UpperExclusiveBounds::new(near, far),
        );

        assert_abs_diff_eq!(
            mapper.map_point(&point![far, -far, far]),
            point![1.0, 1.0, 1.0],
            epsilon = 1e-9
        );
        assert_abs_diff_eq!(
            mapper.map_point(&point![-far, -far, -far]),
            point![-1.0, -1.0, 1.0],
            epsilon = 1e-9
        );
        assert_abs_diff_eq!(
            mapper.map_point(&point![near, -near, near]),
            point![1.0, 1.0, 0.0],
            epsilon = 1e-9
        );
        assert_abs_diff_eq!(
            mapper.map_point(&point![-near, -near, -near]),
            point![-1.0, -1.0, 0.0],
            epsilon = 1e-9
        );
    }

    #[test]
    fn mapping_to_positive_z_cubemap_face_works() {
        let near = 0.1;
        let far = 10.0;
        let mapper = CubeMapper::in_cube_space_for_face(
            CubeMapFace::PositiveZ,
            UpperExclusiveBounds::new(near, far),
        );

        assert_abs_diff_eq!(
            mapper.map_point(&point![far, far, far]),
            point![1.0, 1.0, 1.0],
            epsilon = 1e-9
        );
        assert_abs_diff_eq!(
            mapper.map_point(&point![-far, -far, far]),
            point![-1.0, -1.0, 1.0],
            epsilon = 1e-9
        );
        assert_abs_diff_eq!(
            mapper.map_point(&point![near, near, near]),
            point![1.0, 1.0, 0.0],
            epsilon = 1e-9
        );
        assert_abs_diff_eq!(
            mapper.map_point(&point![-near, -near, near]),
            point![-1.0, -1.0, 0.0],
            epsilon = 1e-9
        );
    }

    #[test]
    fn mapping_to_negative_z_cubemap_face_works() {
        let near = 0.1;
        let far = 10.0;
        let mapper = CubeMapper::in_cube_space_for_face(
            CubeMapFace::NegativeZ,
            UpperExclusiveBounds::new(near, far),
        );

        assert_abs_diff_eq!(
            mapper.map_point(&point![far, far, -far]),
            point![-1.0, 1.0, 1.0],
            epsilon = 1e-9
        );
        assert_abs_diff_eq!(
            mapper.map_point(&point![-far, -far, -far]),
            point![1.0, -1.0, 1.0],
            epsilon = 1e-9
        );
        assert_abs_diff_eq!(
            mapper.map_point(&point![near, near, -near]),
            point![-1.0, 1.0, 0.0],
            epsilon = 1e-9
        );
        assert_abs_diff_eq!(
            mapper.map_point(&point![-near, -near, -near]),
            point![1.0, -1.0, 0.0],
            epsilon = 1e-9
        );
    }
}
