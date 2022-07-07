//! Core rendering system.

use anyhow::{anyhow, Result};
use raw_window_handle::HasRawWindowHandle;
use std::num::NonZeroU32;
use winit::window::Window;

/// Represents the graphics device and the basic
/// rendering entities whose configuration should not
/// change after initialization.
pub struct CoreRenderingSystem {
    /// Connection to graphics device.
    device: wgpu::Device,
    /// Queue where we put commands to execute.
    queue: wgpu::Queue,
    /// Where graphics will be drawn.
    surface: wgpu::Surface,
    surface_config: wgpu::SurfaceConfiguration,
}

impl CoreRenderingSystem {
    /// Initializes the core system for rendering to
    /// the given window.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The width or height of the window is zero.
    /// - A compatible graphics device can not be found.
    /// - Connecting to the graphics device fails.
    pub async fn new(window: &Window) -> Result<Self> {
        let window_size = window.inner_size();
        Self::new_from_raw_window_handle(
            window,
            (
                NonZeroU32::new(window_size.width)
                    .ok_or_else(|| anyhow!("Window width is zero"))?,
                NonZeroU32::new(window_size.height)
                    .ok_or_else(|| anyhow!("Window height is zero"))?,
            ),
        )
        .await
    }

    /// Returns the underlying `wgpu` device.
    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    /// Returns the underlying `wgpu` queue.
    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    /// Returns the underlying `wgpu` surface.
    pub fn surface(&self) -> &wgpu::Surface {
        &self.surface
    }

    /// Returns the underlying `wgpu` surface configuration.
    pub fn surface_config(&self) -> &wgpu::SurfaceConfiguration {
        &self.surface_config
    }

    /// Returns the ratio of width to height of the rendering surface.
    pub fn surface_aspect_ratio(&self) -> f32 {
        let width = self.surface_config().width;
        let height = self.surface_config().height;
        width as f32 / height as f32
    }

    /// Resizes the rendering surface to the given widht and height.
    pub fn resize_surface(&mut self, (new_width, new_height): (u32, u32)) {
        if new_width > 0 && new_height > 0 {
            self.surface_config.width = new_width;
            self.surface_config.height = new_height;
            self.initialize_surface();
        }
    }

    /// Initializes the rendering surface for presentation.
    pub fn initialize_surface(&mut self) {
        self.surface.configure(&self.device, &self.surface_config);
    }

    async fn new_from_raw_window_handle<W>(
        window: &W,
        window_size: (NonZeroU32, NonZeroU32),
    ) -> Result<Self>
    where
        W: HasRawWindowHandle,
    {
        let wgpu_instance = Self::create_wgpu_instance();
        let surface = unsafe { wgpu_instance.create_surface(window) };
        let adapter = Self::create_adapter(&wgpu_instance, &surface).await?;
        let (device, queue) = Self::connect_to_device(&adapter).await?;
        let surface_config = Self::create_surface_config(&surface, &adapter, window_size);

        Ok(Self {
            device,
            queue,
            surface,
            surface_config,
        })
    }

    fn create_wgpu_instance() -> wgpu::Instance {
        // Allow all backends
        wgpu::Instance::new(wgpu::Backends::all())
    }

    /// Creates a handle to a graphics device.
    ///
    /// # Errors
    /// Returns an error if a compatible graphics device can not be found.
    async fn create_adapter(
        wgpu_instance: &wgpu::Instance,
        surface: &wgpu::Surface,
    ) -> Result<wgpu::Adapter> {
        wgpu_instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(), // High performance if available
                compatible_surface: Some(surface),                  // Must work with this surface
                force_fallback_adapter: false, // Do not fallback to software rendering system
            })
            .await
            .ok_or_else(|| anyhow!("Could not find compatible adapter"))
    }

    /// Opens a connection to a graphics device.
    ///
    /// # Errors
    /// Returns an error if the connection request fails.
    async fn connect_to_device(adapter: &wgpu::Adapter) -> Result<(wgpu::Device, wgpu::Queue)> {
        Ok(adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    features: wgpu::Features::empty(),
                    limits: if cfg!(target_arch = "wasm32") {
                        // Use looser limits for wasm
                        wgpu::Limits::downlevel_webgl2_defaults()
                    } else {
                        wgpu::Limits::default()
                    },
                    label: None,
                },
                None,
            )
            .await?)
    }

    /// Creates configuration defining how the surface will
    /// create its underlying `SurfaceTexture`.
    fn create_surface_config(
        surface: &wgpu::Surface,
        adapter: &wgpu::Adapter,
        (width, height): (NonZeroU32, NonZeroU32),
    ) -> wgpu::SurfaceConfiguration {
        wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface
                .get_preferred_format(adapter)
                .expect("Surface and adaptor not compatible"), // If this fails there is a bug
            width: u32::from(width),
            height: u32::from(height),
            present_mode: wgpu::PresentMode::Fifo,
        }
    }
}
