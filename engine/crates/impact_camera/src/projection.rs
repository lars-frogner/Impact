//! Camera projection transforms.

use approx::assert_abs_diff_ne;
use impact_geometry::{
    Frustum,
    projection::{OrthographicTransform, PerspectiveTransform},
};
use impact_math::{
    angle::{Angle, Radians},
    bounds::{Bounds, UpperExclusiveBounds},
    transform::Projective3,
};
use std::fmt::Debug;

/// Represents a camera projection transform.
pub trait CameraProjection: Debug + Send + Sync + 'static {
    /// Returns the projective transform.
    fn transform(&self) -> &Projective3;

    /// Returns the vertical field of view angle in radians.
    fn vertical_field_of_view(&self) -> Radians;

    /// Returns the near distance of the view frustum.
    fn near_distance(&self) -> f32;

    /// Returns the far distance of the view frustum.
    fn far_distance(&self) -> f32;

    /// Returns the frustum representing the view volume.
    fn view_frustum(&self) -> &Frustum;

    /// Returns the ratio of width to height of the view plane.
    fn aspect_ratio(&self) -> f32;

    /// Returns the height of the field of view at the given view distance.
    fn view_height_at_distance(&self, distance: f32) -> f32;

    /// Sets the ratio of width to height of the view plane.
    ///
    /// # Panics
    /// If `aspect_ratio` is zero.
    fn set_aspect_ratio(&mut self, aspect_ratio: f32);

    /// Version number to allow callers to know whether the projection transform
    /// changed since they last checked it.
    fn transform_version(&self) -> u64;
}

/// A perspective projection transform for a camera.
#[derive(Debug)]
pub struct PerspectiveCameraProjection {
    perspective_transform: PerspectiveTransform,
    view_frustum: Frustum,
    transform_version: u64,
}

/// An orthographic projection transform for a camera.
#[derive(Debug)]
pub struct OrthographicCameraProjection {
    aspect_ratio: f32,
    vertical_field_of_view: Radians,
    near_and_far_distance: UpperExclusiveBounds<f32>,
    orthographic_transform: OrthographicTransform,
    view_frustum: Frustum,
    transform_version: u64,
}

impl PerspectiveCameraProjection {
    /// Creates a new perspective camera projection.
    ///
    /// # Note
    /// `aspect_ratio` is the ratio of width to height of the view plane.
    ///
    /// # Panics
    /// If `aspect_ratio`, `vertical_field_of_view` or the near distance is
    /// zero.
    pub fn new<A: Angle>(
        aspect_ratio: f32,
        vertical_field_of_view: A,
        near_and_far_distance: UpperExclusiveBounds<f32>,
    ) -> Self {
        let perspective_transform =
            PerspectiveTransform::new(aspect_ratio, vertical_field_of_view, near_and_far_distance);

        let view_frustum = Frustum::from_transform(perspective_transform.as_projective());

        Self {
            perspective_transform,
            view_frustum,
            transform_version: 0,
        }
    }

    /// Sets the vertical field of view angle.
    ///
    /// # Panics
    /// If `fov` is zero.
    pub fn set_vertical_field_of_view<A: Angle>(&mut self, fov: A) {
        self.perspective_transform.set_vertical_field_of_view(fov);
        self.update_frustum_and_increment_version();
    }

    pub fn set_near_and_far_distance(&mut self, near_and_far_distance: UpperExclusiveBounds<f32>) {
        self.perspective_transform
            .set_near_and_far_distance(near_and_far_distance);
        self.update_frustum_and_increment_version();
    }

    fn update_frustum_and_increment_version(&mut self) {
        self.view_frustum = Frustum::from_transform(self.perspective_transform.as_projective());
        self.transform_version = self.transform_version.wrapping_add(1);
    }
}

impl CameraProjection for PerspectiveCameraProjection {
    fn transform(&self) -> &Projective3 {
        self.perspective_transform.as_projective()
    }

    fn vertical_field_of_view(&self) -> Radians {
        self.perspective_transform.vertical_field_of_view()
    }

    fn near_distance(&self) -> f32 {
        self.perspective_transform.near_distance()
    }

    fn far_distance(&self) -> f32 {
        self.perspective_transform.far_distance()
    }

    fn view_frustum(&self) -> &Frustum {
        &self.view_frustum
    }

    fn aspect_ratio(&self) -> f32 {
        self.perspective_transform.aspect_ratio()
    }

