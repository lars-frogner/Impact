//! Capturing the incident scene luminance in a camera.

pub mod average_luminance;
pub mod bloom;
pub mod tone_mapping;

use crate::gpu::{
    query::TimestampQueryRegistry,
    rendering::{
        postprocessing::Postprocessor, resource::SynchronizedRenderResources,
        surface::RenderingSurface,
    },
    resource_group::GPUResourceGroupManager,
    shader::ShaderManager,
    storage::StorageGPUBufferManager,
    texture::attachment::RenderAttachmentTextureManager,
    GraphicsDevice,
};
use anyhow::Result;
use average_luminance::{AverageLuminanceComputationConfig, AverageLuminanceComputeCommands};
use bloom::{BloomConfig, BloomRenderCommands};
use std::mem;
use tone_mapping::{ToneMappingMethod, ToneMappingRenderCommands};

/// Configuration options for a capturing camera.
#[derive(Clone, Debug, Default)]
pub struct CapturingCameraConfig {
    /// The initial settings for the camera.
    pub initial_settings: CameraSettings,
    /// Configuration options for bloom.
    pub bloom: BloomConfig,
    /// Configuration options for the computation of the average luminance for
    /// automatic sensitivity.
    pub average_luminance_computation: AverageLuminanceComputationConfig,
    /// The initial tone mapping method to use.
    pub initial_tone_mapping: ToneMappingMethod,
}

/// Capturing settings for a camera.
#[derive(Clone, Debug)]
pub struct CameraSettings {
    /// The relative aperture of the camera, which is the ratio of the focal
    /// length to the aperture diameter.
    ///
    /// # Unit
    /// F-stops.
    relative_aperture: f32,
    /// The duration the sensor is exposed.
    ///
    /// # Unit
    /// Seconds.
    shutter_speed: f32,
    /// The sensitivity of the camera sensor.
    sensitivity: SensorSensitivity,
    /// The maximum exposure of the camera sensor. This corresponds to the
    /// reciprocal of the minimum incident luminance in cd/m² that can saturate
    /// the sensor.
    max_exposure: f32,
}

/// The sensitivity of a camera sensor, which may be set manually as an ISO
/// value or determined automatically based on the incident luminance, with
/// optional exposure value compensation in f-stops.
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
    produces_bloom: bool,
    tone_mapping_method: ToneMappingMethod,
    average_luminance_commands: AverageLuminanceComputeCommands,
    bloom_commands: BloomRenderCommands,
    tone_mapping_commands: ToneMappingRenderCommands,
}

