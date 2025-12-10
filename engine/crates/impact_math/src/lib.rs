//! Math utilities.

#[macro_use]
mod macros;

pub mod angle;
pub mod bounds;
pub mod halton;
pub mod hash;
pub mod num;
pub mod power_law;
pub mod splitmix;

pub use num::Float;
