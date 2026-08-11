//! Window management.

use anyhow::Result;
use impact_gpu::wgpu;
use impact_rendering::surface::window::SurfaceWindow;
use serde::{Deserialize, Serialize};
use std::{
    num::NonZeroU32,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};
use winit::{
    dpi::PhysicalSize,
    event_loop::{ActiveEventLoop, OwnedDisplayHandle},
    window::{Window as WinitWindow, WindowAttributes},
};

/// Handle to a window.
#[derive(Clone, Debug)]
pub struct Window {
    window: Arc<WinitWindow>,
    display_handle: OwnedDisplayHandle,
    cached_properties: Arc<CachedWindowProperties>,
}

/// Configuration options for window creation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowConfig {
    /// The initial inner width and height of the window in physical pixels.
    pub initial_size: (NonZeroU32, NonZeroU32),
}

/// Window properties cached for access from any thread.
///
/// Querying a `winit` window may require a synchronous round-trip to the main
/// thread. This would lead to a deadlock if queried from a task worker thread
/// while the main thread is blocked waiting for the tasks to complete. We
/// therefore keep a snapshot that is only ever written from the main thread, in
/// response to window events.
#[derive(Debug)]
struct CachedWindowProperties {
    /// Width in the upper and height in the lower 32 bits.
    dimensions: AtomicU64,
    /// The bits of the `f64` number of physical pixels per point.
    pixels_per_point: AtomicU64,
}

impl Window {
    pub fn create(event_loop: &ActiveEventLoop, config: &WindowConfig) -> Result<Self> {
        let (width, height) = config.initial_size;
        let size = PhysicalSize::new(width.get(), height.get());
        let attributes = WindowAttributes::default().with_inner_size(size);
        let window = event_loop.create_window(attributes)?;
        let display_handle = event_loop.owned_display_handle();

        let (width, height) =
            non_zero_dimensions(window.inner_size()).unwrap_or(config.initial_size);

        let cached_properties = Arc::new(CachedWindowProperties::new(
            width,
            height,
            window.scale_factor(),
        ));

        Ok(Self {
            window: Arc::new(window),
            display_handle,
            cached_properties,
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

    /// Returns a reference to the window's [`OwnedDisplayHandle`].
    pub fn display_handle(&self) -> &OwnedDisplayHandle {
        &self.display_handle
    }

    /// Returns the number of physical pixels per point/logical pixel of the
    /// screen the window is on.
    pub fn pixels_per_point(&self) -> f64 {
        self.cached_properties.pixels_per_point()
    }

    /// Returns a tuple (width, height) with the extents of the
    /// window in number of pixels.
    pub fn dimensions(&self) -> (NonZeroU32, NonZeroU32) {
        self.cached_properties.dimensions()
    }

    /// Updates the cached window dimensions.
    pub fn handle_resize(&self, new_width: NonZeroU32, new_height: NonZeroU32) {
        self.cached_properties.set_dimensions(new_width, new_height);
    }

    /// Updates the cached number of physical pixels per point.
    pub fn handle_scale_factor_change(&self, pixels_per_point: f64) {
        self.cached_properties
            .set_pixels_per_point(pixels_per_point);
    }

    pub fn request_redraw(&self) {
        self.window.request_redraw();
    }
}

impl SurfaceWindow for Window {
    fn dimensions(&self) -> (NonZeroU32, NonZeroU32) {
        self.cached_properties.dimensions()
    }

    fn pixels_per_point(&self) -> f64 {
        self.cached_properties.pixels_per_point()
    }

    fn as_target(&self) -> wgpu::SurfaceTarget<'static> {
        self.arc_window().into()
    }
}

impl CachedWindowProperties {
    fn new(width: NonZeroU32, height: NonZeroU32, pixels_per_point: f64) -> Self {
        Self {
            dimensions: AtomicU64::new(pack_dimensions(width, height)),
            pixels_per_point: AtomicU64::new(pixels_per_point.to_bits()),
        }
    }

    fn dimensions(&self) -> (NonZeroU32, NonZeroU32) {
        unpack_dimensions(self.dimensions.load(Ordering::Relaxed))
    }

    fn set_dimensions(&self, width: NonZeroU32, height: NonZeroU32) {
        self.dimensions
            .store(pack_dimensions(width, height), Ordering::Relaxed);
    }

    fn pixels_per_point(&self) -> f64 {
        f64::from_bits(self.pixels_per_point.load(Ordering::Relaxed))
    }

    fn set_pixels_per_point(&self, pixels_per_point: f64) {
        self.pixels_per_point
            .store(pixels_per_point.to_bits(), Ordering::Relaxed);
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

fn non_zero_dimensions(size: PhysicalSize<u32>) -> Option<(NonZeroU32, NonZeroU32)> {
    Some((NonZeroU32::new(size.width)?, NonZeroU32::new(size.height)?))
}

fn pack_dimensions(width: NonZeroU32, height: NonZeroU32) -> u64 {
    (u64::from(width.get()) << 32) | u64::from(height.get())
}

fn unpack_dimensions(packed: u64) -> (NonZeroU32, NonZeroU32) {
    let width = (packed >> 32) as u32;
    let height = packed as u32;
    (
        NonZeroU32::new(width).unwrap(),
        NonZeroU32::new(height).unwrap(),
    )
}
