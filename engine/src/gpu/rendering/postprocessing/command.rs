//! Commands for controlling post-processing.

use super::{Postprocessor, capturing::SensorSensitivity};
use crate::{
    engine::command::{ModifiedActiveState, ToActiveState},
    gpu::{
        rendering::postprocessing::capturing::tone_mapping::ToneMappingMethod,
        texture::attachment::RenderAttachmentQuantity,
    },
};
use anyhow::Result;
use roc_codegen::roc;

#[roc(parents = "Command")]
#[derive(Clone, Debug)]
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
#[derive(Clone, Debug)]
pub enum ToToneMappingMethod {
    Next,
    Specific(ToneMappingMethod),
}

#[roc(parents = "Command")]
#[derive(Clone, Debug)]
pub enum ToExposure {
    DifferentByStops(f32),
    Auto { ev_compensation: f32 },
    Manual { iso: f32 },
}

#[roc(parents = "Command")]
#[derive(Clone, Debug)]
pub enum ToRenderAttachmentQuantity {
    Next,
    Previous,
    Specific(RenderAttachmentQuantity),
}

impl Postprocessor {
    pub fn set_ambient_occlusion(&mut self, to: ToActiveState) -> ModifiedActiveState {
        to.set(&mut self.ambient_occlusion_enabled)
    }

    pub fn set_temporal_anti_aliasing(&mut self, to: ToActiveState) -> ModifiedActiveState {
        to.set(&mut self.temporal_anti_aliasing_enabled)
    }

    pub fn set_bloom(&mut self, to: ToActiveState) -> ModifiedActiveState {
        to.set(self.capturing_camera.produces_bloom_mut())
    }

    pub fn set_tone_mapping_method(&mut self, to: ToToneMappingMethod) -> ToneMappingMethod {
        let method = self.capturing_camera.tone_mapping_method_mut();
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
                self.capturing_camera.change_sensitivity_by_stops(f_stops);
            }
            ToExposure::Auto { ev_compensation } => self
                .capturing_camera
                .set_sensor_sensitivity(SensorSensitivity::Auto { ev_compensation }),
            ToExposure::Manual { iso } => self
                .capturing_camera
                .set_sensor_sensitivity(SensorSensitivity::Manual { iso }),
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
