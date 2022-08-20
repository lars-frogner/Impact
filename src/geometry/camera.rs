//! Camera types.

use crate::{
    geometry::{Angle, Bounds, EntityChangeTracker, Radians, UpperExclusiveBounds},
    num::Float,
};
use anyhow::{anyhow, Result};
use approx::assert_abs_diff_ne;
use nalgebra::{
    Isometry3, Perspective3, Point3, Projective3, Rotation3, Translation3, UnitVector3, Vector3,
};
use std::{
    collections::{hash_map::Entry, HashMap},
    fmt::Debug,
};

stringhash_newtype!(
    /// Identifier for specific cameras.
    /// Wraps a [`StringHash`](crate::hash::StringHash).
    [pub] CameraID
);

/// Repository where [`Camera`]s are stored under a
/// unique [`CameraID`].
#[derive(Debug, Default)]
pub struct CameraRepository<F: Float> {
    /// Cameras using perspective transformations.
    perspective_cameras: HashMap<CameraID, PerspectiveCamera<F>>,
}

/// Position and orientation of a 3D camera.
#[derive(Debug)]
pub struct CameraConfiguration<F: Float> {
    position: Point3<F>,
    look_direction: UnitVector3<F>,
    up_direction: UnitVector3<F>,
    view_transform: Isometry3<F>,
    /// Tracker for whether the view transform has changed.
    view_transform_change_tracker: EntityChangeTracker,
}

/// 3D camera using a perspective transformation.
#[derive(Debug)]
pub struct PerspectiveCamera<F: Float> {
    configuration: CameraConfiguration<F>,
    perspective_transform: Perspective3<F>,
    /// Tracker for whether the projection transform has changed.
    projection_transform_change_tracker: EntityChangeTracker,
}

/// Represents a 3D camera.
pub trait Camera<F: Float> {
    /// Returns the spatial configuration of the camera.
    fn config(&self) -> &CameraConfiguration<F>;

    /// Returns the spatial configuration of the camera for
    /// modification.
    fn config_mut(&mut self) -> &mut CameraConfiguration<F>;

    /// Returns the projection transform used by the camera.
    ///
    /// When the projection transform is applied to a point,
    /// the point is transformed into normalized device
    /// coordinates. In this coordinate space, the camera
    /// frustum is a cube enclosing all coordinates ranging
    /// from -1.0 to 1.0 in all three dimensions.
    fn projection_transform(&self) -> &Projective3<F>;

    /// Computes the transformation from the camera's parent
    /// space to normalized device coordinates.
    fn compute_view_projection_transform(&self) -> Projective3<F> {
        self.projection_transform() * self.config().view_transform()
    }

    /// Whether the projection transform has changed since the
    /// last reset of change tracing.
    fn projection_transform_changed(&self) -> bool;

    /// Whether the view projection transform has changed since
    /// the last reset of change tracing.
    fn view_projection_transform_changed(&self) -> bool;

    /// Forgets any recorded changes to the projection transform.
    fn reset_projection_change_tracking(&self);

    /// Forgets any recorded changes to the view projection transform.
    fn reset_view_projection_change_tracking(&self);
}

impl<F: Float> CameraRepository<F> {
    /// Creates a new empty camera repository.
    pub fn new() -> Self {
        Self {
            perspective_cameras: HashMap::new(),
        }
    }

    /// Returns a trait object representing the [`Camera`] with
    /// the given ID, or [`None`] if the camera is not present.
    pub fn get_camera(&self, camera_id: CameraID) -> Option<&dyn Camera<F>> {
        Some(self.perspective_cameras.get(&camera_id).unwrap())
    }

    /// Returns a mutable trait object representing the [`Camera`]
    /// with the given ID, or [`None`] if the camera is not present.
    pub fn get_camera_mut(&mut self, camera_id: CameraID) -> Option<&mut dyn Camera<F>> {
        Some(self.perspective_cameras.get_mut(&camera_id).unwrap())
    }

    /// Returns a reference to the [`HashMap`] storing all
    /// [`PerspectiveCamera`]s.
    pub fn perspective_cameras(&self) -> &HashMap<CameraID, PerspectiveCamera<F>> {
        &self.perspective_cameras
    }

