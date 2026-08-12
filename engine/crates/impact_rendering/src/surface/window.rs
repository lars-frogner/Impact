//! Render surfaces attached to a window.

use crate::surface::SurfaceAcquisition;
use anyhow::{Result, anyhow, bail};
use impact_gpu::{device::GraphicsDevice, wgpu};
use std::{fmt::Debug, num::NonZeroU32, sync::Arc};

/// Represents a window that can be used as a rendering surface.
pub trait SurfaceWindow: Debug + Send + Sync + 'static {
    /// Returns the `(width, height)` dimensions of the window in physical
    /// pixels.
    fn dimensions(&self) -> (NonZeroU32, NonZeroU32);

    /// Returns the number of physical pixels per point/logical pixel of the
    /// screen the window is on.
    fn pixels_per_point(&self) -> f64;

    /// Returns the surface target for this window.
    fn as_target(&self) -> wgpu::SurfaceTarget<'static>;
}

/// A render surface attached to a window.
#[derive(Debug)]
pub struct WindowRenderingSurface {
    /// Handle to the `wgpu` instance the surface was created with. Kept around
    /// because recreating a lost surface requires an instance.
    wgpu_instance: wgpu::Instance,
    /// The window the surface is attached to. Kept around because recovering
    /// from an outdated or lost surface requires the current window dimensions.
    window: Arc<dyn SurfaceWindow>,
    /// Where graphics will be drawn.
    surface: wgpu::Surface<'static>,
    /// Configuration defining how the surface will create its underlying
    /// [`wgpu::SurfaceTexture`].
    surface_config: SurfaceConfiguration,
    /// Whether the surface must be reconfigured before the next surface texture
    /// is acquired.
    needs_reconfiguration: bool,
    /// Whether the surface has been lost and must be recreated before the next
    /// surface texture is acquired.
    needs_recreation: bool,
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
    pub fn new(wgpu_instance: &wgpu::Instance, window: Arc<dyn SurfaceWindow>) -> Result<Self> {
        let (width, height) = window.dimensions();

        let surface = wgpu_instance.create_surface(window.as_target())?;

        let surface_config = SurfaceConfiguration::Uninitialized { width, height };

        Ok(Self {
            wgpu_instance: wgpu_instance.clone(),
            window,
            surface,
            surface_config,
            needs_reconfiguration: false,
            needs_recreation: false,
        })
    }

    /// Creates the configuration for the rendering surface based on the given
    /// graphics device and uses it to initialize the surface for presentation
    /// through that device.
    ///
    /// # Errors
    /// Returns an error if no suitable surface texture format is available.
    pub fn initialize_for_device(&mut self, graphics_device: &GraphicsDevice) -> Result<()> {
        self.initialize_surface_config_for_adapter(graphics_device.adapter(), None)?;
        self.configure_surface_for_device(graphics_device);
        Ok(())
    }

