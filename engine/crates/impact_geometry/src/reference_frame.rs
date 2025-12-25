//! Reference frames.

use bytemuck::{Pod, Zeroable};
use impact_math::{
    point::{Point3, Point3A},
    quaternion::{UnitQuaternion, UnitQuaternionA},
    transform::Isometry3A,
};
use roc_integration::roc;

define_component_type! {
    /// A reference frame defined by an origin position and an orientation.
    ///
    /// This type only supports a few basic operations, as is primarily intended for
    /// compact storage inside other types and collections. For computations, prefer
    /// the SIMD-friendly 16-byte aligned [`ReferenceFrameA`].
    #[roc(parents = "Comp")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    pub struct ReferenceFrame {
        /// The coordinates of the origin of the entity's reference frame measured
        /// in the parent space.
        pub position: Point3,
        /// The 3D orientation of the entity's reference frame in the parent space.
        pub orientation: UnitQuaternion,
    }
}

/// A reference frame defined by an origin position and an orientation.
///
/// The position and orientation are stored in 128-bit SIMD registers for
/// efficient computation. That leads to an extra 4 bytes in size (due to the
/// padded point) and 16-byte alignment. For cache-friendly storage, prefer
/// [`ReferenceFrame`].
#[derive(Clone, Debug)]
pub struct ReferenceFrameA {
    /// The coordinates of the origin of the entity's reference frame measured
    /// in the parent space.
    pub position: Point3A,
    /// The 3D orientation of the entity's reference frame in the parent space.
    pub orientation: UnitQuaternionA,
}

#[roc]
impl ReferenceFrame {
    /// Creates a new reference frame with the given position and orientation.
    #[roc(body = "{ position, orientation }")]
    #[inline]
    pub const fn new(position: Point3, orientation: UnitQuaternion) -> Self {
        Self {
            position,
            orientation,
        }
    }

    /// Creates a new reference frame with the given position and the identity
    /// orientation.
    #[roc(body = "new(position, UnitQuaternion.identity)")]
    #[inline]
    pub const fn unoriented(position: Point3) -> Self {
        Self::new(position, UnitQuaternion::identity())
    }

    /// Creates a new reference frame with the given orientation, located at the
    /// origin.
    #[roc(body = "new(Point3.origin, orientation)")]
    #[inline]
    pub const fn unlocated(orientation: UnitQuaternion) -> Self {
        Self::new(Point3::origin(), orientation)
    }

    /// Converts the reference frame to the 16-byte aligned SIMD-friendly
    /// [`ReferenceFrameA`].
    #[inline]
    pub fn aligned(&self) -> ReferenceFrameA {
        ReferenceFrameA::new(self.position.aligned(), self.orientation.aligned())
    }
}

impl Default for ReferenceFrame {
    #[inline]
    fn default() -> Self {
        Self {
            position: Point3::origin(),
            orientation: UnitQuaternion::identity(),
        }
    }
}

impl ReferenceFrameA {
    /// Creates a new reference frame with the given position and orientation.
    #[inline]
    pub const fn new(position: Point3A, orientation: UnitQuaternionA) -> Self {
        Self {
            position,
            orientation,
        }
    }

    /// Creates a new reference frame with the given position and the identity
    /// orientation.
    #[inline]
    pub const fn unoriented(position: Point3A) -> Self {
        Self::new(position, UnitQuaternionA::identity())
    }

    /// Creates a new reference frame with the given orientation, located at the
    /// origin.
    #[inline]
    pub const fn unlocated(orientation: UnitQuaternionA) -> Self {
        Self::new(Point3A::origin(), orientation)
    }

    /// Creates the [`Isometry3`] transform from the entity's reference frame
    /// to the parent space.
    #[inline]
    pub fn create_transform_to_parent_space(&self) -> Isometry3A {
        Isometry3A::from_parts(*self.position.as_vector(), self.orientation)
    }

    /// Converts the reference frame to the 4-byte aligned cache-friendly
    /// [`ReferenceFrame`].
    #[inline]
    pub fn unaligned(&self) -> ReferenceFrame {
        ReferenceFrame::new(self.position.unaligned(), self.orientation.unaligned())
    }
}

impl Default for ReferenceFrameA {
    #[inline]
    fn default() -> Self {
        Self {
            position: Point3A::origin(),
            orientation: UnitQuaternionA::identity(),
        }
    }
}
