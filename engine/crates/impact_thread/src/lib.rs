//! Utilities for multithreading.

pub mod pool;

#[cfg(feature = "rayon")]
pub mod rayon;
