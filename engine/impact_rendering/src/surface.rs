//! Representation of surfaces to render to.

#[cfg(feature = "window")]
pub mod window;

use anyhow::Result;
use impact_gpu::{device::GraphicsDevice, wgpu};
use std::num::NonZeroU32;

/// A surface that can be rendered to.
#[derive(Debug)]
pub enum RenderingSurface {
    Headless,
    #[cfg(feature = "window")]
    Window(window::WindowRenderingSurface),
}

impl RenderingSurface {
    /// Creates a rendering surface for the given window.
    ///
    /// # Errors
    /// Returns an error if surface creation fails.
    #[cfg(feature = "window")]
    pub fn new_for_window(
        wgpu_instance: &wgpu::Instance,
        window: &impl window::SurfaceWindow,
    ) -> Result<Self> {
        window::WindowRenderingSurface::new(wgpu_instance, window).map(Self::Window)
    }

    /// Initializes the surface for being rendered to with the given device.
    pub fn initialize_for_device(&mut self, graphics_device: &GraphicsDevice) -> Result<()> {
        match self {
            Self::Headless => todo!(),
            #[cfg(feature = "window")]
            Self::Window(surface) => surface.initialize_for_device(graphics_device),
        }
    }

    /// Call when the surface has been lost to re-initialize it.
    ///
    /// # Panics
    /// If this is a window surface and [`Self::initialize_for_device`] has not
    /// been called.
    pub fn reinitialize_lost_surface(&self, graphics_device: &GraphicsDevice) {
        match self {
            Self::Headless => todo!(),
            #[cfg(feature = "window")]
            Self::Window(surface) => surface.configure_surface_for_device(graphics_device),
        }
    }

    /// Returns a reference to the underlying window-backed [`wgpu::Surface`],
    /// or [`None`] if the surface is not attached to a window.
    pub fn presentable_surface(&self) -> Option<&wgpu::Surface<'static>> {
        match self {
            Self::Headless => todo!(),
            #[cfg(feature = "window")]
            Self::Window(surface) => Some(surface.surface()),
        }
    }

    /// Returns a view of the surface texture, along with the
    /// [`wgpu::SurfaceTexture`] to present after rendering if the surface is
    /// attached to a window.
    pub fn get_texture_view_with_presentable_texture(
        &self,
    ) -> Result<(wgpu::TextureView, Option<wgpu::SurfaceTexture>)> {
        match self {
            Self::Headless => todo!(),
            #[cfg(feature = "window")]
            Self::Window(surface) => surface
                .get_current_surface_texture_with_view()
                .map(|(texture, view)| (view, Some(texture))),
        }
    }

    /// Returns the `(width, height)` dimensions of the rendering surface in
    /// physical pixels.
    pub fn surface_dimensions(&self) -> (NonZeroU32, NonZeroU32) {
        match self {
            Self::Headless => todo!(),
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
            Self::Headless => todo!(),
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
            Self::Headless => todo!(),
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
            Self::Headless => todo!(),
            #[cfg(feature = "window")]
            Self::Window(surface) => surface.resize(graphics_device, new_width, new_height),
        }
    }

    /// Informs the surface of a new number of pixels per point.
    pub fn update_pixels_per_point(&mut self, pixels_per_point: f64) {
        match self {
            Self::Headless => todo!(),
            #[cfg(feature = "window")]
            Self::Window(surface) => surface.update_pixels_per_point(pixels_per_point),
        }
    }

    /// Returns the data for the push constant containing the reciprocals of the
    /// window dimensions in pixels.
    pub fn inverse_window_dimensions_push_constant(&self) -> [f32; 2] {
        let (width, height) = self.surface_dimensions();
        [
            1.0 / (u32::from(width) as f32),
            1.0 / (u32::from(height) as f32),
        ]
    }

    /// Returns the data for the push constant containing the total surface
    /// pixel count.
    pub fn pixel_count_push_constant(&self) -> f32 {
        let (width, height) = self.surface_dimensions();
        (u32::from(width) as f32) * (u32::from(height) as f32)
    }
}

/// Calculates the ratio of width to height.
pub fn calculate_aspect_ratio(width: NonZeroU32, height: NonZeroU32) -> f32 {
    u32::from(width) as f32 / u32::from(height) as f32
}
