//! Graphics device.

use anyhow::{anyhow, Result};

/// Interface to a connected graphics device.
#[derive(Debug)]
pub struct GraphicsDevice {
    /// The connection to the graphics device.
    device: wgpu::Device,
    /// The queue where we put commands to execute on the device.
    queue: wgpu::Queue,
    /// The adapter used to connect to the device.
    adapter: wgpu::Adapter,
}

impl GraphicsDevice {
    /// Opens a connection to a graphics device with the given requirements for
    /// the features and limits of the device and optionally the surface the
    /// device must be compatible with.
    ///
    /// # Errors
    /// Returns an error if:
    /// - A compatible graphics device can not be found.
    /// - The connection request fails.
    pub async fn connect(
        wgpu_instance: &wgpu::Instance,
        required_features: wgpu::Features,
        required_limits: wgpu::Limits,
        memory_hints: wgpu::MemoryHints,
        compatible_surface: Option<&wgpu::Surface<'_>>,
    ) -> Result<Self> {
        let adapter = Self::create_adapter(wgpu_instance, compatible_surface).await?;

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    required_features,
                    required_limits,
                    memory_hints,
                    label: None,
                },
                None,
            )
            .await?;

        Ok(Self {
            device,
            queue,
            adapter,
        })
    }

    /// Returns a reference to the underlying [`wgpu::Device`].
    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    /// Returns  a reference tothe underlying [`wgpu::Queue`].
    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    /// Returns a reference to the [`wgpu::Adapter`] used to connect to the
    /// device.
    pub fn adapter(&self) -> &wgpu::Adapter {
        &self.adapter
    }

    /// Creates a handle to a graphics device that is compatible with the given
    /// surface, if provided.
    ///
    /// # Errors
    /// Returns an error if a compatible graphics device can not be found.
    async fn create_adapter(
        wgpu_instance: &wgpu::Instance,
        compatible_surface: Option<&wgpu::Surface<'_>>,
    ) -> Result<wgpu::Adapter> {
        wgpu_instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface,
                force_fallback_adapter: false, // Do not fallback to software rendering system
            })
            .await
            .ok_or_else(|| anyhow!("Could not find compatible adapter"))
    }
}
