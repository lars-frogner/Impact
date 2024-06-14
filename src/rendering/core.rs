//! Core rendering system.

use crate::{rendering::fre, window::Window};
use anyhow::{anyhow, Result};
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use std::{mem, num::NonZeroU32};

/// Represents the graphics device and the basic
/// rendering entities whose configuration should not
/// change after initialization.
#[derive(Debug)]
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
    pub const INVERSE_WINDOW_DIMENSIONS_PUSH_CONSTANT_SIZE: u32 = 2 * mem::size_of::<f32>() as u32;

    /// Initializes the core system for rendering to
    /// the given window.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The width or height of the window is zero.
    /// - A compatible graphics device can not be found.
    /// - Connecting to the graphics device fails.
    pub async fn new(window: &Window) -> Result<Self> {
        let window_size = window.window().inner_size();
        Self::new_from_raw_window_handle(
            window.window(),
            (
                NonZeroU32::new(window_size.width)
                    .ok_or_else(|| anyhow!("Window width is zero"))?,
                NonZeroU32::new(window_size.height)
                    .ok_or_else(|| anyhow!("Window height is zero"))?,
            ),
        )
        .await
    }

    /// Returns the underlying [`wgpu::Device`].
    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    /// Returns the underlying [`wgpu::Queue`].
    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    /// Returns the underlying [`wgpu::Surface`].
    pub fn surface(&self) -> &wgpu::Surface {
        &self.surface
    }

    /// Returns the underlying [`wgpu::SurfaceConfiguration`].
    pub fn surface_config(&self) -> &wgpu::SurfaceConfiguration {
        &self.surface_config
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
    pub fn initialize_surface(&self) {
        self.surface.configure(&self.device, &self.surface_config);
    }

    /// Returns the data for the push constant containing the reciprocals of the
    /// window dimensions in pixels.
    pub fn get_inverse_window_dimensions_push_constant(&self) -> [fre; 2] {
        [
            1.0 / (self.surface_config.width as fre),
            1.0 / (self.surface_config.height as fre),
        ]
    }

    async fn new_from_raw_window_handle(
        window: &(impl HasRawWindowHandle + HasRawDisplayHandle),
        window_size: (NonZeroU32, NonZeroU32),
    ) -> Result<Self> {
        let wgpu_instance = Self::create_wgpu_instance();
        let surface = unsafe { wgpu_instance.create_surface(window)? };
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
        wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            flags: wgpu::InstanceFlags::default(),
            dx12_shader_compiler: wgpu::Dx12Compiler::Fxc,
            gles_minor_version: wgpu::Gles3MinorVersion::Automatic,
        })
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
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(surface), // Must work with this surface
                force_fallback_adapter: false,     // Do not fallback to software rendering system
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
                    features: wgpu::Features::PUSH_CONSTANTS
                        | wgpu::Features::POLYGON_MODE_LINE
                        | wgpu::Features::DEPTH32FLOAT_STENCIL8,
                    limits: wgpu::Limits {
                        max_bind_groups: 7,
                        max_push_constant_size: 128,
                        ..wgpu::Limits::default()
                    },
                    label: None,
                },
                None,
            )
            .await?)
    }

    /// Creates configuration defining how the surface will
    /// create its underlying [`wgpu::SurfaceTexture`].
    fn create_surface_config(
        surface: &wgpu::Surface,
        adapter: &wgpu::Adapter,
        (width, height): (NonZeroU32, NonZeroU32),
    ) -> wgpu::SurfaceConfiguration {
        let caps = surface.get_capabilities(adapter);
        wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            format: caps.formats[0],
            width: u32::from(width),
            height: u32::from(height),
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: Vec::new(),
        }
    }
}
