//! Window management.

pub mod input;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{num::NonZeroU32, sync::Arc};
use winit::{
    dpi::PhysicalSize,
    event_loop::ActiveEventLoop,
    window::{Window as WinitWindow, WindowAttributes},
};

/// Handle to a window.
#[derive(Clone, Debug)]
pub struct Window {
    window: Arc<WinitWindow>,
}

/// Configuration options for window creation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowConfig {
    /// The initial inner width and height of the window in physical pixels.
    pub initial_size: (NonZeroU32, NonZeroU32),
}

impl Window {
    pub fn create(event_loop: &ActiveEventLoop, config: &WindowConfig) -> Result<Self> {
        let (width, height) = config.initial_size;
        let size = PhysicalSize::new(width.get(), height.get());
        let attributes = WindowAttributes::default().with_inner_size(size);
        let window = event_loop.create_window(attributes)?;
        Ok(Self {
            window: Arc::new(window),
        })
    }

    /// Returns the underlying [`winit::window::Window`].
    pub fn window(&self) -> &WinitWindow {
        &self.window
    }

    /// Returns the underlying [`winit::window::Window`] wrapped in an [`Arc`].
    pub fn arc_window(&self) -> Arc<WinitWindow> {
        Arc::clone(&self.window)
    }

    /// Returns the number of physical pixels per point/logical pixel of the
    /// screen the window is on.
    pub fn pixels_per_point(&self) -> f64 {
        self.window.scale_factor()
    }

    /// Returns a tuple (width, height) with the extents of the
    /// window in number of pixels.
    pub fn dimensions(&self) -> (NonZeroU32, NonZeroU32) {
        let window_size = self.window.inner_size();
        (
            NonZeroU32::new(window_size.width).unwrap(),
            NonZeroU32::new(window_size.height).unwrap(),
        )
    }

    pub fn request_redraw(&self) {
        self.window.request_redraw();
    }
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            initial_size: (
                NonZeroU32::new(1600).unwrap(),
                NonZeroU32::new(1200).unwrap(),
            ),
        }
    }
}

/// Calculates the ratio of width to height.
pub fn calculate_aspect_ratio(width: NonZeroU32, height: NonZeroU32) -> f32 {
    u32::from(width) as f32 / u32::from(height) as f32
}
