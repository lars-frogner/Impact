//! Utilities for performance profiling.

#[macro_use]
pub mod macros;

pub mod benchmark;
pub mod instrumentation;

pub use instrumentation::timing::{TaskTimer, TimedTask};

/// Re-exported for use by the logging macros in [`macros`].
#[doc(hidden)]
pub use log as __log;
