//! Projection transformations.

use crate::{
    geometry::{Angle, Bounds, Radians, UpperExclusiveBounds},
    num::Float,
};
use approx::assert_abs_diff_ne;
use bytemuck::{Pod, Zeroable};
use nalgebra::{Matrix4, Point3, Projective3, Vector3};
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
        let translation_x = -F::ONE_HALF * (left + right);
        let scaling_x = -F::TWO / (right - left);
        self.matrix.m11 = scaling_x;
        self.matrix.m14 = scaling_x * translation_x;
    }

    pub fn set_bottom_and_top(&mut self, bottom: F, top: F) {
        assert_abs_diff_ne!(bottom, top);
        let translation_y = -F::ONE_HALF * (bottom + top);
        let scaling_y = F::TWO / (top - bottom);
        self.matrix.m22 = scaling_y;
        self.matrix.m24 = scaling_y * translation_y;
    }

    pub fn set_near_and_far(&mut self, near: F, far: F) {
        assert_abs_diff_ne!(near, far);
        let translation_z = near;
        let scaling_z = -F::ONE / (far - near);
        self.matrix.m33 = scaling_z;
        self.matrix.m34 = scaling_z * translation_z;
    }
}

unsafe impl<F: Float> Zeroable for PerspectiveTransform<F> {}
unsafe impl<F: Float> Pod for PerspectiveTransform<F> {}

unsafe impl<F: Float> Zeroable for OrthographicTransform<F> {}
unsafe impl<F: Float> Pod for OrthographicTransform<F> {}

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
}
