//! Render surfaces attached to a window.

use anyhow::{Result, anyhow};
use impact_gpu::{device::GraphicsDevice, wgpu};
use std::num::NonZeroU32;

/// Represents a window that can be used as a rendering surface.
pub trait SurfaceWindow {
    /// Returns the width of the window in physical pixels.
    fn width(&self) -> NonZeroU32;

    /// Returns the height of the window in physical pixels.
    fn height(&self) -> NonZeroU32;

    /// Returns the number of physical pixels per point/logical pixel of the
    /// screen the window is on.
    fn pixels_per_point(&self) -> f64;

    /// Returns the surface target for this window.
    fn as_target(&self) -> wgpu::SurfaceTarget<'static>;
}

/// A render surface attached to a window.
#[derive(Debug)]
pub struct WindowRenderingSurface {
    /// Where graphics will be drawn.
    surface: wgpu::Surface<'static>,
    /// Configuration defining how the surface will create its underlying
    /// [`wgpu::SurfaceTexture`].
    surface_config: SurfaceConfiguration,
    /// DPI factor of the screen the surface is rendered to. Needed when
    /// rendering GUI.
    pixels_per_point: f64,
}

#[derive(Debug)]
enum SurfaceConfiguration {
    Initialized(wgpu::SurfaceConfiguration),
    Uninitialized {
        width: NonZeroU32,
        height: NonZeroU32,
    },
}

impl WindowRenderingSurface {
    /// Creates a rendering surface for the given window.
    ///
    /// # Errors
    /// Returns an error if surface creation fails.
    pub fn new(wgpu_instance: &wgpu::Instance, window: &impl SurfaceWindow) -> Result<Self> {
        let width = window.width();
        let height = window.height();
        let pixels_per_point = window.pixels_per_point();

        let surface = wgpu_instance.create_surface(window.as_target())?;

        let surface_config = SurfaceConfiguration::Uninitialized { width, height };

        Ok(Self {
            surface,
            surface_config,
            pixels_per_point,
        })
    }

    /// Creates the configuration for the rendering surface based on the given
    /// graphics device and uses it to initialize the surface for presentation
    /// through that device.
    pub fn initialize_for_device(&mut self, graphics_device: &GraphicsDevice) -> Result<()> {
        self.initialize_surface_config_for_adapter(graphics_device.adapter())?;
        self.configure_surface_for_device(graphics_device);
        Ok(())
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

    /// Returns the next [`wgpu::SurfaceTexture`] to be presented for drawing,
    /// along with a view of the texture.
    pub fn get_current_surface_texture_with_view(
        &self,
    ) -> Result<(wgpu::SurfaceTexture, wgpu::TextureView)> {
        let surface_texture = self.surface.get_current_texture()?;

        let surface_texture_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        Ok((surface_texture, surface_texture_view))
    }

    /// Returns the `(width, height)` dimensions of the rendering surface in
    /// physical pixels.
    pub fn surface_dimensions(&self) -> (NonZeroU32, NonZeroU32) {
        self.surface_config.surface_dimensions()
    }

    /// Returns the number of physical pixels per point/logical pixel of the
    /// screen the surface is rendered to.
    pub fn pixels_per_point(&self) -> f64 {
        self.pixels_per_point
    }

    /// Returns the [`wgpu::TextureFormat`] of the rendering surface texture.
    ///
    /// # Panics
    /// If [`Self::initialize_for_device`] has not been called.
    pub fn texture_format(&self) -> wgpu::TextureFormat {
        self.surface_config
            .initialized()
            .expect("`initialize` must be called before `texture_format`")
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

    /// Informs the surface of a new number of pixels per point.
    pub fn update_pixels_per_point(&mut self, pixels_per_point: f64) {
        self.pixels_per_point = pixels_per_point;
    }

    fn initialize_surface_config_for_adapter(&mut self, adapter: &wgpu::Adapter) -> Result<()> {
        let (width, height) = self.surface_config.surface_dimensions();
        self.surface_config = SurfaceConfiguration::Initialized(Self::create_surface_config(
            &self.surface,
            adapter,
            width,
            height,
        )?);
        Ok(())
    }

    /// Creates configuration defining how the surface will create its
    /// underlying [`wgpu::SurfaceTexture`].
    fn create_surface_config(
        surface: &wgpu::Surface<'_>,
        adapter: &wgpu::Adapter,
        width: NonZeroU32,
        height: NonZeroU32,
    ) -> Result<wgpu::SurfaceConfiguration> {
        let caps = surface.get_capabilities(adapter);

        let format = Self::select_surface_texture_format(&caps)?;

        Ok(wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::COPY_DST,
            format,
            width: u32::from(width),
            height: u32::from(height),
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: Vec::new(),
            desired_maximum_frame_latency: 2,
        })
    }

    fn select_surface_texture_format(
        caps: &wgpu::SurfaceCapabilities,
    ) -> Result<wgpu::TextureFormat> {
        caps.formats
            .iter()
            .find(|format| !format.is_srgb())
            .copied()
            .ok_or_else(|| {
                anyhow!(
                    "No linear texture formats available for surface: {:?}",
                    caps.formats
                )
            })
    }
}

impl SurfaceConfiguration {
    fn initialized(&self) -> Option<&wgpu::SurfaceConfiguration> {
        match self {
            Self::Initialized(config) => Some(config),
            Self::Uninitialized { .. } => None,
        }
    }

    fn initialized_mut(&mut self) -> Option<&mut wgpu::SurfaceConfiguration> {
        match self {
            Self::Initialized(config) => Some(config),
            Self::Uninitialized { .. } => None,
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
