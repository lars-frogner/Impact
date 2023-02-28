//! Camera types.

use crate::{
    geometry::{
        Angle, Bounds, EntityChangeTracker, Frustum, OrthographicTransform, PerspectiveTransform,
        Radians, UpperExclusiveBounds,
    },
    num::Float,
};
use approx::assert_abs_diff_ne;
use nalgebra::Projective3;
use std::fmt::Debug;

/// Represents a 3D camera.
pub trait Camera<F: Float>: Debug + Send + Sync + 'static {
    /// Returns the projection transform used by the camera.
    fn projection_transform(&self) -> &Projective3<F>;

    /// Returns the frustum representing the view volume of the
    /// camera.
    fn view_frustum(&self) -> &Frustum<F>;

    /// Returns the ratio of width to height of the camera's view plane.
    fn aspect_ratio(&self) -> F;

    /// Sets the ratio of width to height of the camera's view plane.
    ///
    /// # Panics
    /// If `aspect_ratio` is zero.
    fn set_aspect_ratio(&mut self, aspect_ratio: F);

    /// Whether the projection transform has changed since the
    /// last reset of change tracing.
    fn projection_transform_changed(&self) -> bool;

    /// Forgets any recorded changes to the projection transform.
    fn reset_projection_change_tracking(&self);
}

/// 3D camera using a perspective transformation.
#[derive(Debug)]
pub struct PerspectiveCamera<F: Float> {
    perspective_transform: PerspectiveTransform<F>,
    view_frustum: Frustum<F>,
    /// Tracker for whether the projection transform has changed.
    projection_transform_change_tracker: EntityChangeTracker,
}

/// 3D camera using an orthographic transformation.
#[derive(Debug)]
pub struct OrthographicCamera<F: Float> {
    aspect_ratio: F,
    vertical_field_of_view: Radians<F>,
    near_and_far_distance: UpperExclusiveBounds<F>,
    orthographic_transform: OrthographicTransform<F>,
    view_frustum: Frustum<F>,
    /// Tracker for whether the projection transform has changed.
    projection_transform_change_tracker: EntityChangeTracker,
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
        let perspective_transform =
            PerspectiveTransform::new(aspect_ratio, vertical_field_of_view, near_and_far_distance);

        let view_frustum = Frustum::from_transform(perspective_transform.as_projective());

        Self {
            perspective_transform,
            view_frustum,
            projection_transform_change_tracker: EntityChangeTracker::default(),
        }
    }

    /// Returns the vertical field of view angle in radians.
    pub fn vertical_field_of_view(&self) -> Radians<F> {
        self.perspective_transform.vertical_field_of_view()
    }

    /// Returns the near distance of the camera.
    pub fn near_distance(&self) -> F {
        self.perspective_transform.near_distance()
    }

    /// Returns the far distance of the camera.
    pub fn far_distance(&self) -> F {
        self.perspective_transform.far_distance()
    }

    /// Sets the vertical field of view angle.
    ///
    /// # Panics
    /// If `fov` is zero.
    pub fn set_vertical_field_of_view<A: Angle<F>>(&mut self, fov: A) {
        self.perspective_transform.set_vertical_field_of_view(fov);
        self.update_frustum_and_notify_change();
    }

    pub fn set_near_and_far_distance(&mut self, near_and_far_distance: UpperExclusiveBounds<F>) {
        self.perspective_transform
            .set_near_and_far_distance(near_and_far_distance);
        self.update_frustum_and_notify_change();
    }

    fn update_frustum_and_notify_change(&mut self) {
        self.view_frustum = Frustum::from_transform(self.perspective_transform.as_projective());
        self.projection_transform_change_tracker.notify_change();
    }
}

impl<F: Float> Camera<F> for PerspectiveCamera<F> {
    fn projection_transform(&self) -> &Projective3<F> {
        self.perspective_transform.as_projective()
    }

    fn view_frustum(&self) -> &Frustum<F> {
        &self.view_frustum
    }

    fn aspect_ratio(&self) -> F {
        self.perspective_transform.aspect_ratio()
    }

    fn set_aspect_ratio(&mut self, aspect_ratio: F) {
        self.perspective_transform.set_aspect_ratio(aspect_ratio);
        self.update_frustum_and_notify_change();
    }

    fn projection_transform_changed(&self) -> bool {
        self.projection_transform_change_tracker.changed()
    }

