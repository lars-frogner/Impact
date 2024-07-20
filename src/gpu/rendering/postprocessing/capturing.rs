//! Capturing the incident scene luminance in a camera.

pub mod average_luminance;
pub mod bloom;
pub mod tone_mapping;

use crate::gpu::{
    rendering::{
        fre,
        render_command::{RenderCommandSpecification, RenderCommandState},
    },
    resource_group::GPUResourceGroupManager,
    shader::ShaderManager,
    storage::StorageGPUBufferManager,
    texture::attachment::RenderAttachmentTextureManager,
    GraphicsDevice,
};
use anyhow::Result;
use average_luminance::AverageLuminanceComputationConfig;
use bloom::BloomConfig;
use std::{iter, mem};
use tone_mapping::ToneMapping;

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
    pub initial_tone_mapping: ToneMapping,
}

/// Capturing settings for a camera.
#[derive(Clone, Debug)]
pub struct CameraSettings {
    /// The relative aperture of the camera, which is the ratio of the focal
    /// length to the aperture diameter.
    ///
    /// # Unit
    /// F-stops.
    relative_aperture: fre,
    /// The duration the sensor is exposed.
    ///
    /// # Unit
    /// Seconds.
    shutter_speed: fre,
    /// The sensitivity of the camera sensor.
    sensitivity: SensorSensitivity,
}

/// The sensitivity of a camera sensor, which may be set manually as an ISO
/// value or determined automatically based on the incident luminance, with
/// optional exposure value compensation in f-stops.
#[derive(Clone, Copy, Debug)]
pub enum SensorSensitivity {
    Manual { iso: fre },
    Auto { ev_compensation: fre },
}

/// A camera capturing the incident scene luminance.
#[derive(Clone, Debug)]
pub struct CapturingCamera {
    settings: CameraSettings,
    produces_bloom: bool,
    exposure: fre,
    tone_mapping: ToneMapping,
    bloom_commands: Vec<RenderCommandSpecification>,
    average_luminance_commands: Vec<RenderCommandSpecification>,
    tone_mapping_commands: Vec<RenderCommandSpecification>,
    average_luminance_computation_config: AverageLuminanceComputationConfig,
}

impl Default for CameraSettings {
    fn default() -> Self {
        Self {
            relative_aperture: 1.0 / 4.0,
            shutter_speed: 1.0 / 200.0,
            sensitivity: SensorSensitivity::Auto {
                ev_compensation: 0.0,
            },
        }
    }
}

impl CameraSettings {
    /// The fraction of attenuation by the lens and vignetting.
    const ATTENUATION_FACTOR: fre = 0.65;

    /// The reflected-light meter calibration constant.
    const CALIBRATION_CONSTANT: fre = 12.5;

    /// Groups the given camera settings.
    pub fn new(relative_aperture: fre, shutter_speed: fre, sensitivity: SensorSensitivity) -> Self {
        Self {
            relative_aperture,
            shutter_speed,
            sensitivity,
        }
    }

    /// Returns the relative aperture in f-stops.
    pub fn relative_aperture(&self) -> fre {
        self.relative_aperture
    }

    /// Returns the camera shutter speed in seconds.
    pub fn shutter_speed(&self) -> fre {
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
    pub fn compute_exposure(
        &self,
        obtain_average_luminance: impl FnOnce() -> Result<fre>,
    ) -> Result<fre> {
        let ev_100 = match self.sensitivity {
            SensorSensitivity::Manual { iso } => self.compute_exposure_value_at_100_iso(iso),
            SensorSensitivity::Auto { ev_compensation } => {
                let average_luminance = obtain_average_luminance()?;
                let ev_100 = Self::compute_exposure_value_at_100_iso_for_correct_exposure_with_average_luminance(average_luminance);
                ev_100 - ev_compensation
            }
        };

        let max_luminance = Self::compute_maximum_luminance_to_saturate_sensor(ev_100);

        Ok(max_luminance.recip())
    }

    fn compute_exposure_value_at_100_iso(&self, iso: fre) -> fre {
        fre::log2(self.relative_aperture.powi(2) * 100.0 / (self.shutter_speed * iso))
    }

    fn compute_exposure_value_at_100_iso_for_correct_exposure_with_average_luminance(
        average_luminance: fre,
    ) -> fre {
        fre::log2(100.0 * average_luminance / Self::CALIBRATION_CONSTANT)
    }

    fn compute_maximum_luminance_to_saturate_sensor(exposure_value_at_100_iso: fre) -> fre {
        (78.0 / (100.0 * Self::ATTENUATION_FACTOR)) * fre::exp2(exposure_value_at_100_iso)
    }
}

impl SensorSensitivity {
    /// Whether the sensor sensitivity is to be determined automatically.
    pub fn is_auto(&self) -> bool {
        matches!(self, Self::Auto { .. })
    }
}

impl CapturingCamera {
    pub const EXPOSURE_PUSH_CONSTANT_SIZE: u32 = mem::size_of::<fre>() as u32;
    pub const INVERSE_EXPOSURE_PUSH_CONSTANT_SIZE: u32 = mem::size_of::<fre>() as u32;