impl Default for CameraSettings {
    fn default() -> Self {
        Self {
            relative_aperture: 1.0 / 4.0,
            shutter_speed: 1.0 / 200.0,
            sensitivity: SensorSensitivity::Auto {
                ev_compensation: 0.0,
            },
            max_exposure: 1e-1,
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
        shutter_speed: f32,
        sensitivity: SensorSensitivity,
        max_exposure: f32,
    ) -> Self {
        Self {
            relative_aperture,
            shutter_speed,
            sensitivity,
            max_exposure,
        }
    }

    /// Returns the relative aperture in f-stops.
    pub fn relative_aperture(&self) -> f32 {
        self.relative_aperture
    }

    /// Returns the camera shutter speed in seconds.
    pub fn shutter_speed(&self) -> f32 {
        self.shutter_speed
    }

    /// Returns the sensor sensitivity.
    pub fn sensitivity(&self) -> SensorSensitivity {
        self.sensitivity
    }

    /// Returns a mutable reference to the sensor sensitivity.
    pub fn sensitivity_mut(&mut self) -> &mut SensorSensitivity {
        &mut self.sensitivity
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
        f32::log2(self.relative_aperture.powi(2) * 100.0 / (self.shutter_speed * iso))
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
}

impl CapturingCamera {
    /// Creates a new capturing camera along with the required render commands
    /// according to the given configuration.
    pub(super) fn new(
        graphics_device: &GraphicsDevice,
        rendering_surface: &RenderingSurface,
        shader_manager: &mut ShaderManager,
        render_attachment_texture_manager: &mut RenderAttachmentTextureManager,
        gpu_resource_group_manager: &mut GPUResourceGroupManager,
        storage_gpu_buffer_manager: &mut StorageGPUBufferManager,
        config: &CapturingCameraConfig,
    ) -> Result<Self> {
        let average_luminance_commands = AverageLuminanceComputeCommands::new(
            graphics_device,
            shader_manager,
            render_attachment_texture_manager,
            gpu_resource_group_manager,
            storage_gpu_buffer_manager,
            &config.average_luminance_computation,
        )?;

        let bloom_commands = BloomRenderCommands::new(
            graphics_device,
            shader_manager,
            render_attachment_texture_manager,
            &config.bloom,
        )?;

        let tone_mapping_commands = ToneMappingRenderCommands::new(
            graphics_device,
            rendering_surface,
            shader_manager,
            render_attachment_texture_manager,
            gpu_resource_group_manager,
        )?;

        let settings = config.initial_settings.clone();

        let initial_exposure = match settings.sensitivity() {
            SensorSensitivity::Auto { .. } => 0.0,
            SensorSensitivity::Manual { iso } => settings.compute_exposure_value_at_100_iso(iso),
        };

        Ok(Self {
            settings,
            exposure: initial_exposure,
            produces_bloom: config.bloom.initially_enabled,
            tone_mapping_method: config.initial_tone_mapping,
            average_luminance_commands,
            bloom_commands,
            tone_mapping_commands,
        })
    }

    /// Returns the size of the push constant obtained by calling
    /// [`Self::exposure_push_constant`].
    pub const fn exposure_push_constant_size() -> u32 {
        mem::size_of::<f32>() as u32
    }

    /// Returns the exposure push constant.
    pub fn exposure_push_constant(&self) -> f32 {
        self.exposure
    }

    /// Returns the size of the push constant obtained by calling
    /// [`Self::inverse_exposure_push_constant`].
    pub const fn inverse_exposure_push_constant_size() -> u32 {
        mem::size_of::<f32>() as u32
    }

    /// Returns the inverse exposure push constant.
    pub fn inverse_exposure_push_constant(&self) -> f32 {
        self.exposure.recip()
    }

    /// Records the render commands that should be performed before tone mapping
    /// into the given command encoder.
    ///
    /// # Errors
    /// Returns an error if any of the required GPU resources are missing.
    pub fn record_commands_before_tone_mapping(
        &self,
        rendering_surface: &RenderingSurface,
        render_resources: &SynchronizedRenderResources,
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
            self.settings.sensitivity().is_auto(),
            command_encoder,
        )?;
        self.bloom_commands.record(
            render_resources,
            render_attachment_texture_manager,
            timestamp_recorder,
            self.produces_bloom,
            command_encoder,
        )
    }

    /// Records the render commands for tone mapping into the given command
    /// encoder.
    ///
    /// # Errors
    /// Returns an error if any of the required GPU resources are missing.
    pub fn record_tone_mapping_render_commands(
        &self,
        rendering_surface: &RenderingSurface,
        surface_texture_view: &wgpu::TextureView,
        render_resources: &SynchronizedRenderResources,
        render_attachment_texture_manager: &RenderAttachmentTextureManager,
        gpu_resource_group_manager: &GPUResourceGroupManager,
        postprocessor: &Postprocessor,
        frame_counter: u32,
        timestamp_recorder: &mut TimestampQueryRegistry<'_>,
        command_encoder: &mut wgpu::CommandEncoder,
    ) -> Result<()> {
        self.tone_mapping_commands.record(
            rendering_surface,
            surface_texture_view,
            render_resources,
            render_attachment_texture_manager,
            gpu_resource_group_manager,
            postprocessor,
            frame_counter,
            timestamp_recorder,
            self.tone_mapping_method,
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

    /// Toggles bloom.
    pub fn toggle_bloom(&mut self) {
        self.produces_bloom = !self.produces_bloom;
    }

    /// Cycles tone mapping.
    pub fn cycle_tone_mapping(&mut self) {
        self.tone_mapping_method = match self.tone_mapping_method {
            ToneMappingMethod::None => ToneMappingMethod::ACES,
            ToneMappingMethod::ACES => ToneMappingMethod::KhronosPBRNeutral,
            ToneMappingMethod::KhronosPBRNeutral => ToneMappingMethod::None,
        };
    }

    /// Increases the sensor sensitivity by a small multiplicative factor.
    pub fn increase_sensitivity(&mut self) {
        match self.settings.sensitivity_mut() {
            SensorSensitivity::Auto { ev_compensation } => {
                *ev_compensation += 0.1;
            }
            SensorSensitivity::Manual { iso } => {
                *iso *= 1.1;
            }
        }
    }

    /// Decreases the sensor sensitivity by a small multiplicative factor.
    pub fn decrease_sensitivity(&mut self) {
        match self.settings.sensitivity_mut() {
            SensorSensitivity::Auto { ev_compensation } => {
                *ev_compensation -= 0.1;
            }
            SensorSensitivity::Manual { iso } => {
                *iso /= 1.1;
            }
        }
    }
}
