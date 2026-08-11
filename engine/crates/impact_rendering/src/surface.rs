//! Representation of surfaces to render to.

pub mod headless;

#[cfg(feature = "window")]
pub mod window;

use anyhow::Result;
use headless::HeadlessRenderingSurface;
use impact_gpu::{device::GraphicsDevice, wgpu};
use std::num::NonZeroU32;

/// A surface that can be rendered to.
#[derive(Debug)]
pub enum RenderingSurface {
    Headless(HeadlessRenderingSurface),
    #[cfg(feature = "window")]
    Window(window::WindowRenderingSurface),
}

/// The outcome of attempting to acquire the surface to render into for a frame.
#[derive(Debug)]
pub enum SurfaceAcquisition {
    Acquired {
        /// The view to render into.
        texture_view: wgpu::TextureView,
        /// The texture to present after rendering, if the surface is attached
        /// to a window.
        presentable_texture: Option<wgpu::SurfaceTexture>,
    },
    /// The surface is temporarily unavailable, so the frame should be skipped.
    Skip,
    /// The surface was reconfigured or recreated with new dimensions. The frame
    /// should be skipped, and everything derived from the surface dimensions
    /// must be recreated before the next frame.
    Invalidated,
}

impl RenderingSurface {
    /// Creates a headless rendering surface with the given dimensions in
    /// physical pixels.
    pub fn new_headless(width: NonZeroU32, height: NonZeroU32) -> Self {
        Self::Headless(HeadlessRenderingSurface::new(width, height))
    }

    /// Creates a rendering surface for the given window.
    ///
    /// # Errors
    /// Returns an error if surface creation fails.
    #[cfg(feature = "window")]
    pub fn new_for_window(
        wgpu_instance: &wgpu::Instance,
        window: std::sync::Arc<dyn window::SurfaceWindow>,
    ) -> Result<Self> {
        window::WindowRenderingSurface::new(wgpu_instance, window).map(Self::Window)
    }

    /// Initializes the surface for being rendered to with the given device.
    pub fn initialize_for_device(&mut self, graphics_device: &GraphicsDevice) -> Result<()> {
        match self {
            Self::Headless(surface) => surface.initialize_for_device(graphics_device),
            #[cfg(feature = "window")]
            Self::Window(surface) => surface.initialize_for_device(graphics_device),
        }
    }

    /// Acquires the surface to render into for the current frame.
    ///
    /// # Errors
    /// Returns an error if the surface is in a state we can not recover from.
    ///
    /// # Panics
    /// If this is a window surface and [`Self::initialize_for_device`] has not
    /// been called.
    #[allow(unused_variables)]
    pub fn acquire_surface(
        &mut self,
        graphics_device: &GraphicsDevice,
    ) -> Result<SurfaceAcquisition> {
        match self {
            Self::Headless(surface) => Ok(SurfaceAcquisition::Acquired {
                texture_view: surface.create_surface_texture_view(),
                presentable_texture: None,
            }),
            #[cfg(feature = "window")]
            Self::Window(surface) => surface.acquire_surface(graphics_device),
        }
    }

    /// Recreates the surface if it was lost when it was last acquired. Returns
    /// whether the surface was recreated, in which case everything derived from
    /// the surface dimensions must be recreated.
    ///
    /// Must be called on the thread owning the window if this is a window
    /// surface, since creating a surface requires access to the window.
    ///
    /// # Errors
    /// Returns an error if surface creation fails.
    #[allow(unused_variables)]
    pub fn recreate_if_lost(&mut self, graphics_device: &GraphicsDevice) -> Result<bool> {
        match self {
            Self::Headless(_) => Ok(false),
            #[cfg(feature = "window")]
            Self::Window(surface) => surface.recreate_if_lost(graphics_device),
        }
    }

    /// Returns a reference to the underlying window-backed [`wgpu::Surface`],
    /// or [`None`] if the surface is not attached to a window.
    pub fn presentable_surface(&self) -> Option<&wgpu::Surface<'static>> {
        match self {
            Self::Headless(_) => None,
            #[cfg(feature = "window")]
            Self::Window(surface) => Some(surface.surface()),
        }
    }

    /// Returns the `(width, height)` dimensions of the rendering surface in
    /// physical pixels.
    pub fn surface_dimensions(&self) -> (NonZeroU32, NonZeroU32) {
        match self {
            Self::Headless(surface) => surface.surface_dimensions(),
            #[cfg(feature = "window")]
            Self::Window(surface) => surface.surface_dimensions(),
        }
    }

    /// Returns the ratio of width to height of the rendering surface.
    pub fn surface_aspect_ratio(&self) -> f32 {
        let (width, height) = self.surface_dimensions();
        calculate_aspect_ratio(width, height)
    }

    /// Returns the number of physical pixels per point/logical pixel of the
    /// screen the surface is rendered to.
    pub fn pixels_per_point(&self) -> f64 {
        match self {
            Self::Headless(_) => 1.0,
            #[cfg(feature = "window")]
            Self::Window(surface) => surface.pixels_per_point(),
        }
    }

    /// Returns the [`wgpu::TextureFormat`] of the rendering surface texture.
    ///
    /// # Panics
    /// If this is a window surface and [`Self::initialize_for_device`] has not
    /// been called.
    pub fn texture_format(&self) -> wgpu::TextureFormat {
        match self {
            Self::Headless(_) => HeadlessRenderingSurface::texture_format(),
            #[cfg(feature = "window")]
            Self::Window(surface) => surface.texture_format(),
        }
    }

    /// Resizes the rendering surface to the given width and height.
    ///
    /// # Panics
    /// If this is a window surface and [`Self::initialize_for_device`] has not
    /// been called.
    pub fn resize(
        &mut self,
        graphics_device: &GraphicsDevice,
        new_width: NonZeroU32,
        new_height: NonZeroU32,
    ) {
        match self {
            Self::Headless(surface) => surface.resize(graphics_device, new_width, new_height),
            #[cfg(feature = "window")]
            Self::Window(surface) => surface.resize(graphics_device, new_width, new_height),
        }
    }

    /// Returns the data for the immediate containing the reciprocals of the
    /// window dimensions in pixels.
    pub fn inverse_window_dimensions_immediate(&self) -> [f32; 2] {
        let (width, height) = self.surface_dimensions();
        [
            1.0 / (u32::from(width) as f32),
            1.0 / (u32::from(height) as f32),
        ]
    }

    /// Returns the data for the immediate containing the total surface pixel
    /// count.
    pub fn pixel_count_immediate(&self) -> f32 {
        let (width, height) = self.surface_dimensions();
        (u32::from(width) as f32) * (u32::from(height) as f32)
    }
}

/// Calculates the ratio of width to height.
pub fn calculate_aspect_ratio(width: NonZeroU32, height: NonZeroU32) -> f32 {
    u32::from(width) as f32 / u32::from(height) as f32
}
