//! Graphics and physics engine.

#[macro_use]
mod macros;

pub mod application;
pub mod assets;
pub mod camera;
pub mod command;
pub mod component;
pub mod control;
pub mod engine;
pub mod ffi;
pub mod game_loop;
pub mod gizmo;
pub mod gpu;
pub mod instrumentation;
pub mod light;
pub mod material;
pub mod mesh;
pub mod physics;
pub mod run;
pub mod runtime;
pub mod scene;
pub mod ui;
pub mod voxel;

#[cfg(feature = "window")]
pub mod window;

#[cfg(feature = "profiling")]
pub mod profiling;

pub use impact_camera;
pub use impact_containers;
pub use impact_ecs;
pub use impact_geometry;
pub use impact_gpu;
pub use impact_light;
pub use impact_material;
pub use impact_math;
pub use impact_mesh;
pub use impact_model;
pub use roc_integration;

#[cfg(feature = "egui")]
pub use egui;