    /// Includes the given [`PerspectiveCamera`] in the repository
    /// under the given ID.
    ///
    /// # Errors
    /// Returns an error if a camera with the given ID already
    /// exists. The repository will remain unchanged.
    pub fn add_perspective_camera(
        &mut self,
        camera_id: CameraID,
        camera: PerspectiveCamera<F>,
    ) -> Result<()> {
        match self.perspective_cameras.entry(camera_id) {
            Entry::Vacant(entry) => {
                entry.insert(camera);
                Ok(())
            }
            Entry::Occupied(_) => Err(anyhow!(
                "Camera {} already present in repository",
                camera_id
            )),
        }
    }
}

impl<F: Float> CameraConfiguration<F> {
    /// Creates a new orientation for a camera located at the
    /// given position, looking in the given direction with
    /// the given up direction.
    ///
    /// # Panics
    /// If `look_direction` and `up_direction` are equal.
    pub fn new(
        position: Point3<F>,
        look_direction: UnitVector3<F>,
        up_direction: UnitVector3<F>,
    ) -> Self {
        assert_abs_diff_ne!(look_direction, up_direction);
        let view_transform = Self::create_view_transform(&position, &look_direction, &up_direction);
        Self {
            position,
            look_direction,
            up_direction,
            view_transform,
            view_transform_change_tracker: EntityChangeTracker::default(),
        }
    }

    /// Creates a new orientation for a camera located at the
    /// given position, looking at the given target position with
    /// the given up direction.
    ///
    /// # Panics
    /// If the direction to `target_position` is the same as `up_direction`.
    pub fn new_looking_at(
        camera_position: Point3<F>,
        target_position: Point3<F>,
        up_direction: UnitVector3<F>,
    ) -> Self {
        let look_direction = UnitVector3::new_normalize(target_position - camera_position);
        Self::new(camera_position, look_direction, up_direction)
    }

    /// Returns the position of the camera.
    pub fn position(&self) -> &Point3<F> {
        &self.position
    }

    /// Returns the direction the camera field of view is centered on.
    pub fn look_direction(&self) -> &UnitVector3<F> {
        &self.look_direction
    }

    /// Returns the direction that when projected onto the field of view
    /// plane gives the vertical direction.
    pub fn up_direction(&self) -> &UnitVector3<F> {
        &self.up_direction
    }

    /// Returns the transformation to this camera's view space (where the
    /// camera is in the origin and looking along the positive z-axis).
    pub fn view_transform(&self) -> &Isometry3<F> {
        &self.view_transform
    }

    /// Whether the view transform has changed since the last reset of
    /// view transform change tracing.
    #[cfg(test)]
    fn view_transform_changed(&self) -> bool {
        self.view_transform_change_tracker.changed()
    }

    /// Moves the camera to the given position.
    pub fn move_to(&mut self, position: Point3<F>) {
        self.position = position;
        self.update_view_transform();
    }

    /// Makes the camera look in the given direction.
    ///
    /// # Panics
    /// If `look_direction` is equal to the current `up_direction`.
    pub fn point_to(&mut self, look_direction: UnitVector3<F>) {
        assert_abs_diff_ne!(look_direction, self.up_direction);
        self.look_direction = look_direction;
        self.update_view_transform();
    }

    /// Makes the camera look at the given target position.
    ///
    /// # Panics
    /// If the direction to `target_position` is the same as the current `up_direction`.
    pub fn point_at(&mut self, target_position: Point3<F>) {
        self.point_to(UnitVector3::new_normalize(target_position - self.position));
    }

    /// Moves the camera to the given position and makes it
    /// look at the given position.
    ///
    /// # Panics
    /// If the direction to `target_position` is the same as the current `up_direction`.
    pub fn move_to_and_point_at(&mut self, camera_position: Point3<F>, target_position: Point3<F>) {
        // Important to update position before determining new direction
        self.position = camera_position;
        self.point_at(target_position);
    }

    /// Translates the camera to a new position using the given
    /// translation.
    pub fn translate(&mut self, translation: &Translation3<F>) {
        self.position = translation.transform_point(self.position());
        self.update_view_transform();
    }

    /// Rotates the camera orientation using the given rotation.
    pub fn rotate_orientation(&mut self, rotation: &Rotation3<F>) {
        self.look_direction =
            UnitVector3::new_unchecked(rotation.transform_vector(self.look_direction()));
        self.up_direction =
            UnitVector3::new_unchecked(rotation.transform_vector(self.up_direction()));
        self.update_view_transform();
    }

    /// Transforms the position and orientation of the camera using
    /// the given transform.
    pub fn transform(&mut self, transform: &Isometry3<F>) {
        self.position = transform.transform_point(self.position());
        self.look_direction =
            UnitVector3::new_normalize(transform.transform_vector(self.look_direction()));
        self.up_direction =
            UnitVector3::new_normalize(transform.transform_vector(self.up_direction()));
        self.update_view_transform();
    }

