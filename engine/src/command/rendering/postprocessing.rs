//! Commands for controlling post-processing.

use crate::command::uils::{ModifiedActiveState, ToActiveState};
use anyhow::Result;
use impact_rendering::{
    attachment::RenderAttachmentQuantity,
    postprocessing::{
        Postprocessor,
        capturing::{SensorSensitivity, dynamic_range_compression::ToneMappingMethod},
    },
};

#[derive(Clone, Debug)]
pub enum ToToneMappingMethod {
    Next,
    Specific(ToneMappingMethod),
}

#[derive(Clone, Debug)]
pub enum ToExposure {
    DifferentByStops(f32),
    Auto { ev_compensation: f32 },
    Manual { iso: f32 },
}

#[derive(Clone, Debug)]
pub enum ToRenderAttachmentQuantity {
    Next,
    Previous,
    Specific(RenderAttachmentQuantity),
}

pub fn set_ambient_occlusion(
    postprocessor: &mut Postprocessor,
    to: ToActiveState,
) -> ModifiedActiveState {
    to.set(postprocessor.ambient_occlusion_commands_mut().enabled_mut())
}

pub fn set_temporal_anti_aliasing(
    postprocessor: &mut Postprocessor,
    to: ToActiveState,
) -> ModifiedActiveState {
    to.set(
        postprocessor
            .temporal_anti_aliasing_commands_mut()
            .enabled_mut(),
    )
}

pub fn set_bloom(postprocessor: &mut Postprocessor, to: ToActiveState) -> ModifiedActiveState {
    to.set(postprocessor.capturing_camera_mut().produces_bloom_mut())
}

pub fn set_tone_mapping_method(
    postprocessor: &mut Postprocessor,
    to: ToToneMappingMethod,
) -> ToneMappingMethod {
    let method = &mut postprocessor
        .capturing_camera_mut()
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

pub fn set_exposure(postprocessor: &mut Postprocessor, to: ToExposure) {
    match to {
        ToExposure::DifferentByStops(f_stops) => {
            postprocessor
                .capturing_camera_mut()
                .settings_mut()
                .sensitivity
                .change_by_stops(f_stops);
        }
        ToExposure::Auto { ev_compensation } => {
            postprocessor
                .capturing_camera_mut()
                .settings_mut()
                .sensitivity = SensorSensitivity::Auto { ev_compensation }
        }
        ToExposure::Manual { iso } => {
            postprocessor
                .capturing_camera_mut()
                .settings_mut()
                .sensitivity = SensorSensitivity::Manual { iso }
        }
    }
}

pub fn set_render_attachment_visualization(
    postprocessor: &mut Postprocessor,
    to: ToActiveState,
) -> ModifiedActiveState {
    to.set(
        postprocessor
            .render_attachment_visualization_passes_mut()
            .enabled_mut(),
    )
}

pub fn set_visualized_render_attachment_quantity(
    postprocessor: &mut Postprocessor,
    to: ToRenderAttachmentQuantity,
) -> Result<RenderAttachmentQuantity> {
    match to {
        ToRenderAttachmentQuantity::Next => {
            postprocessor
                .render_attachment_visualization_passes_mut()
                .cycle_quantity_forward();
        }
        ToRenderAttachmentQuantity::Previous => {
            postprocessor
                .render_attachment_visualization_passes_mut()
                .cycle_quantity_backward();
        }
        ToRenderAttachmentQuantity::Specific(to) => {
            postprocessor
                .render_attachment_visualization_passes_mut()
                .set_quantity(to)?;
        }
    }
    Ok(postprocessor
        .render_attachment_visualization_passes_mut()
        .quantity())
}