    /// Creates a new capturing camera along with the required ender commands
    /// according to the given configuration.
    pub(super) fn new(
        graphics_device: &GraphicsDevice,
        shader_manager: &mut ShaderManager,
        render_attachment_texture_manager: &RenderAttachmentTextureManager,
        gpu_resource_group_manager: &mut GPUResourceGroupManager,
        storage_gpu_buffer_manager: &mut StorageGPUBufferManager,
        config: &CapturingCameraConfig,
    ) -> Self {
        let bloom_commands = bloom::create_bloom_render_commands(
            graphics_device,
            shader_manager,
            gpu_resource_group_manager,
            &config.bloom,
        );

        let average_luminance_commands =
            average_luminance::setup_average_luminance_computations_and_render_commands(
                graphics_device,
                shader_manager,
                render_attachment_texture_manager,
                gpu_resource_group_manager,
                storage_gpu_buffer_manager,
                &config.average_luminance_computation,
            );

        let tone_mapping_commands =
            tone_mapping::create_tone_mapping_render_commands(graphics_device, shader_manager);

        let settings = config.initial_settings.clone();

        let initial_exposure = match settings.sensitivity() {
            SensorSensitivity::Auto { .. } => 0.0,
            SensorSensitivity::Manual { iso } => settings.compute_exposure_value_at_100_iso(iso),
        };

        Self {
            settings,
            produces_bloom: config.bloom.initially_enabled,
            exposure: initial_exposure,
            tone_mapping: config.initial_tone_mapping,
            bloom_commands,
            average_luminance_commands,
            tone_mapping_commands,
            average_luminance_computation_config: config.average_luminance_computation.clone(),
        }
    }

    /// Returns the exposure push constant.
    pub fn get_exposure_push_constant(&self) -> fre {
        self.exposure
    }

    /// Returns the inverse exposure push constant.
    pub fn get_inverse_exposure_push_constant(&self) -> fre {
        self.exposure.recip()
    }

    /// Returns an iterator over the specifications for all capturing
    /// render commands, in the order in which they are to be performed.
    pub fn render_commands(&self) -> impl Iterator<Item = RenderCommandSpecification> + '_ {
        assert_eq!(self.tone_mapping_commands.len(), ToneMapping::all().len());
        self.bloom_commands
            .iter()
            .cloned()
            .chain(self.average_luminance_commands.iter().cloned())
            .chain(self.tone_mapping_commands.iter().cloned())
    }

    /// Returns an iterator over the current states of all capturing render
    /// commands, in the same order as from [`Self::render_commands`].
    pub fn render_command_states(&self) -> impl Iterator<Item = RenderCommandState> + '_ {
        assert_eq!(self.tone_mapping_commands.len(), ToneMapping::all().len());
        iter::once(!self.produces_bloom)
            .chain(iter::repeat(self.produces_bloom).take(self.bloom_commands.len() - 1))
            .chain(
                iter::repeat(self.settings.sensitivity().is_auto())
                    .take(self.average_luminance_commands.len()),
            )
            .chain(ToneMapping::all().map(|mapping| mapping == self.tone_mapping))
            .map(RenderCommandState::active_if)
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

    /// Updates the required resources used by the capturing camera to account
    /// for the render attachment textures being recreated.
    pub fn handle_new_render_attachment_textures(
        &mut self,
        graphics_device: &GraphicsDevice,
        shader_manager: &mut ShaderManager,
        render_attachment_texture_manager: &RenderAttachmentTextureManager,
        gpu_resource_group_manager: &mut GPUResourceGroupManager,
        storage_gpu_buffer_manager: &mut StorageGPUBufferManager,
    ) {
        self.average_luminance_commands[0] =
            average_luminance::create_luminance_histogram_compute_pass(
                graphics_device,
                shader_manager,
                render_attachment_texture_manager,
                gpu_resource_group_manager,
                storage_gpu_buffer_manager,
                &self.average_luminance_computation_config.luminance_bounds,
            );
    }

    /// Toggles bloom.
    pub fn toggle_bloom(&mut self) {
        self.produces_bloom = !self.produces_bloom;
    }

    /// Cycles tone mapping.
    pub fn cycle_tone_mapping(&mut self) {
        self.tone_mapping = match self.tone_mapping {
            ToneMapping::None => ToneMapping::ACES,
            ToneMapping::ACES => ToneMapping::KhronosPBRNeutral,
            ToneMapping::KhronosPBRNeutral => ToneMapping::None,
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
