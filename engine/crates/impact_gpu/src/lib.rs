//! Abstractions for GPU interaction.

#[macro_use]
mod macros;

pub mod bind_group_layout;
pub mod buffer;
pub mod device;
pub mod indirect;
pub mod push_constant;
pub mod query;
pub mod resource_group;
pub mod shader;
pub mod storage;
pub mod texture;
pub mod uniform;

pub use naga;
pub use wgpu;
