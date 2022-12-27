//! Representation and computation of motion.

mod components;
mod systems;

pub use components::{PositionComp, VelocityComp};
pub use systems::AdvancePositions;

use super::fph;
use nalgebra::{Point3, Vector3};

/// A 3D position.
pub type Position = Point3<fph>;

/// A 3D velocity.
pub type Velocity = Vector3<fph>;
