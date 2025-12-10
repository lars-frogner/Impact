//! Camera setup.

use bytemuck::{Pod, Zeroable};
use impact_math::angle::{Angle, Radians};
use roc_integration::roc;

define_setup_type! {
    /// Properties of a [`PerspectiveCamera`](crate::PerspectiveCamera).
    #[roc(parents = "Setup")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct PerspectiveCamera {
        vertical_field_of_view: Radians<f32>,
        near_distance: f32,
        far_distance: f32,
    }
}

define_setup_type! {
    /// Properties of an [`OrthographicCamera`](crate::OrthographicCamera).
    #[roc(parents = "Setup")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct OrthographicCamera {
        vertical_field_of_view: Radians<f32>,
        near_distance: f32,
        far_distance: f32,
    }
}

#[roc]
impl PerspectiveCamera {
    /// Creates a new value representing a
    /// [`PerspectiveCamera`](crate::PerspectiveCamera) with the given
    /// vertical field of view (in radians) and near and far distance.
    ///
    /// # Panics
    /// If the field of view or the near distance does not exceed zero, or if
    /// the far distance does not exceed the near distance.
    #[roc(body = r#"
    # These can be uncommented once https://github.com/roc-lang/roc/issues/5680 is fixed
    # expect vertical_field_of_view > 0.0
    # expect near_distance > 0.0
    # expect far_distance > near_distance
    {
        vertical_field_of_view,
        near_distance,
        far_distance,
    }"#)]
    pub fn new(
        vertical_field_of_view: Radians<f32>,
        near_distance: f32,
        far_distance: f32,
    ) -> Self {
        assert!(vertical_field_of_view.radians() > 0.0);
        assert!(near_distance > 0.0);
        assert!(far_distance > near_distance);

        Self {
            vertical_field_of_view,
            near_distance,
            far_distance,
        }
    }

    /// Returns the vertical field of view angle in radians.
    pub fn vertical_field_of_view(&self) -> Radians<f32> {
        self.vertical_field_of_view
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

#[roc]
impl OrthographicCamera {
    /// Creates a new value representing an
    /// [`OrthographicCamera`](crate::OrthographicCamera) with the given
    /// vertical field of view (in radians) and near and far distance.
    ///
    /// # Panics
    /// If the field of view or the near distance does not exceed zero, or if
    /// the far distance does not exceed the near distance.
    #[roc(body = r#"
    # These can be uncommented once https://github.com/roc-lang/roc/issues/5680 is fixed
    # expect vertical_field_of_view > 0.0
    # expect near_distance > 0.0
    # expect far_distance > near_distance
    {
        vertical_field_of_view,
        near_distance,
        far_distance
    }"#)]
    pub fn new(
        vertical_field_of_view: Radians<f32>,
        near_distance: f32,
        far_distance: f32,
    ) -> Self {
        assert!(vertical_field_of_view.radians() > 0.0);
        assert!(near_distance > 0.0);
        assert!(far_distance > near_distance);

        Self {
            vertical_field_of_view,
            near_distance,
            far_distance,
        }
    }

    /// Returns the vertical field of view angle in radians.
    pub fn vertical_field_of_view(&self) -> Radians<f32> {
        self.vertical_field_of_view
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
