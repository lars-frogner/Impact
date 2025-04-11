//! [`Component`](impact_ecs::component::Component)s related to cameras.

use crate::{
    geometry::{Angle, Radians},
    util::bounds::{Bounds, UpperExclusiveBounds},
};
use approx::assert_abs_diff_ne;
use bytemuck::{Pod, Zeroable};
use impact_ecs::SetupComponent;

/// [`SetupComponent`](impact_ecs::component::SetupComponent) for initializing
/// entities that have a
/// [`PerspectiveCamera`](crate::camera::PerspectiveCamera).
///
/// The purpose of this component is to aid in constructing a
/// [`SceneGraphCameraNodeComp`](crate::scene::SceneGraphCameraNodeComp) for the
/// entity and a [`SceneCamera`](crate::camera::SceneCamera) for the
/// [`Scene`](crate::scene::Scene). It is therefore not kept after entity
/// creation.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, SetupComponent)]
pub struct PerspectiveCameraComp {
    vertical_field_of_view_rad: f32,
    near_distance: f32,
    far_distance: f32,
}

/// [`SetupComponent`](impact_ecs::component::SetupComponent) for initializing
/// entities that have an
/// [`OrthographicCamera`](crate::camera::OrthographicCamera).
///
/// The purpose of this component is to aid in constructing a
/// [`SceneGraphCameraNodeComp`](crate::scene::SceneGraphCameraNodeComp) for the
/// entity and a [`SceneCamera`](crate::camera::SceneCamera) for the
/// [`Scene`](crate::scene::Scene). It is therefore not kept after entity
/// creation.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, SetupComponent)]
pub struct OrthographicCameraComp {
    vertical_field_of_view_rad: f32,
    near_distance: f32,
    far_distance: f32,
}

impl PerspectiveCameraComp {
    /// Creates a new component representing a
    /// [`PerspectiveCamera`](crate::camera::PerspectiveCamera) with the given
    /// vertical field of view and near and far distance.
    ///
    /// # Panics
    /// If `vertical_field_of_view` or the near distance is zero.
    pub fn new(
        vertical_field_of_view: impl Angle<f32>,
        near_and_far_distance: UpperExclusiveBounds<f32>,
    ) -> Self {
        let vertical_field_of_view_rad = vertical_field_of_view.radians();
        assert_abs_diff_ne!(vertical_field_of_view_rad, 0.0);

        let (near_distance, far_distance) = near_and_far_distance.bounds();
        assert_abs_diff_ne!(near_distance, 0.0);

        Self {
            vertical_field_of_view_rad,
            near_distance,
            far_distance,
        }
    }

    /// Returns the vertical field of view angle in radians.
    pub fn vertical_field_of_view(&self) -> Radians<f32> {
        Radians(self.vertical_field_of_view_rad)
    }

    /// Returns the near distance of the camera.
    pub fn near_distance(&self) -> f32 {
        self.near_distance
    }

    /// Returns the far distance of the camera.
    pub fn far_distance(&self) -> f32 {
        self.far_distance
    }
}

impl OrthographicCameraComp {
    /// Creates a new component representing an
    /// [`OrthographicCamera`](crate::camera::OrthographicCamera) with the given
    /// vertical field of view and near and far distance.
    ///
    /// # Panics
    /// If `vertical_field_of_view` is zero.
    pub fn new(
        vertical_field_of_view: impl Angle<f32>,
        near_and_far_distance: UpperExclusiveBounds<f32>,
    ) -> Self {
        let vertical_field_of_view_rad = vertical_field_of_view.radians();
        assert_abs_diff_ne!(vertical_field_of_view_rad, 0.0);

        let (near_distance, far_distance) = near_and_far_distance.bounds();

        Self {
            vertical_field_of_view_rad,
            near_distance,
            far_distance,
        }
    }

    /// Returns the vertical field of view angle in radians.
    pub fn vertical_field_of_view(&self) -> Radians<f32> {
        Radians(self.vertical_field_of_view_rad)
    }

    /// Returns the near distance of the camera.
    pub fn near_distance(&self) -> f32 {
        self.near_distance
    }

    /// Returns the far distance of the camera.
    pub fn far_distance(&self) -> f32 {
        self.far_distance
    }
}