    /// Returns a reference to the underlying [`wgpu::Surface`].
    pub fn surface(&self) -> &wgpu::Surface<'static> {
        &self.surface
    }

    /// Acquires the surface texture to render into for the current frame.
    ///
    /// # Errors
    /// Returns an error if the surface is in a state we can not recover from.
    ///
    /// # Panics
    /// If [`Self::initialize_for_device`] has not been called.
    pub fn acquire_surface(
        &mut self,
        graphics_device: &GraphicsDevice,
    ) -> Result<SurfaceAcquisition> {
        // Perform any deferred reconfiguration (e.g. due to a suboptimal
        // surface texture the previous frame)
        if self.needs_reconfiguration && self.reconfigure_for_window(graphics_device) {
            return Ok(SurfaceAcquisition::Invalidated);
        }

        match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(surface_texture) => {
                Ok(Self::acquisition_for_texture(surface_texture))
            }
            wgpu::CurrentSurfaceTexture::Suboptimal(surface_texture) => {
                // If the surface is configured for the wrong dimensions, we can
                // fix it by reconfiguring, but we need to defer it until there
                // is no acquired surface texture alive
                self.needs_reconfiguration = self.is_configured_for_stale_dimensions();
                Ok(Self::acquisition_for_texture(surface_texture))
            }
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                Ok(SurfaceAcquisition::Skip)
            }
            wgpu::CurrentSurfaceTexture::Outdated => {
                // Reconfigure for the current window dimensions
                Ok(if self.reconfigure_for_window(graphics_device) {
                    SurfaceAcquisition::Invalidated
                } else {
                    SurfaceAcquisition::Skip
                })
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                // Recreating the surface requires access to the window, so it
                // must be deferred to the thread owning the window
                self.needs_reconfiguration = false;
                self.needs_recreation = true;
                Ok(SurfaceAcquisition::Skip)
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                bail!("Unexpected validation error when acquiring surface texture")
            }
        }
    }

    /// Returns the `(width, height)` dimensions of the rendering surface in
    /// physical pixels.
    ///
    /// These are the dimensions the surface is currently configured with, which
    /// may lag the window dimensions until the next
    /// [`Self::resize`] or surface reconfiguration.
    pub fn surface_dimensions(&self) -> (NonZeroU32, NonZeroU32) {
        self.surface_config.surface_dimensions()
    }

    /// Returns the number of physical pixels per point/logical pixel of the
    /// screen the surface is rendered to.
    pub fn pixels_per_point(&self) -> f64 {
        self.window.pixels_per_point()
    }

    /// Returns the [`wgpu::TextureFormat`] of the rendering surface texture.
    ///
    /// # Panics
    /// If [`Self::initialize_for_device`] has not been called.
    pub fn texture_format(&self) -> wgpu::TextureFormat {
        self.surface_config
            .initialized()
            .expect("`initialize_for_device` must be called before `texture_format`")
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
        self.reconfigure_with_dimensions(graphics_device, new_width, new_height);
    }

    /// Recreates the surface if it was lost when it was last acquired. Returns
    /// whether the surface was recreated, in which case everything derived from
    /// the surface dimensions must be recreated.
    ///
    /// Must be called on the thread owning the window, since creating a surface
    /// requires access to the window.
    ///
    /// # Errors
    /// Returns an error if surface creation fails.
    pub fn recreate_if_lost(&mut self, graphics_device: &GraphicsDevice) -> Result<bool> {
        if !self.needs_recreation {
            return Ok(false);
        }
        self.recreate(graphics_device)?;
        Ok(true)
    }

    fn acquisition_for_texture(surface_texture: wgpu::SurfaceTexture) -> SurfaceAcquisition {
        let texture_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        SurfaceAcquisition::Acquired {
            texture_view,
            presentable_texture: Some(surface_texture),
        }
    }

    fn is_configured_for_stale_dimensions(&self) -> bool {
        self.surface_dimensions() != self.window.dimensions()
    }

    /// Returns whether the surface dimensions changed as a result of the
    /// reconfiguration.
    fn reconfigure_for_window(&mut self, graphics_device: &GraphicsDevice) -> bool {
        let (width, height) = self.window.dimensions();
        self.reconfigure_with_dimensions(graphics_device, width, height)
    }

    /// Returns whether the surface dimensions changed as a result of the
    /// reconfiguration.
    fn reconfigure_with_dimensions(
        &mut self,
        graphics_device: &GraphicsDevice,
        width: NonZeroU32,
        height: NonZeroU32,
    ) -> bool {
        let config = self
            .surface_config
            .initialized_mut()
            .expect("`initialize_for_device` must be called before reconfiguring the surface");

        let dimensions_changed =
            config.width != u32::from(width) || config.height != u32::from(height);

        config.width = u32::from(width);
        config.height = u32::from(height);

        self.configure_surface_for_device(graphics_device);
        self.needs_reconfiguration = false;

        dimensions_changed
    }

    fn recreate(&mut self, graphics_device: &GraphicsDevice) -> Result<()> {
        *self = Self::new_initialized(
            &self.wgpu_instance,
            graphics_device,
            Arc::clone(&self.window),
            self.surface_config
                .initialized()
                .map(|config| config.format),
        )?;
        Ok(())
    }

    fn new_initialized(
        wgpu_instance: &wgpu::Instance,
        graphics_device: &GraphicsDevice,
        window: Arc<dyn SurfaceWindow>,
        preferred_format: Option<wgpu::TextureFormat>,
    ) -> Result<Self> {
        let mut surface = Self::new(wgpu_instance, window)?;
        surface
            .initialize_surface_config_for_adapter(graphics_device.adapter(), preferred_format)?;
        surface.configure_surface_for_device(graphics_device);
        Ok(surface)
    }

    /// Uses the current surface configuration to initialize the surface for
    /// presentation through the given device.
    ///
    /// # Panics
    /// If [`Self::initialize_for_device`] has not been called.
    fn configure_surface_for_device(&self, graphics_device: &GraphicsDevice) {
        self.surface.configure(
            graphics_device.device(),
            self.surface_config
                .initialized()
                .expect("`initialize_for_device` must be called before configuring the surface"),
        );
    }

    fn initialize_surface_config_for_adapter(
        &mut self,
        adapter: &wgpu::Adapter,
        preferred_format: Option<wgpu::TextureFormat>,
    ) -> Result<()> {
        let (width, height) = self.surface_config.surface_dimensions();
        self.surface_config = SurfaceConfiguration::Initialized(Self::create_surface_config(
            &self.surface,
            adapter,
            width,
            height,
            preferred_format,
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
        preferred_format: Option<wgpu::TextureFormat>,
    ) -> Result<wgpu::SurfaceConfiguration> {
        let caps = surface.get_capabilities(adapter);

        let format = Self::select_surface_texture_format(&caps, preferred_format)?;

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
            color_space: wgpu::SurfaceColorSpace::Auto,
            desired_maximum_frame_latency: 2,
        })
    }

    fn select_surface_texture_format(
        caps: &wgpu::SurfaceCapabilities,
        preferred_format: Option<wgpu::TextureFormat>,
    ) -> Result<wgpu::TextureFormat> {
        // Every pipeline rendering to the surface is built for a specific
        // texture format, so when recreating a surface we must keep the format
        // it had rather than re-running the selection below.
        if let Some(preferred_format) = preferred_format {
            if caps.formats.contains(&preferred_format) {
                return Ok(preferred_format);
            }
            bail!(
                "Surface texture format {:?} is no longer available after recreating the surface \
                 (available formats: {:?})",
                preferred_format,
                caps.formats
            );
        }

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
