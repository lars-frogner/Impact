//! Commands for controlling post-processing.

use crate::{
    command::{ModifiedActiveState, ToActiveState},
    gpu::rendering::{
        attachment::RenderAttachmentQuantity,
        postprocessing::{
            Postprocessor,
            capturing::{SensorSensitivity, dynamic_range_compression::ToneMappingMethod},
        },
    },
};
use anyhow::Result;
use roc_integration::roc;

#[roc(parents = "Command")]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PostprocessingCommand {
    SetAmbientOcclusion(ToActiveState),
    SetTemporalAntiAliasing(ToActiveState),
    SetBloom(ToActiveState),
    SetToneMappingMethod(ToToneMappingMethod),
    SetExposure(ToExposure),
    SetRenderAttachmentVisualization(ToActiveState),
    SetVisualizedRenderAttachmentQuantity(ToRenderAttachmentQuantity),
}

#[roc(parents = "Command")]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ToToneMappingMethod {
    Next,
    Specific(ToneMappingMethod),
}

#[roc(parents = "Command")]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Clone, Debug)]
pub enum ToExposure {
    DifferentByStops(f32),
    Auto { ev_compensation: f32 },
    Manual { iso: f32 },
}

#[roc(parents = "Command")]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ToRenderAttachmentQuantity {
    Next,
    Previous,
    Specific(RenderAttachmentQuantity),
}

impl Postprocessor {
    pub fn set_ambient_occlusion(&mut self, to: ToActiveState) -> ModifiedActiveState {
        to.set(self.ambient_occlusion_commands.enabled_mut())
    }

    pub fn set_temporal_anti_aliasing(&mut self, to: ToActiveState) -> ModifiedActiveState {
        to.set(self.temporal_anti_aliasing_commands.enabled_mut())
    }

    pub fn set_bloom(&mut self, to: ToActiveState) -> ModifiedActiveState {
        to.set(self.capturing_camera.produces_bloom_mut())
    }

    pub fn set_tone_mapping_method(&mut self, to: ToToneMappingMethod) -> ToneMappingMethod {
        let method = &mut self
            .capturing_camera
            .dynamic_range_compression_config_mut()
            .tone_mapping_method;
        *method = match to {
            ToToneMappingMethod::Next => match *method {
                ToneMappingMethod::None => ToneMappingMethod::ACES,
                ToneMappingMethod::ACES => ToneMappingMethod::KhronosPBRNeutral,
                ToneMappingMethod::KhronosPBRNeutral => ToneMappingMethod::None,
            },
            ToToneMappingMethod::Specific(to) => to,
        };
        *method
    }

    pub fn set_exposure(&mut self, to: ToExposure) {
        match to {
            ToExposure::DifferentByStops(f_stops) => {
                self.capturing_camera
                    .settings_mut()
                    .sensitivity
                    .change_by_stops(f_stops);
            }
            ToExposure::Auto { ev_compensation } => {
                self.capturing_camera.settings_mut().sensitivity =
                    SensorSensitivity::Auto { ev_compensation }
            }
            ToExposure::Manual { iso } => {
                self.capturing_camera.settings_mut().sensitivity = SensorSensitivity::Manual { iso }
            }
        }
    }

    pub fn set_render_attachment_visualization(
        &mut self,
        to: ToActiveState,
    ) -> ModifiedActiveState {
        to.set(self.render_attachment_visualization_passes.enabled_mut())
    }

    pub fn set_visualized_render_attachment_quantity(
        &mut self,
        to: ToRenderAttachmentQuantity,
    ) -> Result<RenderAttachmentQuantity> {
        match to {
            ToRenderAttachmentQuantity::Next => {
                self.render_attachment_visualization_passes
                    .cycle_quantity_forward();
            }
            ToRenderAttachmentQuantity::Previous => {
                self.render_attachment_visualization_passes
                    .cycle_quantity_backward();
            }
            ToRenderAttachmentQuantity::Specific(to) => {
                self.render_attachment_visualization_passes
                    .set_quantity(to)?;
            }
        }
        Ok(self.render_attachment_visualization_passes.quantity())
    }
}

impl PartialEq for ToExposure {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Auto { ev_compensation: a }, Self::Auto { ev_compensation: b })
            | (Self::DifferentByStops(a), Self::DifferentByStops(b))
            | (Self::Manual { iso: a }, Self::Manual { iso: b }) => a.to_bits() == b.to_bits(),
            _ => false,
        }
    }
}

impl Eq for ToExposure {}
