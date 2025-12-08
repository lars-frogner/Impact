//! Graphics and physics engine.

#[macro_use]
mod macros;

pub mod application;
pub mod command;
pub mod component;
pub mod engine;
pub mod ffi;
pub mod game_loop;
pub mod gizmo;
pub mod gpu;
pub mod input;
pub mod instrumentation;
pub mod lock_order;
pub mod physics;
pub mod rendering;
pub mod resource;
pub mod run;
pub mod runtime;
pub mod scene;
pub mod setup;
pub mod tasks;
pub mod ui;

#[cfg(feature = "window")]
pub mod window;

#[cfg(feature = "benchmark")]
pub mod benchmark;

pub use impact_alloc;
pub use impact_camera;
pub use impact_containers;
pub use impact_ecs;
pub use impact_geometry;
pub use impact_gpu;
pub use impact_io;
pub use impact_light;
pub use impact_log;
pub use impact_material;
pub use impact_math;
pub use impact_mesh;
pub use impact_model;
pub use impact_rendering;
pub use impact_scene;
pub use roc_integration;

#[cfg(feature = "egui")]
pub use egui;
