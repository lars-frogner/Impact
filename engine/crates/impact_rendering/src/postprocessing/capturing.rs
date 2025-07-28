//! Capturing the incident scene luminance in a camera.

pub mod average_luminance;
pub mod bloom;
pub mod dynamic_range_compression;

use crate::{
    attachment::RenderAttachmentTextureManager,
    postprocessing::Postprocessor,
    resource::{BasicGPUResources, BasicResourceRegistries},
    surface::RenderingSurface,
};
use anyhow::Result;
use average_luminance::{AverageLuminanceComputationConfig, AverageLuminanceComputeCommands};
use bloom::{BloomConfig, BloomRenderCommands};
use dynamic_range_compression::{
    DynamicRangeCompressionConfig, DynamicRangeCompressionRenderCommands,
};
use impact_gpu::{
    bind_group_layout::BindGroupLayoutRegistry, device::GraphicsDevice,
    query::TimestampQueryRegistry, resource_group::GPUResourceGroupManager, shader::ShaderManager,
    storage::StorageGPUBufferManager, wgpu,
};
use impact_math::Bounds;
use roc_integration::roc;

/// Configuration options for a capturing camera.
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(default)
)]
#[derive(Clone, Debug, Default)]
pub struct CapturingCameraConfig {
    /// The settings for the camera.
    pub settings: CameraSettings,
    /// Configuration options for bloom.
    pub bloom: BloomConfig,
    /// Configuration options for the computation of the average luminance for
    /// automatic sensitivity.
    pub average_luminance_computation: AverageLuminanceComputationConfig,
    /// Configuration options for dynamic range compression.
    pub dynamic_range_compression: DynamicRangeCompressionConfig,
}

/// Capturing settings for a camera.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct CameraSettings {
    /// The relative aperture of the camera, which is the ratio of the focal
    /// length to the aperture diameter.
    ///
    /// # Unit
    /// F-stops.
    pub relative_aperture: f32,
    /// The duration the sensor is exposed.
    ///
    /// # Unit
    /// Seconds.
    pub shutter_duration: f32,
    /// The sensitivity of the camera sensor.
    pub sensitivity: SensorSensitivity,
    /// The maximum exposure of the camera sensor. This corresponds to the
    /// reciprocal of the minimum incident luminance in cd/m² that can saturate
    /// the sensor.
    pub max_exposure: f32,
}

/// The sensitivity of a camera sensor, which may be set manually as an ISO
/// value or determined automatically based on the incident luminance, with
/// optional exposure value compensation in f-stops.
#[roc(parents = "Rendering")]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug)]
pub enum SensorSensitivity {
    Manual { iso: f32 },
    Auto { ev_compensation: f32 },
}

/// A camera capturing the incident scene luminance.
#[derive(Debug)]
pub struct CapturingCamera {
    settings: CameraSettings,
    exposure: f32,
    average_luminance_commands: AverageLuminanceComputeCommands,
    bloom_commands: BloomRenderCommands,
    dynamic_range_compression_commands: DynamicRangeCompressionRenderCommands,
}

impl Default for CameraSettings {
    fn default() -> Self {
        Self {
            relative_aperture: 4.0,
            shutter_duration: 1.0 / 200.0,
            sensitivity: SensorSensitivity::Auto {
                ev_compensation: 0.0,
            },
            max_exposure: 1e-2,
        }
    }
}

impl CameraSettings {
    /// The fraction of attenuation by the lens and vignetting.
    const ATTENUATION_FACTOR: f32 = 0.65;

    /// The reflected-light meter calibration constant.
    const CALIBRATION_CONSTANT: f32 = 12.5;

    /// Groups the given camera settings.
    pub fn new(
        relative_aperture: f32,
        shutter_duration: f32,
        sensitivity: SensorSensitivity,
        max_exposure: f32,
    ) -> Self {
        Self {
            relative_aperture,
            shutter_duration,
            sensitivity,
            max_exposure,
        }
    }

