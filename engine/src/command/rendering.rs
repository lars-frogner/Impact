//! Commands for operating the rendering system.

pub mod postprocessing;

use crate::{
    command::uils::{ModifiedActiveState, ToActiveState},
    rendering::RenderingSystem,
    scene::Scene,
};
use anyhow::Result;
use impact_rendering::{
    attachment::RenderAttachmentQuantity,
    postprocessing::capturing::dynamic_range_compression::ToneMappingMethod,
};
use parking_lot::RwLock;
use postprocessing::{ToExposure, ToRenderAttachmentQuantity, ToToneMappingMethod};
use roc_integration::roc;

#[roc(parents = "Command")]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RenderingCommand {
    SetAmbientOcclusion(ToActiveState),
    SetTemporalAntiAliasing(ToActiveState),
    SetBloom(ToActiveState),
    SetToneMappingMethod(ToToneMappingMethod),
    SetExposure(ToExposure),
    SetRenderAttachmentVisualization(ToActiveState),
    SetVisualizedRenderAttachmentQuantity(ToRenderAttachmentQuantity),
    SetShadowMapping(ToActiveState),
    SetWireframeMode(ToActiveState),
    SetRenderPassTimings(ToActiveState),
}

pub fn set_ambient_occlusion(renderer: &RenderingSystem, to: ToActiveState) -> ModifiedActiveState {
    impact_log::info!("Setting ambient occlusion to {to:?}");
    postprocessing::set_ambient_occlusion(&mut renderer.postprocessor().write(), to)
}

pub fn set_temporal_anti_aliasing(
    renderer: &RenderingSystem,
    scene: &RwLock<Scene>,
    to: ToActiveState,
) -> ModifiedActiveState {
    impact_log::info!("Setting temporal anti-aliasing to {to:?}");

    let state =
        postprocessing::set_temporal_anti_aliasing(&mut renderer.postprocessor().write(), to);

    if state.changed {
        let scene = scene.read();
        let mut scene_camera = scene.scene_camera().write();

        if let Some(camera) = scene_camera.as_mut() {
            camera.set_jitter_enabled(state.state.is_enabled());
            renderer.declare_render_resources_desynchronized();
        }
    }
    state
}

pub fn set_bloom(renderer: &RenderingSystem, to: ToActiveState) -> ModifiedActiveState {
    impact_log::info!("Setting bloom to {to:?}");
    postprocessing::set_bloom(&mut renderer.postprocessor().write(), to)
}

pub fn set_tone_mapping_method(
    renderer: &RenderingSystem,
    to: ToToneMappingMethod,
) -> ToneMappingMethod {
    impact_log::info!("Setting tone mapping method to {to:?}");
    postprocessing::set_tone_mapping_method(&mut renderer.postprocessor().write(), to)
}

pub fn set_exposure(renderer: &RenderingSystem, to: ToExposure) {
    impact_log::info!("Setting exposure to {to:?}");
    postprocessing::set_exposure(&mut renderer.postprocessor().write(), to);
}

pub fn set_render_attachment_visualization(
    renderer: &RenderingSystem,
    to: ToActiveState,
) -> ModifiedActiveState {
    impact_log::info!("Setting render attachment visualization to {to:?}");
    postprocessing::set_render_attachment_visualization(&mut renderer.postprocessor().write(), to)
}

pub fn set_visualized_render_attachment_quantity(
    renderer: &RenderingSystem,
    to: ToRenderAttachmentQuantity,
) -> Result<RenderAttachmentQuantity> {
    impact_log::info!("Setting visualized render attachment quantity to {to:?}");
    postprocessing::set_visualized_render_attachment_quantity(
        &mut renderer.postprocessor().write(),
        to,
    )
}

pub fn set_shadow_mapping(
    renderer: &mut RenderingSystem,
    to: ToActiveState,
) -> ModifiedActiveState {
    impact_log::info!("Setting shadow mapping to {to:?}");
    to.set(renderer.shadow_mapping_enabled_mut())
}

pub fn set_wireframe_mode(
    renderer: &mut RenderingSystem,
    to: ToActiveState,
) -> ModifiedActiveState {
    impact_log::info!("Setting wireframe mode to {to:?}");
    let mut enabled = renderer.basic_config().wireframe_mode_on;
    let state = to.set(&mut enabled);
    if state.changed {
        renderer.set_wireframe_mode_enabled(enabled);
    }
    state
}

pub fn set_render_pass_timings(
    renderer: &mut RenderingSystem,
    to: ToActiveState,
) -> ModifiedActiveState {
    impact_log::info!("Setting render pass timings to {to:?}");
    let mut enabled = renderer.basic_config().timings_enabled;
    let state = to.set(&mut enabled);
    if state.changed {
        renderer.set_render_pass_timings_enabled(enabled);
    }
    state
}
