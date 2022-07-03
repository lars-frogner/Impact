//! Camera types.

use crate::{
    geometry::{Angle, Bounds, Radians, UpperExclusiveBounds},
    num::Float,
};
use approx::assert_abs_diff_ne;
use nalgebra::{
    Isometry3, Perspective3, Point3, Projective3, Rotation3, Translation3, UnitVector3,
};

/// Position and orientation of a 3D camera.
#[derive(Clone, Debug)]
pub struct CameraConfiguration3<F: Float> {
    position: Point3<F>,
    look_direction: UnitVector3<F>,
    up_direction: UnitVector3<F>,
    view_transform: Isometry3<F>,
}

/// 3D camera using a perspective transformation.
#[derive(Clone, Debug)]
pub struct PerspectiveCamera3<F: Float> {
    configuration: CameraConfiguration3<F>,
    perspective_transform: Perspective3<F>,
}

pub trait Camera3<F: Float> {
    /// Returns the spatial configuration of the camera.
    fn config(&self) -> &CameraConfiguration3<F>;

    fn projection_transform(&self) -> &Projective3<F>;

    fn create_view_projection_transform(&self) -> Projective3<F> {
        self.projection_transform() * self.config().view_transform()
    }
}

impl<F: Float> CameraConfiguration3<F> {
    /// Creates a new orientation for a camera located at the
    /// given position, looking at the given direction with
    /// the given up direction.
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
        }
    }

    /// Creates a new orientation for a camera located at the
    /// given position, looking at the given direction with
    /// the given up direction.
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

    /// Returns the transformation to this camera's view space (where
    /// the camera is in the origin and looking along the positive z-axis).
    pub fn view_transform(&self) -> &Isometry3<F> {
        &self.view_transform
    }

    /// Moves the camera to the given position.
    pub fn move_to(&mut self, position: Point3<F>) {
        self.position = position;
        self.update_view_transform();
    }

    /// Makes the camera look in the given direction.
    pub fn point_to(&mut self, look_direction: UnitVector3<F>) {
        assert_abs_diff_ne!(look_direction, self.up_direction);
        self.look_direction = look_direction;
        self.update_view_transform();
    }

    /// Makes the camera look at the given position.
    pub fn point_at(&mut self, target_position: Point3<F>) {
        self.point_to(UnitVector3::new_normalize(target_position - self.position));
    }

    /// Moves the camera to the given position and makes it
    /// look at the given position.
    pub fn move_to_and_point_at(&mut self, camera_position: Point3<F>, target_position: Point3<F>) {
        self.position = camera_position;
        self.point_at(target_position);
    }

    /// Translates the camera to a new position using the given
    /// translation.
    pub fn translate(&mut self, translation: &Translation3<F>) {
        self.position = translation.transform_point(self.position());
        self.update_view_transform();
    }

    /// Rotates the camera using the given rotation.
    pub fn rotate(&mut self, rotation: &Rotation3<F>) {
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

    fn update_view_transform(&mut self) {
        self.view_transform = Self::create_view_transform(
            self.position(),
            self.look_direction(),
            self.up_direction(),
        );
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

impl<F: Float> PerspectiveCamera3<F> {
    /// Creates a new perspective camera.
    ///
    /// `aspect_ratio` is the ratio of width to height of the view plane.
    pub fn new<A: Angle<F>>(
        configuration: CameraConfiguration3<F>,
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
        }
    }

    /// Returns the ratio of width to height of the view plane.
    pub fn aspect_ratio(&self) -> F {
        self.perspective_transform.aspect()
    }

    pub fn vertical_field_of_view(&self) -> Radians<F> {
        Radians(self.perspective_transform.fovy())
    }

    pub fn near_distance(&self) -> F {
        self.perspective_transform.znear()
    }

    pub fn far_distance(&self) -> F {
        self.perspective_transform.zfar()
    }

    pub fn set_aspect_ratio(&mut self, aspect_ratio: F) {
        assert_abs_diff_ne!(aspect_ratio, F::zero());
        self.perspective_transform.set_aspect(aspect_ratio);
    }

    pub fn set_vertical_field_of_view<A: Angle<F>>(&mut self, fov: A) {
        let fov = fov.as_radians();
        assert_abs_diff_ne!(fov, Radians::zero());
        self.perspective_transform.set_fovy(fov.radians());
    }

    pub fn set_near_and_far_distance(&mut self, near_and_far_distance: UpperExclusiveBounds<F>) {
        let (near_distance, far_distance) = near_and_far_distance.bounds();
        self.perspective_transform
            .set_znear_and_zfar(near_distance, far_distance);
    }

    fn create_perspective_transform(
        aspect_ratio: F,
        vertical_field_of_view: Radians<F>,
        near_and_far_distance: &UpperExclusiveBounds<F>,
    ) -> Perspective3<F> {
        // `cgmath`'s matrix uses OpenGL clip space, so we
        // must convert to wgpu clip space
        Perspective3::new(
            aspect_ratio,
            vertical_field_of_view.radians(),
            near_and_far_distance.lower(),
            near_and_far_distance.upper(),
        )
    }
}

impl<F: Float> Camera3<F> for PerspectiveCamera3<F> {
    fn config(&self) -> &CameraConfiguration3<F> {
        &self.configuration
    }

    fn projection_transform(&self) -> &Projective3<F> {
        self.perspective_transform.as_projective()
    }
}

/// Matrix for converting from OpenGL's clip space (with z between
/// -1.0 and 1.0) to  wgpu's clip space (with z between 0.0 and 1.0).
// #[rustfmt::skip]
// const OPENGL_TO_WGPU_CLIP_SPACE: Matrix4<f32> = Matrix4::new(
//     1.0, 0.0, 0.0, 0.0,
//     0.0, 1.0, 0.0, 0.0,
//     0.0, 0.0, 0.5, 0.0,
//     0.0, 0.0, 0.5, 1.0,
// );

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Degrees;
    use nalgebra::Vector3;

    #[test]
    fn camera_transforms() {
        let camera = PerspectiveCamera3::new(
            CameraConfiguration3::new_looking_at(
                Point3::new(0.0, 1.0, 2.0),
                Point3::origin(),
                Vector3::y_axis(),
            ),
            1.0,
            Degrees(45.0),
            UpperExclusiveBounds::new(0.1, 100.0),
        );
        dbg!(camera
            .create_view_projection_transform()
            .transform_vector(&Vector3::new(0.0, 0.0, 1.0)));
    }
}