    fn reset_projection_change_tracking(&self) {
        self.projection_transform_change_tracker.reset();
    }
}

impl<F: Float> OrthographicCamera<F> {
    /// Creates a new orthographic camera.
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
            projection_transform_change_tracker: EntityChangeTracker::default(),
        }
    }

    /// Returns the vertical field of view angle in radians.
    pub fn vertical_field_of_view(&self) -> Radians<F> {
        self.vertical_field_of_view
    }

    /// Returns the near distance of the camera.
    pub fn near_distance(&self) -> F {
        self.near_and_far_distance.lower()
    }

    /// Returns the far distance of the camera.
    pub fn far_distance(&self) -> F {
        self.near_and_far_distance.upper()
    }

    /// Sets the vertical field of view angle.
    ///
    /// # Panics
    /// If `fov` is zero.
    pub fn set_vertical_field_of_view<A: Angle<F>>(&mut self, fov: A) {
        let fov = fov.as_radians();
        assert_abs_diff_ne!(fov, Radians::zero());
        self.vertical_field_of_view = fov;
        self.update_projection_transform_and_frustum();
    }

    pub fn set_near_and_far_distance(&mut self, near_and_far_distance: UpperExclusiveBounds<F>) {
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
        self.projection_transform_change_tracker.notify_change();
    }
}

impl<F: Float> Camera<F> for OrthographicCamera<F> {
    fn projection_transform(&self) -> &Projective3<F> {
        self.orthographic_transform.as_projective()
    }

    fn view_frustum(&self) -> &Frustum<F> {
        &self.view_frustum
    }

    fn aspect_ratio(&self) -> F {
        self.aspect_ratio
    }

    fn set_aspect_ratio(&mut self, aspect_ratio: F) {
        assert_abs_diff_ne!(aspect_ratio, F::zero());
        self.aspect_ratio = aspect_ratio;
        self.update_projection_transform_and_frustum();
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
    fn resetting_projection_change_tracking_for_perspective_camera_works() {
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

    #[test]
    #[should_panic]
    fn constructing_orthographic_camera_with_zero_aspect_ratio() {
        OrthographicCamera::new(0.0, Degrees(45.0), UpperExclusiveBounds::new(0.1, 100.0));
    }

    #[test]
    #[should_panic]
    fn constructing_orthographic_camera_with_zero_vertical_fov() {
        OrthographicCamera::new(1.0, Degrees(0.0), UpperExclusiveBounds::new(0.1, 100.0));
    }

    #[test]
    fn setting_orthographic_camera_aspect_ratio_works() {
        let mut camera =
            OrthographicCamera::new(1.0, Degrees(45.0), UpperExclusiveBounds::new(0.1, 100.0));
        assert_abs_diff_eq!(camera.aspect_ratio(), 1.0);
        camera.set_aspect_ratio(0.5);
        assert_abs_diff_eq!(camera.aspect_ratio(), 0.5);
        assert!(camera.projection_transform_changed());
    }

    #[test]
    fn setting_orthographic_camera_vertical_field_of_view_works() {
        let mut camera =
            OrthographicCamera::new(1.0, Degrees(45.0), UpperExclusiveBounds::new(0.1, 100.0));
        assert_abs_diff_eq!(camera.vertical_field_of_view(), Degrees(45.0));
        camera.set_vertical_field_of_view(Degrees(90.0));
        assert_abs_diff_eq!(camera.vertical_field_of_view(), Degrees(90.0));
        assert!(camera.projection_transform_changed());
    }

    #[test]
    fn setting_orthographic_camera_near_and_far_distance_works() {
        let mut camera =
            OrthographicCamera::new(1.0, Degrees(45.0), UpperExclusiveBounds::new(0.1, 100.0));
        assert_abs_diff_eq!(camera.near_distance(), 0.1);
        assert_abs_diff_eq!(camera.far_distance(), 100.0, epsilon = 1e-7);
        camera.set_near_and_far_distance(UpperExclusiveBounds::new(42.0, 256.0));
        assert_abs_diff_eq!(camera.near_distance(), 42.0);
        assert_abs_diff_eq!(camera.far_distance(), 256.0, epsilon = 1e-7);
        assert!(camera.projection_transform_changed());
    }

    #[test]
    fn resetting_projection_change_tracking_for_orthographic_camera_works() {
        let mut camera =
            OrthographicCamera::new(1.0, Degrees(45.0), UpperExclusiveBounds::new(0.1, 100.0));
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
