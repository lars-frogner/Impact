//! Graphics device.

use anyhow::{Result, anyhow};

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

        let adapter_info = adapter.get_info();
        impact_log::info!(
            "Using adapter: {{ \
                name: {name}, \
                device_type: {device_type:?}, \
                driver: {driver}, \
                driver_info: {driver_info}, \
                backend: {backend} \
            }}",
            name = adapter_info.name,
            device_type = adapter_info.device_type,
            driver = adapter_info.driver,
            driver_info = adapter_info.driver_info,
            backend = adapter_info.backend,
        );

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

    /// Whether the graphics device supports the given features.
    pub fn supports_features(&self, features: wgpu::Features) -> bool {
        self.device.features().contains(features)
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
        let power_preference = wgpu::PowerPreference::HighPerformance;

        let adapter = wgpu_instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                force_fallback_adapter: false,
                power_preference,
                compatible_surface,
            })
            .await;

        // Fallback to software if hardware adapter was not found
        let adapter = if adapter.is_none() {
            wgpu_instance
                .request_adapter(&wgpu::RequestAdapterOptions {
                    force_fallback_adapter: true,
                    power_preference,
                    compatible_surface,
                })
                .await
        } else {
            adapter
        };

        adapter.ok_or_else(|| anyhow!("Could not find compatible adapter"))
    }
}