    fn view_height_at_distance(&self, distance: f32) -> f32 {
        2.0 * distance * f32::tan(0.5 * self.vertical_field_of_view().radians())
    }

    fn set_aspect_ratio(&mut self, aspect_ratio: f32) {
        self.perspective_transform.set_aspect_ratio(aspect_ratio);
        self.update_frustum_and_increment_version();
    }

    fn transform_version(&self) -> u64 {
        self.transform_version
    }
}

impl OrthographicCameraProjection {
    /// Creates a new orthographic camera projection.
    ///
    /// # Note
    /// `aspect_ratio` is the ratio of width to height of the view plane.
    ///
    /// # Panics
    /// If `aspect_ratio` or `vertical_field_of_view` is zero.
    pub fn new<A: Angle>(
        aspect_ratio: f32,
        vertical_field_of_view: A,
        near_and_far_distance: UpperExclusiveBounds<f32>,
    ) -> Self {
        let orthographic_transform = OrthographicTransform::with_field_of_view(
            aspect_ratio,
            vertical_field_of_view,
            near_and_far_distance.clone(),
        );

        let view_frustum = Frustum::from_transform(orthographic_transform.as_projective());

        Self {
            aspect_ratio,
            vertical_field_of_view: vertical_field_of_view.as_radians(),
            near_and_far_distance,
            orthographic_transform,
            view_frustum,
            transform_version: 0,
        }
    }

    /// Sets the vertical field of view angle.
    ///
    /// # Panics
    /// If `fov` is zero.
    pub fn set_vertical_field_of_view<A: Angle>(&mut self, fov: A) {
        let fov = fov.as_radians();
        assert_abs_diff_ne!(fov, Radians::zero());
        self.vertical_field_of_view = fov;
        self.update_projection_transform_and_frustum();
    }

    pub fn set_near_and_far_distance(&mut self, near_and_far_distance: UpperExclusiveBounds<f32>) {
        self.near_and_far_distance = near_and_far_distance;
        self.update_projection_transform_and_frustum();
    }

    fn update_projection_transform_and_frustum(&mut self) {
        self.orthographic_transform = OrthographicTransform::with_field_of_view(
            self.aspect_ratio,
            self.vertical_field_of_view,
            self.near_and_far_distance.clone(),
        );
        self.view_frustum = Frustum::from_transform(self.orthographic_transform.as_projective());
        self.transform_version = self.transform_version.wrapping_add(1);
    }
}

impl CameraProjection for OrthographicCameraProjection {
    fn transform(&self) -> &Projective3 {
        self.orthographic_transform.as_projective()
    }

    fn vertical_field_of_view(&self) -> Radians {
        self.vertical_field_of_view
    }

    fn near_distance(&self) -> f32 {
        self.near_and_far_distance.lower()
    }

    fn far_distance(&self) -> f32 {
        self.near_and_far_distance.upper()
    }

    fn view_frustum(&self) -> &Frustum {
        &self.view_frustum
    }

    fn aspect_ratio(&self) -> f32 {
        self.aspect_ratio
    }

    fn view_height_at_distance(&self, _distance: f32) -> f32 {
        2.0 * self.near_and_far_distance.upper()
            * f32::tan(0.5 * self.vertical_field_of_view().radians())
    }

    fn set_aspect_ratio(&mut self, aspect_ratio: f32) {
        assert_abs_diff_ne!(aspect_ratio, 0.0);
        self.aspect_ratio = aspect_ratio;
        self.update_projection_transform_and_frustum();
    }

