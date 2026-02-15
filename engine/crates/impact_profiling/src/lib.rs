//! Utilities for performance profiling.

#[macro_use]
pub mod macros;

pub mod benchmark;
pub mod instrumentation;

pub use instrumentation::timing::{TaskTimer, TimedTask};
