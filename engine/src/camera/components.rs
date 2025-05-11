//! [`Component`](impact_ecs::component::Component)s related to cameras.

use crate::geometry::{Angle, Degrees, Radians};
use bytemuck::{Pod, Zeroable};
use impact_ecs::SetupComponent;
use roc_codegen::roc;

/// [`SetupComponent`](impact_ecs::component::SetupComponent) for initializing
/// entities that have a
/// [`PerspectiveCamera`](crate::camera::PerspectiveCamera).
///
/// The purpose of this component is to aid in constructing a
/// [`SceneGraphCameraNodeComp`](crate::scene::SceneGraphCameraNodeComp) for the
/// entity and a [`SceneCamera`](crate::camera::SceneCamera) for the
/// [`Scene`](crate::scene::Scene). It is therefore not kept after entity
/// creation.
#[roc(parents = "Comp", name = "PerspectiveCamera")]
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
#[roc(parents = "Comp", name = "OrthographicCamera")]
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod, SetupComponent)]
pub struct OrthographicCameraComp {
    vertical_field_of_view_rad: f32,
    near_distance: f32,
    far_distance: f32,
}

#[roc(dependencies=[Degrees<f32>])]
impl PerspectiveCameraComp {
    /// Creates a new component representing a
    /// [`PerspectiveCamera`](crate::camera::PerspectiveCamera) with the given
    /// vertical field of view (in degrees) and near and far distance.
    ///
    /// # Panics
    /// If the field of view or the near distance does not exceed zero, or if
    /// the far distance does not exceed the near distance.
    #[roc(body = r#"
    expect vertical_field_of_view > 0.0
    expect near_distance > 0.0
    expect far_distance > near_distance
    vertical_field_of_view_rad = Degrees.to_radians(vertical_field_of_view)
    {
        vertical_field_of_view_rad,
        near_distance,
        far_distance,
    }"#)]
    pub fn new(
        vertical_field_of_view: Degrees<f32>,
        near_distance: f32,
        far_distance: f32,
    ) -> Self {
        let vertical_field_of_view_rad = vertical_field_of_view.radians();
        assert!(vertical_field_of_view_rad > 0.0);
        assert!(near_distance > 0.0);
        assert!(far_distance > near_distance);

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

#[roc(dependencies=[Degrees<f32>])]
impl OrthographicCameraComp {
    /// Creates a new component representing an
    /// [`OrthographicCamera`](crate::camera::OrthographicCamera) with the given
    /// vertical field of view (in degrees) and near and far distance.
    ///
    /// # Panics
    /// If the field of view or the near distance does not exceed zero, or if
    /// the far distance does not exceed the near distance.
    #[roc(body = r#"
    expect vertical_field_of_view > 0.0
    expect near_distance > 0.0
    expect far_distance > near_distance
    vertical_field_of_view_rad = Degrees.to_radians(vertical_field_of_view)
    {
        vertical_field_of_view_rad,
        near_distance,
        far_distance
    }"#)]
    pub fn new(
        vertical_field_of_view: Degrees<f32>,
        near_distance: f32,
        far_distance: f32,
    ) -> Self {
        let vertical_field_of_view_rad = vertical_field_of_view.radians();
        assert!(vertical_field_of_view_rad > 0.0);
        assert!(near_distance > 0.0);
        assert!(far_distance > near_distance);

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
