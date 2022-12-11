//! Graphics and physics engine.

#![warn(missing_debug_implementations)]
#![warn(rust_2018_idioms)]
#![warn(clippy::cast_lossless)]

#[macro_use]
mod macros;

pub mod control;
pub mod game_loop;
pub mod geometry;
pub mod hash;
mod num;
pub mod physics;
pub mod rendering;
pub mod run;
pub mod scene;
pub mod scheduling;
pub mod thread;
mod util;
pub mod window;
pub mod world;
