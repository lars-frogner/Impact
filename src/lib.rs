//! Graphics and physics engine.

#![warn(missing_debug_implementations)]
#![warn(rust_2018_idioms)]
#![warn(clippy::cast_lossless)]

pub mod control;
pub mod game_loop;
pub mod geometry;
mod num;
pub mod physics;
pub mod rendering;
pub mod run;
pub mod scheduling;
pub mod thread;
pub mod window;
pub mod world;