    /// Computes the exposure with which to scale the incident scene luminance
    /// according to the current settings.
    ///
    /// If the sensitivity is set to automatic, the given closure is used to
    /// obtain the average luminance incident from the scene, which is used to
    /// determine the highest exposure that does not lead to an overly clipped
    /// output using the Saturation Based Sensitivity method.
    ///
    /// The exposure is clamped to the maximum exposure of the camera.
    pub fn compute_exposure(
        &self,
        obtain_average_luminance: impl FnOnce() -> Result<f32>,
    ) -> Result<f32> {
        let ev_100 = match self.sensitivity {
            SensorSensitivity::Manual { iso } => self.compute_exposure_value_at_100_iso(iso),
            SensorSensitivity::Auto { ev_compensation } => {
                let average_luminance = obtain_average_luminance()?;
                let ev_100 = Self::compute_exposure_value_at_100_iso_for_correct_exposure_with_average_luminance(average_luminance);
                ev_100 - ev_compensation
            }
        };

        let max_luminance = Self::compute_maximum_luminance_to_saturate_sensor(ev_100);

        let exposure = max_luminance.recip();

        Ok(f32::min(self.max_exposure, exposure))
    }

    fn compute_exposure_value_at_100_iso(&self, iso: f32) -> f32 {
        f32::log2(self.relative_aperture.powi(2) * 100.0 / (self.shutter_duration * iso))
    }

    fn compute_exposure_value_at_100_iso_for_correct_exposure_with_average_luminance(
        average_luminance: f32,
    ) -> f32 {
        f32::log2(100.0 * average_luminance / Self::CALIBRATION_CONSTANT)
    }

    fn compute_maximum_luminance_to_saturate_sensor(exposure_value_at_100_iso: f32) -> f32 {
        (78.0 / (100.0 * Self::ATTENUATION_FACTOR)) * f32::exp2(exposure_value_at_100_iso)
    }
}

impl SensorSensitivity {
    /// Whether the sensor sensitivity is to be determined automatically.
    pub fn is_auto(&self) -> bool {
        matches!(self, Self::Auto { .. })
    }

    /// Changes the sensor sensitivity by the given number of F-stops (can be
    /// positive or negative).
    pub fn change_by_stops(&mut self, f_stops: f32) {
        match self {
            SensorSensitivity::Auto { ev_compensation } => {
                *ev_compensation += f_stops;
            }
            SensorSensitivity::Manual { iso } => {
                *iso *= f_stops.exp2();
            }
        }
    }
}

impl CapturingCamera {
    /// Creates a new capturing camera along with the required render commands
    /// according to the given configuration.
    pub(super) fn new(
        config: CapturingCameraConfig,
        graphics_device: &GraphicsDevice,
        rendering_surface: &RenderingSurface,
        shader_manager: &mut ShaderManager,
        render_attachment_texture_manager: &mut RenderAttachmentTextureManager,
        gpu_resource_group_manager: &mut GPUResourceGroupManager,
        storage_gpu_buffer_manager: &mut StorageGPUBufferManager,
        bind_group_layout_registry: &BindGroupLayoutRegistry,
    ) -> Result<Self> {
        let average_luminance_commands = AverageLuminanceComputeCommands::new(
            config.average_luminance_computation.clone(),
            graphics_device,
            shader_manager,
            render_attachment_texture_manager,
            gpu_resource_group_manager,
            storage_gpu_buffer_manager,
        )?;

        let bloom_commands = BloomRenderCommands::new(
            config.bloom,
            graphics_device,
            shader_manager,
            render_attachment_texture_manager,
        );

        let dynamic_range_compression_commands = DynamicRangeCompressionRenderCommands::new(
            config.dynamic_range_compression,
            graphics_device,
            rendering_surface,
            shader_manager,
            render_attachment_texture_manager,
            gpu_resource_group_manager,
            bind_group_layout_registry,
        )?;

        let initial_exposure = config
            .settings
            .compute_exposure(|| {
                Ok(config
                    .average_luminance_computation
                    .luminance_bounds
                    .lower())
            })
            .unwrap();

        Ok(Self {
            settings: config.settings,
            exposure: initial_exposure,
            average_luminance_commands,
            bloom_commands,
            dynamic_range_compression_commands,
        })
    }

    /// Returns the exposure push constant.
    pub fn exposure_push_constant(&self) -> f32 {
        self.exposure
    }

    /// Returns the inverse exposure push constant.
    pub fn inverse_exposure_push_constant(&self) -> f32 {
        self.exposure.recip()
    }

