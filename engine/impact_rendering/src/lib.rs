//! Graphics rendering.

pub mod attachment;
pub mod brdf;
pub mod compute;
pub mod lookup_tables;
pub mod postprocessing;
pub mod push_constant;
pub mod render_command;
pub mod resource;
pub mod shader_templates;
pub mod surface;

/// Basic rendering configuration options.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct BasicRenderingConfig {
    pub wireframe_mode_on: bool,
    pub timings_enabled: bool,
}

impl Default for BasicRenderingConfig {
    fn default() -> Self {
        Self {
            wireframe_mode_on: false,
            timings_enabled: false,
        }
    }
}
