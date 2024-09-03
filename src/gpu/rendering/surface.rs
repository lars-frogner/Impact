//! Representation of surfaces to render to.

use crate::{gpu::rendering::fre, gpu::GraphicsDevice, window::Window};
use anyhow::Result;
use std::{mem, num::NonZeroU32};
use wgpu::SurfaceTarget;

/// A surface that can be rendered to.
#[derive(Debug)]
pub struct RenderingSurface {
    /// Where graphics will be drawn.
    surface: wgpu::Surface<'static>,
    /// Configuration defining how the surface will create its underlying
    /// [`wgpu::SurfaceTexture`].
    surface_config: SurfaceConfiguration,
}

#[derive(Debug)]
enum SurfaceConfiguration {
    Initialized(wgpu::SurfaceConfiguration),
    Uninitialized {
        width: NonZeroU32,
        height: NonZeroU32,
    },
}

impl RenderingSurface {
    /// Creates a rendering surface for the given window.
    ///
    /// # Errors
    /// Returns an error if surface creation fails.
    pub fn new(wgpu_instance: &wgpu::Instance, window: &Window) -> Result<Self> {
        let (width, height) = window.dimensions();
        Self::new_from_surface_target(wgpu_instance, window.arc_window(), width, height)
    }

    /// Creates the configuration for the rendering surface based on the given
    /// graphics device and uses it to initialize the surface for presentation
    /// through that device.
    pub fn initialize_for_device(&mut self, graphics_device: &GraphicsDevice) {
        self.initialize_surface_config_for_adapter(graphics_device.adapter());
        self.configure_surface_for_device(graphics_device);
    }

    /// Uses the current surface configuration to initialize the surface for
    /// presentation through the given device.
    ///
    /// # Panics
    /// If [`Self::initialize_for_device`] has not been called.
    pub fn configure_surface_for_device(&self, graphics_device: &GraphicsDevice) {
        self.surface.configure(
            graphics_device.device(),
            self.surface_config.initialized().unwrap(),
        );
    }

    /// Returns a reference to the underlying [`wgpu::Surface`].
    pub fn surface(&self) -> &wgpu::Surface<'static> {
        &self.surface
    }

    /// Returns the `(width, height)` dimensions of the rendering surface in
    /// physical pixels.
    pub fn surface_dimensions(&self) -> (NonZeroU32, NonZeroU32) {
        self.surface_config.surface_dimensions()
    }

    /// Returns the [`wgpu::TextureFormat`] of the rendering surface texture.
    ///
    /// # Panics
    /// If [`Self::initialize_for_device`] has not been called.
    pub fn texture_format(&self) -> wgpu::TextureFormat {
        self.surface_config
            .initialized()
            .expect("`initialize` must be called before `surface_config`")
            .format
    }

    /// Resizes the rendering surface to the given width and height.
    ///
    /// # Panics
    /// If [`Self::initialize_for_device`] has not been called.
    pub fn resize(
        &mut self,
        graphics_device: &GraphicsDevice,
        new_width: NonZeroU32,
        new_height: NonZeroU32,
    ) {
        let config = self
            .surface_config
            .initialized_mut()
            .expect("`initialize` must be called before `resize`");
        config.width = u32::from(new_width);
        config.height = u32::from(new_height);
        self.configure_surface_for_device(graphics_device);
    }

    /// Returns the size of the push constant obtained by calling
    /// [`Self::inverse_window_dimensions_push_constant`].
    pub const fn inverse_window_dimensions_push_constant_size() -> u32 {
        2 * mem::size_of::<f32>() as u32
    }

    /// Returns the data for the push constant containing the reciprocals of the
    /// window dimensions in pixels.
    pub fn inverse_window_dimensions_push_constant(&self) -> [fre; 2] {
        let (width, height) = self.surface_config.surface_dimensions();
        [
            1.0 / (u32::from(width) as fre),
            1.0 / (u32::from(height) as fre),
        ]
    }

    /// Returns the data for the push constant containing the total surface
    /// pixel count.
    pub fn pixel_count_push_constant(&self) -> f32 {
        let (width, height) = self.surface_config.surface_dimensions();
        (u32::from(width) as f32) * (u32::from(height) as f32)
    }

    fn initialize_surface_config_for_adapter(&mut self, adapter: &wgpu::Adapter) {
        let (width, height) = self.surface_config.surface_dimensions();
        self.surface_config = SurfaceConfiguration::Initialized(Self::create_surface_config(
            &self.surface,
            adapter,
            width,
            height,
        ));
    }

    fn new_from_surface_target(
        wgpu_instance: &wgpu::Instance,
        surface_target: impl Into<SurfaceTarget<'static>>,
        width: NonZeroU32,
        height: NonZeroU32,
    ) -> Result<Self> {
        let surface = wgpu_instance.create_surface(surface_target)?;
        let surface_config = SurfaceConfiguration::Uninitialized { width, height };
        Ok(Self {
            surface,
            surface_config,
        })
    }

    /// Creates configuration defining how the surface will create its
    /// underlying [`wgpu::SurfaceTexture`].
    fn create_surface_config(
        surface: &wgpu::Surface<'_>,
        adapter: &wgpu::Adapter,
        width: NonZeroU32,
        height: NonZeroU32,
    ) -> wgpu::SurfaceConfiguration {
        let caps = surface.get_capabilities(adapter);
        wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::COPY_DST,
            format: caps.formats[0],
            width: u32::from(width),
            height: u32::from(height),
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: Vec::new(),
            desired_maximum_frame_latency: 2,
        }
    }
}

impl SurfaceConfiguration {
    fn initialized(&self) -> Option<&wgpu::SurfaceConfiguration> {
        match self {
            Self::Initialized(config) => Some(config),
            Self::Uninitialized {
                width: _,
                height: _,
            } => None,
        }
    }

    fn initialized_mut(&mut self) -> Option<&mut wgpu::SurfaceConfiguration> {
        match self {
            Self::Initialized(config) => Some(config),
            Self::Uninitialized {
                width: _,
                height: _,
            } => None,
        }
    }

    fn surface_dimensions(&self) -> (NonZeroU32, NonZeroU32) {
        match self {
            Self::Initialized(config) => (
                NonZeroU32::new(config.width).unwrap(),
                NonZeroU32::new(config.height).unwrap(),
            ),
            Self::Uninitialized { width, height } => (*width, *height),
        }
    }
}
