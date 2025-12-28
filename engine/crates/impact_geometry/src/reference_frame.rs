//! Reference frames.

use bytemuck::{Pod, Zeroable};
use impact_math::{point::Point3P, quaternion::UnitQuaternionP, transform::Isometry3};
use roc_integration::roc;

define_component_type! {
    /// A reference frame defined by an origin position and an orientation.
    #[roc(parents = "Comp")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct ReferenceFrame {
        /// The coordinates of the origin of the entity's reference frame measured
        /// in the parent space.
        pub position: Point3P,
        /// The 3D orientation of the entity's reference frame in the parent space.
        pub orientation: UnitQuaternionP,
    }
}

#[roc]
impl ReferenceFrame {
    /// Creates a new reference frame with the given position and orientation.
    #[roc(body = "{ position, orientation }")]
    #[inline]
    pub const fn new(position: Point3P, orientation: UnitQuaternionP) -> Self {
        Self {
            position,
            orientation,
        }
    }

    /// Creates a new reference frame with the given position and the identity
    /// orientation.
    #[roc(body = "new(position, UnitQuaternion.identity)")]
    #[inline]
    pub const fn unoriented(position: Point3P) -> Self {
        Self::new(position, UnitQuaternionP::identity())
    }

    /// Creates a new reference frame with the given orientation, located at the
    /// origin.
    #[roc(body = "new(Point3.origin, orientation)")]
    #[inline]
    pub const fn unlocated(orientation: UnitQuaternionP) -> Self {
        Self::new(Point3P::origin(), orientation)
    }

    /// Creates the [`Isometry3`] transform from the entity's reference frame
    /// to the parent space.
    #[inline]
    pub fn create_transform_to_parent_space(&self) -> Isometry3 {
        let translation = self.position.as_vector().unpack();
        let rotation = self.orientation.unpack();
        Isometry3::from_parts(translation, rotation)
    }
}

impl Default for ReferenceFrame {
    #[inline]
    fn default() -> Self {
        Self {
            position: Point3P::origin(),
            orientation: UnitQuaternionP::identity(),
        }
    }
}
