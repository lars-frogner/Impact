//! Camera types.

use crate::{
    geometry::{Angle, Bounds, EntityChangeTracker, Radians, UpperExclusiveBounds},
    num::Float,
};
use approx::assert_abs_diff_ne;
use nalgebra::{Perspective3, Projective3};
use std::fmt::Debug;

/// 3D camera using a perspective transformation.
#[derive(Debug)]
pub struct PerspectiveCamera<F: Float> {
    perspective_transform: Perspective3<F>,
    /// Tracker for whether the projection transform has changed.
    projection_transform_change_tracker: EntityChangeTracker,
}

/// Represents a 3D camera.
pub trait Camera<F: Float> {
    /// Returns the projection transform used by the camera.
    ///
    /// When the projection transform is applied to a point,
    /// the point is transformed into normalized device
    /// coordinates. In this coordinate space, the camera
    /// frustum is a cube enclosing all coordinates ranging
    /// from -1.0 to 1.0 in all three dimensions.
    fn projection_transform(&self) -> &Projective3<F>;

    /// Whether the projection transform has changed since the
    /// last reset of change tracing.
    fn projection_transform_changed(&self) -> bool;

    /// Forgets any recorded changes to the projection transform.
    fn reset_projection_change_tracking(&self);
}

impl<F: Float> PerspectiveCamera<F> {
    /// Creates a new perspective camera.
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
        let vertical_field_of_view = vertical_field_of_view.as_radians();

        assert_abs_diff_ne!(aspect_ratio, F::zero());
        assert_abs_diff_ne!(vertical_field_of_view, Radians::zero());

        let perspective_transform = Self::create_perspective_transform(
            aspect_ratio,
            vertical_field_of_view,
            &near_and_far_distance,
        );

        Self {
            perspective_transform,
            projection_transform_change_tracker: EntityChangeTracker::default(),
        }
    }

    /// Returns the ratio of width to height of the view plane.
    pub fn aspect_ratio(&self) -> F {
        self.perspective_transform.aspect()
    }

    /// Returns the vertical field of view angle in radians.
    pub fn vertical_field_of_view(&self) -> Radians<F> {
        Radians(self.perspective_transform.fovy())
    }

    pub fn near_distance(&self) -> F {
        self.perspective_transform.znear()
    }

    pub fn far_distance(&self) -> F {
        self.perspective_transform.zfar()
    }

    /// Sets the ratio of width to height of the view plane.
    ///
    /// # Panics
    /// If `aspect_ratio` is zero.
    pub fn set_aspect_ratio(&mut self, aspect_ratio: F) {
        assert_abs_diff_ne!(aspect_ratio, F::zero());
        self.perspective_transform.set_aspect(aspect_ratio);
        self.projection_transform_change_tracker.notify_change();
    }

    /// Sets the vertical field of view angle.
    ///
    /// # Panics
    /// If `fov` is zero.
    pub fn set_vertical_field_of_view<A: Angle<F>>(&mut self, fov: A) {
        let fov = fov.as_radians();
        assert_abs_diff_ne!(fov, Radians::zero());
        self.perspective_transform.set_fovy(fov.radians());
        self.projection_transform_change_tracker.notify_change();
    }

    pub fn set_near_and_far_distance(&mut self, near_and_far_distance: UpperExclusiveBounds<F>) {
        let (near_distance, far_distance) = near_and_far_distance.bounds();
        self.perspective_transform
            .set_znear_and_zfar(near_distance, far_distance);
        self.projection_transform_change_tracker.notify_change();
    }

    fn create_perspective_transform(
        aspect_ratio: F,
        vertical_field_of_view: Radians<F>,
        near_and_far_distance: &UpperExclusiveBounds<F>,
    ) -> Perspective3<F> {
        Perspective3::new(
            aspect_ratio,
            vertical_field_of_view.radians(),
            near_and_far_distance.lower(),
            near_and_far_distance.upper(),
        )
    }
}

impl<F: Float> Camera<F> for PerspectiveCamera<F> {
    fn projection_transform(&self) -> &Projective3<F> {
        self.perspective_transform.as_projective()
    }

    fn projection_transform_changed(&self) -> bool {
        self.projection_transform_change_tracker.changed()
    }

    fn reset_projection_change_tracking(&self) {
        self.projection_transform_change_tracker.reset();
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::geometry::Degrees;
    use approx::assert_abs_diff_eq;

    #[test]
    #[should_panic]
    fn constructing_perspective_camera_with_zero_aspect_ratio() {
        PerspectiveCamera::new(0.0, Degrees(45.0), UpperExclusiveBounds::new(0.1, 100.0));
    }

    #[test]
    #[should_panic]
    fn constructing_perspective_camera_with_zero_vertical_fov() {
        PerspectiveCamera::new(1.0, Degrees(0.0), UpperExclusiveBounds::new(0.1, 100.0));
    }

    #[test]
    fn setting_perspective_camera_aspect_ratio_works() {
        let mut camera =
            PerspectiveCamera::new(1.0, Degrees(45.0), UpperExclusiveBounds::new(0.1, 100.0));
        assert_abs_diff_eq!(camera.aspect_ratio(), 1.0);
        camera.set_aspect_ratio(0.5);
        assert_abs_diff_eq!(camera.aspect_ratio(), 0.5);
        assert!(camera.projection_transform_changed());
    }

    #[test]
    fn setting_perspective_camera_vertical_field_of_view_works() {
        let mut camera =
            PerspectiveCamera::new(1.0, Degrees(45.0), UpperExclusiveBounds::new(0.1, 100.0));
        assert_abs_diff_eq!(camera.vertical_field_of_view(), Degrees(45.0));
        camera.set_vertical_field_of_view(Degrees(90.0));
        assert_abs_diff_eq!(camera.vertical_field_of_view(), Degrees(90.0));
        assert!(camera.projection_transform_changed());
    }

    #[test]
    fn setting_perspective_camera_near_and_far_distance_works() {
        let mut camera =
            PerspectiveCamera::new(1.0, Degrees(45.0), UpperExclusiveBounds::new(0.1, 100.0));
        assert_abs_diff_eq!(camera.near_distance(), 0.1);
        assert_abs_diff_eq!(camera.far_distance(), 100.0, epsilon = 1e-7);
        camera.set_near_and_far_distance(UpperExclusiveBounds::new(42.0, 256.0));
        assert_abs_diff_eq!(camera.near_distance(), 42.0);
        assert_abs_diff_eq!(camera.far_distance(), 256.0, epsilon = 1e-7);
        assert!(camera.projection_transform_changed());
    }

    #[test]
    fn resetting_projection_change_tracking_works() {
        let mut camera =
            PerspectiveCamera::new(1.0, Degrees(45.0), UpperExclusiveBounds::new(0.1, 100.0));
        assert!(
            !camera.projection_transform_changed(),
            "Projection transform change reported after construction"
        );

        camera.set_aspect_ratio(0.5);
        assert!(
            camera.projection_transform_changed(),
            "No projection transform change reported after making change"
        );

        camera.reset_projection_change_tracking();
        assert!(
            !camera.projection_transform_changed(),
            "Projection transform change reported after reset"
        );
    }
}
