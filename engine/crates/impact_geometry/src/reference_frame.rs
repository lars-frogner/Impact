//! Reference frames.

use bytemuck::{Pod, Zeroable};
use impact_math::Float;
use nalgebra::{Isometry3, Point3, Translation3, UnitQuaternion};
use roc_integration::roc;

define_component_type! {
    /// A reference frame defined an origin position, an orientation and a scale
    /// factor, as well as an internal offset for displacing the origin within
    /// the reference frame.
    #[roc(parents = "Comp")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct ReferenceFrame {
        /// The coordinates of the origin of the entity's reference frame measured
        /// in the parent space.
        pub position: Point3<f64>,
        /// The 3D orientation of the entity's reference frame in the parent space.
        pub orientation: UnitQuaternion<f64>,
    }
}

#[roc]
impl ReferenceFrame {
    /// Creates a new reference frame with the given position and orientation.
    #[roc(body = "{ position, orientation }")]
    pub fn new(position: Point3<f64>, orientation: UnitQuaternion<f64>) -> Self {
        Self {
            position,
            orientation,
        }
    }

    /// Creates a new reference frame with the given position and the identity
    /// orientation.
    #[roc(body = "new(position, UnitQuaternion.identity)")]
    pub fn unoriented(position: Point3<f64>) -> Self {
        Self::new(position, UnitQuaternion::identity())
    }

    /// Creates a new reference frame with the given orientation, located at the
    /// origin.
    #[roc(body = "new(Point3.origin, orientation)")]
    pub fn unlocated(orientation: UnitQuaternion<f64>) -> Self {
        Self::new(Point3::origin(), orientation)
    }

    /// Creates the [`Isometry3`] transform from the entity's reference frame
    /// to the parent space.
    pub fn create_transform_to_parent_space<F: Float>(&self) -> Isometry3<F> {
        Isometry3::from_parts(
            Translation3::from(self.position.cast::<F>()),
            self.orientation.cast::<F>(),
        )
    }
}

impl Default for ReferenceFrame {
    fn default() -> Self {
        Self {
            position: Point3::origin(),
            orientation: UnitQuaternion::identity(),
        }
    }
}