    /// Forgets any recorded changes to the view transform.
    pub fn reset_view_change_tracking(&self) {
        self.view_transform_change_tracker.reset();
    }

    fn update_view_transform(&mut self) {
        self.view_transform = Self::create_view_transform(
            self.position(),
            self.look_direction(),
            self.up_direction(),
        );
        self.view_transform_change_tracker.notify_change();
    }

    fn create_view_transform(
        position: &Point3<F>,
        look_direction: &UnitVector3<F>,
        up_direction: &UnitVector3<F>,
    ) -> Isometry3<F> {
        let target = position + look_direction.into_inner();
        Isometry3::look_at_rh(position, &target, up_direction)
    }
}

impl<F: Float> Default for CameraConfiguration<F> {
    fn default() -> Self {
        Self::new(Point3::origin(), Vector3::z_axis(), Vector3::y_axis())
    }
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
        configuration: CameraConfiguration<F>,
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
            configuration,
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
    fn config(&self) -> &CameraConfiguration<F> {
        &self.configuration
    }

    fn config_mut(&mut self) -> &mut CameraConfiguration<F> {
        &mut self.configuration
    }

    fn projection_transform(&self) -> &Projective3<F> {
        self.perspective_transform.as_projective()
    }

    fn projection_transform_changed(&self) -> bool {
        self.projection_transform_change_tracker.changed()
    }

    fn view_projection_transform_changed(&self) -> bool {
        self.projection_transform_change_tracker
            .merged(&self.configuration.view_transform_change_tracker)
            .changed()
    }

    fn reset_projection_change_tracking(&self) {
        self.projection_transform_change_tracker.reset();
    }

