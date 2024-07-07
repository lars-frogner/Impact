//! [`Component`](impact_ecs::component::Component)s related to cameras.

use crate::{
    components::ComponentRegistry,
    geometry::{Angle, Radians},
    gpu::rendering::fre,
    util::bounds::{Bounds, UpperExclusiveBounds},
};
use anyhow::Result;
use approx::assert_abs_diff_ne;
use bytemuck::{Pod, Zeroable};
use impact_ecs::Component;

/// Setup [`Component`](impact_ecs::component::Component) for initializing
/// entities that have a
/// [`PerspectiveCamera`](crate::geometry::PerspectiveCamera).
///
/// The purpose of this component is to aid in constructing a
/// [`SceneGraphCameraNodeComp`](crate::scene::SceneGraphCameraNodeComp) for the
/// entity and a [`SceneCamera`](crate::scene::SceneCamera) for the
/// [`Scene`](crate::scene::Scene). It is therefore not kept after entity
/// creation.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
pub struct PerspectiveCameraComp {
    vertical_field_of_view_rad: fre,
    near_distance: fre,
    far_distance: fre,
}

/// Setup [`Component`](impact_ecs::component::Component) for initializing
/// entities that have an
/// [`OrthographicCamera`](crate::geometry::OrthographicCamera).
///
/// The purpose of this component is to aid in constructing a
/// [`SceneGraphCameraNodeComp`](crate::scene::SceneGraphCameraNodeComp) for the
/// entity and a [`SceneCamera`](crate::scene::SceneCamera) for the
/// [`Scene`](crate::scene::Scene). It is therefore not kept after entity
/// creation.
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
    /// If `vertical_field_of_view` or the near distance is zero.
    pub fn new(
        vertical_field_of_view: impl Angle<fre>,
        near_and_far_distance: UpperExclusiveBounds<fre>,
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
