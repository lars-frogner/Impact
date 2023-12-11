//! [`Component`](impact_ecs::component::Component)s related to cameras.

use crate::{
    components::ComponentRegistry,
    geometry::{Angle, Bounds, Radians, UpperExclusiveBounds},
    rendering::fre,
};
use anyhow::Result;
use approx::assert_abs_diff_ne;
use bytemuck::{Pod, Zeroable};
use impact_ecs::Component;

/// [`Component`](impact_ecs::component::Component) for entities that
/// have a [`PerspectiveCamera`](crate::geometry::PerspectiveCamera).
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct PerspectiveCameraComp {
    vertical_field_of_view_rad: fre,
    near_distance: fre,
    far_distance: fre,
}

/// [`Component`](impact_ecs::component::Component) for entities that
/// have an [`OrthographicCamera`](crate::geometry::OrthographicCamera).
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct OrthographicCameraComp {
    vertical_field_of_view_rad: fre,
    near_distance: fre,
    far_distance: fre,
}

impl PerspectiveCameraComp {
    /// Creates a new component representing a
    /// [`PerspectiveCamera`](crate::geometry::PerspectiveCamera) with the given
    /// vertical field of view and near and far distance.
    ///
    /// # Panics
    /// If `vertical_field_of_view` is zero.
    pub fn new(
        vertical_field_of_view: impl Angle<fre>,
        near_and_far_distance: UpperExclusiveBounds<fre>,
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
    pub fn vertical_field_of_view(&self) -> Radians<fre> {
        Radians(self.vertical_field_of_view_rad)
    }

    /// Returns the near distance of the camera.
    pub fn near_distance(&self) -> fre {
        self.near_distance
    }

    /// Returns the far distance of the camera.
    pub fn far_distance(&self) -> fre {
        self.far_distance
    }
}

impl OrthographicCameraComp {
    /// Creates a new component representing an
    /// [`OrthographicCamera`](crate::geometry::OrthographicCamera) with the given
    /// vertical field of view and near and far distance.
    ///
    /// # Panics
    /// If `vertical_field_of_view` is zero.
    pub fn new(
        vertical_field_of_view: impl Angle<fre>,
        near_and_far_distance: UpperExclusiveBounds<fre>,
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
    pub fn vertical_field_of_view(&self) -> Radians<fre> {
        Radians(self.vertical_field_of_view_rad)
    }

    /// Returns the near distance of the camera.
    pub fn near_distance(&self) -> fre {
        self.near_distance
    }

    /// Returns the far distance of the camera.
    pub fn far_distance(&self) -> fre {
        self.far_distance
    }
}

/// Registers all camera [`Component`](impact_ecs::component::Component)s.
pub fn register_camera_components(registry: &mut ComponentRegistry) -> Result<()> {
    register_setup_component!(registry, PerspectiveCameraComp)?;
    register_setup_component!(registry, OrthographicCameraComp)
}