    fn reset_view_projection_change_tracking(&self) {
        self.reset_projection_change_tracking();
        self.configuration.reset_view_change_tracking();
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::geometry::Degrees;
    use approx::assert_abs_diff_eq;
    use nalgebra::{point, vector, UnitQuaternion, Vector3};
    use std::f64::consts::PI;

    #[test]
    #[should_panic]
    fn constructing_camera_config_with_same_look_and_up_direction() {
        CameraConfiguration::<f64>::new(Point3::origin(), Vector3::z_axis(), Vector3::z_axis());
    }

    #[test]
    #[should_panic]
    fn constructing_camera_config_with_target_position_towards_up_direction() {
        CameraConfiguration::<f64>::new_looking_at(
            Point3::origin(),
            point![0.0, 0.0, 1.0],
            Vector3::z_axis(),
        );
    }

    #[test]
    fn moving_camera_to_position_works() {
        let mut config =
            CameraConfiguration::new(Point3::origin(), Vector3::z_axis(), Vector3::y_axis());
        let position = point![1.0, 2.0, 3.0];
        config.move_to(position);
        assert_abs_diff_eq!(config.position(), &position);
        assert_abs_diff_eq!(config.look_direction(), &Vector3::z_axis());
        assert_abs_diff_eq!(config.up_direction(), &Vector3::y_axis());
        assert!(config.view_transform_changed());
    }

    #[test]
    fn pointing_camera_towards_direction_works() {
        let mut config =
            CameraConfiguration::new(Point3::origin(), Vector3::z_axis(), Vector3::y_axis());
        let direction = UnitVector3::new_normalize(vector![1.0, 2.0, 3.0]);
        config.point_to(direction);
        assert_abs_diff_eq!(config.position(), &Point3::origin());
        assert_abs_diff_eq!(config.look_direction(), &direction);
        assert_abs_diff_eq!(config.up_direction(), &Vector3::y_axis());
        assert!(config.view_transform_changed());
    }

    #[test]
    fn pointing_camera_at_position_works() {
        let mut config =
            CameraConfiguration::new(Point3::origin(), Vector3::z_axis(), Vector3::y_axis());
        config.point_at(point![2.0, 0.0, 0.0]);
        assert_abs_diff_eq!(config.position(), &Point3::origin());
        assert_abs_diff_eq!(config.look_direction(), &Vector3::x_axis());
        assert_abs_diff_eq!(config.up_direction(), &Vector3::y_axis());
        assert!(config.view_transform_changed());
    }

    #[test]
    fn moving_camera_to_position_and_pointing_at_position_works() {
        let mut config =
            CameraConfiguration::new(Point3::origin(), Vector3::z_axis(), Vector3::y_axis());
        let camera_position = point![1.0, 2.0, 0.0];
        config.move_to_and_point_at(camera_position, point![5.0, 2.0, 0.0]);
        assert_abs_diff_eq!(config.position(), &camera_position);
        assert_abs_diff_eq!(config.look_direction(), &Vector3::x_axis());
        assert_abs_diff_eq!(config.up_direction(), &Vector3::y_axis());
        assert!(config.view_transform_changed());
    }

    #[test]
    fn translating_camera_works() {
        let mut config =
            CameraConfiguration::new(point![0.5, 1.5, 2.5], Vector3::z_axis(), Vector3::y_axis());
        config.translate(&Translation3::new(1.0, 2.0, 3.0));
        assert_abs_diff_eq!(config.position(), &point![1.5, 3.5, 5.5]);
        assert_abs_diff_eq!(config.look_direction(), &Vector3::z_axis());
        assert_abs_diff_eq!(config.up_direction(), &Vector3::y_axis());
        assert!(config.view_transform_changed());
    }

    #[test]
    fn rotating_camera_orientation_works() {
        let position = point![0.5, 1.5, 2.5];
        let mut config = CameraConfiguration::new(position, Vector3::z_axis(), Vector3::y_axis());
        config.rotate_orientation(&Rotation3::from_axis_angle(&Vector3::y_axis(), PI));
        assert_abs_diff_eq!(config.position(), &position);
        assert_abs_diff_eq!(config.look_direction(), &-Vector3::z_axis());
        assert_abs_diff_eq!(config.up_direction(), &Vector3::y_axis());
        config.rotate_orientation(&Rotation3::from_axis_angle(&Vector3::x_axis(), PI));
        assert_abs_diff_eq!(config.position(), &position);
        assert_abs_diff_eq!(config.look_direction(), &Vector3::z_axis());
        assert_abs_diff_eq!(config.up_direction(), &-Vector3::y_axis());
        assert!(config.view_transform_changed());
    }

    #[test]
    fn transforming_camera_works() {
        let mut config =
            CameraConfiguration::new(Point3::origin(), Vector3::z_axis(), Vector3::y_axis());
        config.transform(&Isometry3::from_parts(
            Translation3::new(0.5, 1.5, 2.5),
            UnitQuaternion::from_axis_angle(&Vector3::y_axis(), PI),
        ));
        assert_abs_diff_eq!(config.position(), &point![0.5, 1.5, 2.5]);
        assert_abs_diff_eq!(config.look_direction(), &-Vector3::z_axis());
        assert_abs_diff_eq!(config.up_direction(), &Vector3::y_axis());
        assert!(config.view_transform_changed());
    }

    #[test]
    fn special_view_transforms_are_correct() {
        let no_translation = Translation3::new(0.0, 0.0, 0.0);
        let no_rotation = UnitQuaternion::from_axis_angle(&Vector3::z_axis(), 0.0);

        assert_abs_diff_eq!(
            CameraConfiguration::new(Point3::origin(), -Vector3::z_axis(), Vector3::y_axis(),)
                .view_transform(),
            &Isometry3::from_parts(no_translation, no_rotation)
        );

        assert_abs_diff_eq!(
            CameraConfiguration::new(point![1.0, 2.0, 3.0], -Vector3::z_axis(), Vector3::y_axis(),)
                .view_transform(),
            &Isometry3::from_parts(Translation3::new(-1.0, -2.0, -3.0), no_rotation)
        );

        assert_abs_diff_eq!(
            CameraConfiguration::new(Point3::origin(), Vector3::z_axis(), Vector3::y_axis(),)
                .view_transform(),
            &Isometry3::from_parts(
                no_translation,
                UnitQuaternion::from_axis_angle(&Vector3::y_axis(), PI)
            )
        );

        assert_abs_diff_eq!(
            CameraConfiguration::new(Point3::origin(), Vector3::z_axis(), -Vector3::y_axis(),)
                .view_transform(),
            &Isometry3::from_parts(
                no_translation,
                UnitQuaternion::from_axis_angle(&Vector3::x_axis(), PI)
            )
        );

        assert_abs_diff_eq!(
            CameraConfiguration::new(point![1.0, 2.0, 3.0], Vector3::z_axis(), Vector3::y_axis(),)
                .view_transform(),
            &Isometry3::from_parts(
                Translation3::new(1.0, -2.0, 3.0),
                UnitQuaternion::from_axis_angle(&Vector3::y_axis(), PI)
            )
        );
    }

    #[test]
    fn resetting_view_change_tracking_works() {
        let mut config =
            CameraConfiguration::new(Point3::origin(), Vector3::z_axis(), Vector3::y_axis());
        assert!(
            !config.view_transform_changed(),
            "View transform change reported after construction"
        );

        config.move_to(point![1.0, 2.0, 3.0]);
        assert!(
            config.view_transform_changed(),
            "No view transform change reported after making change"
        );

        config.reset_view_change_tracking();
        assert!(
            !config.view_transform_changed(),
            "View transform change reported after reset"
        );
    }

    #[test]
    #[should_panic]
    fn constructing_perspective_camera_with_zero_aspect_ratio() {
        PerspectiveCamera::new(
            CameraConfiguration::default(),
            0.0,
            Degrees(45.0),
            UpperExclusiveBounds::new(0.1, 100.0),
        );
    }

    #[test]
    #[should_panic]
    fn constructing_perspective_camera_with_zero_vertical_fov() {
        PerspectiveCamera::new(
            CameraConfiguration::default(),
            1.0,
            Degrees(0.0),
            UpperExclusiveBounds::new(0.1, 100.0),
        );
    }

    #[test]
    fn setting_perspective_camera_aspect_ratio_works() {
        let mut camera = PerspectiveCamera::new(
            CameraConfiguration::default(),
            1.0,
            Degrees(45.0),
            UpperExclusiveBounds::new(0.1, 100.0),
        );
        assert_abs_diff_eq!(camera.aspect_ratio(), 1.0);
        camera.set_aspect_ratio(0.5);
        assert_abs_diff_eq!(camera.aspect_ratio(), 0.5);
        assert!(camera.projection_transform_changed());
        assert!(camera.view_projection_transform_changed());
    }

    #[test]
    fn setting_perspective_camera_vertical_field_of_view_works() {
        let mut camera = PerspectiveCamera::new(
            CameraConfiguration::default(),
            1.0,
            Degrees(45.0),
            UpperExclusiveBounds::new(0.1, 100.0),
        );
        assert_abs_diff_eq!(camera.vertical_field_of_view(), Degrees(45.0));
        camera.set_vertical_field_of_view(Degrees(90.0));
        assert_abs_diff_eq!(camera.vertical_field_of_view(), Degrees(90.0));
        assert!(camera.projection_transform_changed());
        assert!(camera.view_projection_transform_changed());
    }

    #[test]
    fn setting_perspective_camera_near_and_far_distance_works() {
        let mut camera = PerspectiveCamera::new(
            CameraConfiguration::default(),
            1.0,
            Degrees(45.0),
            UpperExclusiveBounds::new(0.1, 100.0),
        );
        assert_abs_diff_eq!(camera.near_distance(), 0.1);
        assert_abs_diff_eq!(camera.far_distance(), 100.0, epsilon = 1e-7);
        camera.set_near_and_far_distance(UpperExclusiveBounds::new(42.0, 256.0));
        assert_abs_diff_eq!(camera.near_distance(), 42.0);
        assert_abs_diff_eq!(camera.far_distance(), 256.0, epsilon = 1e-7);
        assert!(camera.projection_transform_changed());
        assert!(camera.view_projection_transform_changed());
    }

    #[test]
    fn view_projection_change_reported_when_changing_view() {
        let mut camera = PerspectiveCamera::new(
            CameraConfiguration::default(),
            1.0,
            Degrees(45.0),
            UpperExclusiveBounds::new(0.1, 100.0),
        );
        camera.config_mut().move_to(point![1.0, 2.0, 3.0]);
        assert!(!camera.projection_transform_changed());
        assert!(camera.view_projection_transform_changed());
    }

    #[test]
    fn resetting_projection_change_tracking_works() {
        let mut camera = PerspectiveCamera::new(
            CameraConfiguration::default(),
            1.0,
            Degrees(45.0),
            UpperExclusiveBounds::new(0.1, 100.0),
        );
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
    fn resetting_view_projection_change_tracking_works() {
        let mut camera = PerspectiveCamera::<f64>::new(
            CameraConfiguration::default(),
            1.0,
            Degrees(45.0),
            UpperExclusiveBounds::new(0.1, 100.0),
        );
        assert!(
            !camera.view_projection_transform_changed(),
            "View projection transform change reported after construction"
        );

        camera.set_aspect_ratio(0.5);
        assert!(
            camera.view_projection_transform_changed(),
            "No view projection transform change reported after making change"
        );

        camera.reset_view_projection_change_tracking();
        assert!(
            !camera.view_projection_transform_changed(),
            "View projection transform change reported after reset"
        );
    }
}
