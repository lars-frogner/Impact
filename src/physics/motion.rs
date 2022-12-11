//! Representation and computation of motion.

mod components;
mod systems;

pub use components::{Position, Velocity};

/// Floating point type used for representing and
/// computing motion.
#[allow(non_camel_case_types)]
pub type fmo = f64;