    fn transform_version(&self) -> u64 {
        self.transform_version
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_abs_diff_eq;
    use impact_math::angle::Degrees;

    #[test]
    #[should_panic]
    fn constructing_perspective_camera_with_zero_aspect_ratio() {
        PerspectiveCameraProjection::new(0.0, Degrees(45.0), UpperExclusiveBounds::new(0.1, 100.0));
    }

    #[test]
    #[should_panic]
    fn constructing_perspective_camera_with_zero_vertical_fov() {
        PerspectiveCameraProjection::new(1.0, Degrees(0.0), UpperExclusiveBounds::new(0.1, 100.0));
    }

    #[test]
    fn setting_perspective_camera_aspect_ratio_works() {
        let mut camera = PerspectiveCameraProjection::new(
            1.0,
            Degrees(45.0),
            UpperExclusiveBounds::new(0.1, 100.0),
        );
        assert_abs_diff_eq!(camera.aspect_ratio(), 1.0);
        assert_eq!(camera.transform_version(), 0);

        camera.set_aspect_ratio(0.5);
        assert_abs_diff_eq!(camera.aspect_ratio(), 0.5);
        assert_eq!(camera.transform_version(), 1);
    }

    #[test]
    fn setting_perspective_camera_vertical_field_of_view_works() {
        let mut camera = PerspectiveCameraProjection::new(
            1.0,
            Degrees(45.0),
            UpperExclusiveBounds::new(0.1, 100.0),
        );
        assert_abs_diff_eq!(camera.vertical_field_of_view(), Degrees(45.0));
        assert_eq!(camera.transform_version(), 0);

        camera.set_vertical_field_of_view(Degrees(90.0));
        assert_abs_diff_eq!(camera.vertical_field_of_view(), Degrees(90.0));
        assert_eq!(camera.transform_version(), 1);
    }

    #[test]
    fn setting_perspective_camera_near_and_far_distance_works() {
        let mut camera = PerspectiveCameraProjection::new(
            1.0,
            Degrees(45.0),
            UpperExclusiveBounds::new(0.1, 100.0),
        );
        assert_abs_diff_eq!(camera.near_distance(), 0.1);
        assert_abs_diff_eq!(camera.far_distance(), 100.0, epsilon = 1e-4);
        assert_eq!(camera.transform_version(), 0);

        camera.set_near_and_far_distance(UpperExclusiveBounds::new(42.0, 256.0));
        assert_abs_diff_eq!(camera.near_distance(), 42.0);
        assert_abs_diff_eq!(camera.far_distance(), 256.0, epsilon = 1e-4);
        assert_eq!(camera.transform_version(), 1);
    }

    #[test]
    #[should_panic]
    fn constructing_orthographic_camera_with_zero_aspect_ratio() {
        OrthographicCameraProjection::new(
            0.0,
            Degrees(45.0),
            UpperExclusiveBounds::new(0.1, 100.0),
        );
    }

    #[test]
    #[should_panic]
    fn constructing_orthographic_camera_with_zero_vertical_fov() {
        OrthographicCameraProjection::new(1.0, Degrees(0.0), UpperExclusiveBounds::new(0.1, 100.0));
    }

    #[test]
    fn setting_orthographic_camera_aspect_ratio_works() {
        let mut camera = OrthographicCameraProjection::new(
            1.0,
            Degrees(45.0),
            UpperExclusiveBounds::new(0.1, 100.0),
        );
        assert_abs_diff_eq!(camera.aspect_ratio(), 1.0);
        assert_eq!(camera.transform_version(), 0);

        camera.set_aspect_ratio(0.5);
        assert_abs_diff_eq!(camera.aspect_ratio(), 0.5);
        assert_eq!(camera.transform_version(), 1);
    }

    #[test]
    fn setting_orthographic_camera_vertical_field_of_view_works() {
        let mut camera = OrthographicCameraProjection::new(
            1.0,
            Degrees(45.0),
            UpperExclusiveBounds::new(0.1, 100.0),
        );
        assert_abs_diff_eq!(camera.vertical_field_of_view(), Degrees(45.0));
        assert_eq!(camera.transform_version(), 0);

        camera.set_vertical_field_of_view(Degrees(90.0));
        assert_abs_diff_eq!(camera.vertical_field_of_view(), Degrees(90.0));
        assert_eq!(camera.transform_version(), 1);
    }

    #[test]
    fn setting_orthographic_camera_near_and_far_distance_works() {
        let mut camera = OrthographicCameraProjection::new(
            1.0,
            Degrees(45.0),
            UpperExclusiveBounds::new(0.1, 100.0),
        );
        assert_abs_diff_eq!(camera.near_distance(), 0.1);
        assert_abs_diff_eq!(camera.far_distance(), 100.0, epsilon = 1e-7);
        assert_eq!(camera.transform_version(), 0);

        camera.set_near_and_far_distance(UpperExclusiveBounds::new(42.0, 256.0));
        assert_abs_diff_eq!(camera.near_distance(), 42.0);
        assert_abs_diff_eq!(camera.far_distance(), 256.0, epsilon = 1e-7);
        assert_eq!(camera.transform_version(), 1);
    }
}
