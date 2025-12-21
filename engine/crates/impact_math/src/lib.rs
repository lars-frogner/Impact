//! Math utilities.

#[macro_use]
mod macros;

pub mod angle;
pub mod bounds;
pub mod consts;
pub mod halton;
pub mod hash;
pub mod matrix;
pub mod num;
pub mod point;
pub mod power_law;
pub mod quaternion;
pub mod splitmix;
pub mod transform;
pub mod vector;

pub use num::Float;