    /// Records the render commands that should be performed before dynamic
    /// range compression (tone mapping and gamma correction) into the given
    /// command encoder.
    ///
    /// # Errors
    /// Returns an error if any of the required GPU resources are missing.
    pub fn record_commands_before_dynamic_range_compression(
        &self,
        rendering_surface: &RenderingSurface,
        resource_registries: &impl BasicResourceRegistries,
        gpu_resources: &impl BasicGPUResources,
        render_attachment_texture_manager: &RenderAttachmentTextureManager,
        gpu_resource_group_manager: &GPUResourceGroupManager,
        storage_gpu_buffer_manager: &StorageGPUBufferManager,
        postprocessor: &Postprocessor,
        timestamp_recorder: &mut TimestampQueryRegistry<'_>,
        command_encoder: &mut wgpu::CommandEncoder,
    ) -> Result<()> {
        self.average_luminance_commands.record(
            rendering_surface,
            gpu_resource_group_manager,
            storage_gpu_buffer_manager,
            render_attachment_texture_manager,
            postprocessor,
            timestamp_recorder,
            self.settings.sensitivity.is_auto(),
            command_encoder,
        )?;
        self.bloom_commands.record(
            resource_registries,
            gpu_resources,
            render_attachment_texture_manager,
            timestamp_recorder,
            command_encoder,
        )
    }

    /// Records the render commands for dynamic range compression (tone mapping
    /// and gamma correction) into the given command encoder.
    ///
    /// # Errors
    /// Returns an error if any of the required GPU resources are missing.
    pub fn record_dynamic_range_compression_render_commands(
        &self,
        rendering_surface: &RenderingSurface,
        surface_texture_view: &wgpu::TextureView,
        resource_registries: &impl BasicResourceRegistries,
        gpu_resources: &impl BasicGPUResources,
        render_attachment_texture_manager: &RenderAttachmentTextureManager,
        gpu_resource_group_manager: &GPUResourceGroupManager,
        postprocessor: &Postprocessor,
        frame_counter: u32,
        timestamp_recorder: &mut TimestampQueryRegistry<'_>,
        command_encoder: &mut wgpu::CommandEncoder,
    ) -> Result<()> {
        self.dynamic_range_compression_commands.record(
            rendering_surface,
            surface_texture_view,
            resource_registries,
            gpu_resources,
            render_attachment_texture_manager,
            gpu_resource_group_manager,
            postprocessor,
            frame_counter,
            timestamp_recorder,
            command_encoder,
        )
    }

    /// Updates the exposure based on the current settings and potentially the
    /// average incident luminance.
    pub fn update_exposure(
        &mut self,
        graphics_device: &GraphicsDevice,
        storage_gpu_buffer_manager: &StorageGPUBufferManager,
    ) -> Result<()> {
        self.exposure = self.settings.compute_exposure(|| {
            average_luminance::load_computed_average_luminance(
                graphics_device,
                storage_gpu_buffer_manager,
            )
            .unwrap()
        })?;
        Ok(())
    }

    pub fn average_luminance_computation_config(&self) -> &AverageLuminanceComputationConfig {
        self.average_luminance_commands.config()
    }

    /// Sets the given average luminance computation configuration parameters
    /// and updates the appropriate render resources.
    pub fn set_average_luminance_computation_config(
        &mut self,
        graphics_device: &GraphicsDevice,
        gpu_resource_group_manager: &GPUResourceGroupManager,
        config: AverageLuminanceComputationConfig,
    ) {
        self.average_luminance_commands.set_config(
            graphics_device,
            gpu_resource_group_manager,
            config,
        );
    }

    pub fn produces_bloom_mut(&mut self) -> &mut bool {
        self.bloom_commands.enabled_mut()
    }

    pub fn bloom_config(&self) -> &BloomConfig {
        self.bloom_commands.config()
    }

    /// Sets the given bloom computation configuration parameters and updates
    /// the appropriate render resources.
    pub fn set_bloom_config(
        &mut self,
        graphics_device: &GraphicsDevice,
        shader_manager: &mut ShaderManager,
        render_attachment_texture_manager: &mut RenderAttachmentTextureManager,
        config: BloomConfig,
    ) {
        self.bloom_commands.set_config(
            graphics_device,
            shader_manager,
            render_attachment_texture_manager,
            config,
        );
    }

    pub fn dynamic_range_compression_config(&self) -> &DynamicRangeCompressionConfig {
        self.dynamic_range_compression_commands.config()
    }

    pub fn dynamic_range_compression_config_mut(&mut self) -> &mut DynamicRangeCompressionConfig {
        self.dynamic_range_compression_commands.config_mut()
    }

    pub fn settings_mut(&mut self) -> &mut CameraSettings {
        &mut self.settings
    }
}
