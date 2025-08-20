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

use impact_gpu::{device::GraphicsDevice, wgpu};

/// Basic rendering configuration options.
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(default)
)]
#[derive(Clone, Debug)]
pub struct BasicRenderingConfig {
    pub enabled: bool,
    pub wireframe_mode_on: bool,
    pub timings_enabled: bool,
}

impl BasicRenderingConfig {
    /// Adjusts the configuration parameters to avoid using features not
    /// supported by the given graphics device.
    pub fn make_compatible_with_device(&mut self, graphics_device: &GraphicsDevice) {
        if self.wireframe_mode_on
            && !graphics_device.supports_features(wgpu::Features::POLYGON_MODE_LINE)
        {
            impact_log::warn!("Disabling wireframe mode due to missing device features");
            self.wireframe_mode_on = false;
        }

        if self.timings_enabled
            && !graphics_device.supports_features(wgpu::Features::TIMESTAMP_QUERY)
        {
            impact_log::warn!("Disabling timestamp queries due to missing device features");
            self.timings_enabled = false;
        }
    }
}

impl Default for BasicRenderingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            wireframe_mode_on: false,
            timings_enabled: false,
        }
    }
}
