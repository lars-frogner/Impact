//! Representation and computation of motion.

mod components;
mod systems;

pub use components::{OrientationComp, PositionComp, VelocityComp};
pub use systems::AdvancePositions;

use super::fph;
use nalgebra::{Point3, UnitQuaternion, Vector3};

/// A position in 3D space.
pub type Position = Point3<fph>;

/// A velocity in 3D space.
pub type Velocity = Vector3<fph>;

/// An orientation in 3D space.
pub type Orientation = UnitQuaternion<fph>;
