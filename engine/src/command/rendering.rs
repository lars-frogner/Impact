//! Commands for operating the rendering system.

pub mod postprocessing;

use crate::{
    command::uils::{ModifiedActiveState, ToActiveState},
    lock_order::OrderedRwLock,
    rendering::RenderingSystem,
    scene::Scene,
};
use anyhow::Result;
use impact_rendering::{
    attachment::RenderAttachmentQuantity,
    postprocessing::{
        ambient_occlusion::AmbientOcclusionConfig,
        capturing::{
            CameraSettings,
            average_luminance::AverageLuminanceComputationConfig,
            bloom::BloomConfig,
            dynamic_range_compression::{DynamicRangeCompressionConfig, ToneMappingMethod},
        },
        temporal_anti_aliasing::TemporalAntiAliasingConfig,
    },
};
use parking_lot::RwLock;
use postprocessing::{ToExposure, ToRenderAttachmentQuantity, ToToneMappingMethod};

#[derive(Clone, Debug)]
pub enum RenderingCommand {
    SetAmbientOcclusion(ToActiveState),
    SetAmbientOcclusionConfig(AmbientOcclusionConfig),
    SetTemporalAntiAliasing(ToActiveState),
    SetTemporalAntiAliasingConfig(TemporalAntiAliasingConfig),
    SetBloom(ToActiveState),
    SetBloomConfig(BloomConfig),
    SetCameraSettings(CameraSettings),
    SetAverageLuminanceComputationConfig(AverageLuminanceComputationConfig),
    SetToneMappingMethod(ToToneMappingMethod),
    SetDynamicRangeCompressionConfig(DynamicRangeCompressionConfig),
    SetExposure(ToExposure),
    SetRenderAttachmentVisualization(ToActiveState),
    SetVisualizedRenderAttachmentQuantity(ToRenderAttachmentQuantity),
    SetShadowMapping(ToActiveState),
    SetWireframeMode(ToActiveState),
    SetRenderPassTimings(ToActiveState),
}

pub fn set_ambient_occlusion(renderer: &RenderingSystem, to: ToActiveState) -> ModifiedActiveState {
    impact_log::info!("Setting ambient occlusion to {to:?}");
    postprocessing::set_ambient_occlusion(&mut renderer.postprocessor().owrite(), to)
}

pub fn set_temporal_anti_aliasing(
    scene: &RwLock<Scene>,
    renderer: &RwLock<RenderingSystem>,
    to: ToActiveState,
) -> ModifiedActiveState {
    impact_log::info!("Setting temporal anti-aliasing to {to:?}");

    let state = postprocessing::set_temporal_anti_aliasing(
        &mut renderer.owrite().postprocessor().owrite(),
        to,
    );

    if state.changed {
        let scene = scene.oread();
        scene
            .camera_manager()
            .owrite()
            .set_jitter_enabled(state.state.is_enabled());
    }
    state
}

pub fn set_bloom(renderer: &RenderingSystem, to: ToActiveState) -> ModifiedActiveState {
    impact_log::info!("Setting bloom to {to:?}");
    postprocessing::set_bloom(&mut renderer.postprocessor().owrite(), to)
}

pub fn set_ambient_occlusion_config(renderer: &RenderingSystem, config: AmbientOcclusionConfig) {
    impact_log::info!("Setting ambient occlusion config to {config:?}");
    let gpu_resource_group_manager = renderer.gpu_resource_group_manager().oread();
    let mut postprocessor = renderer.postprocessor().owrite();
    postprocessor.set_ambient_occlusion_config(
        renderer.graphics_device(),
        &gpu_resource_group_manager,
        config,
    );
}

pub fn set_temporal_anti_aliasing_config(
    renderer: &RenderingSystem,
    config: TemporalAntiAliasingConfig,
) {
    impact_log::info!("Setting temporal anti-aliasing config to {config:?}");
    let gpu_resource_group_manager = renderer.gpu_resource_group_manager().oread();
    let mut postprocessor = renderer.postprocessor().owrite();
    postprocessor.set_temporal_anti_aliasing_config(
        renderer.graphics_device(),
        &gpu_resource_group_manager,
        config,
    );
}

pub fn set_bloom_config(renderer: &RenderingSystem, config: BloomConfig) {
    impact_log::info!("Setting bloom config to {config:?}");
    let mut shader_manager = renderer.shader_manager().owrite();
    let mut render_attachment_texture_manager =
        renderer.render_attachment_texture_manager().owrite();
    let mut postprocessor = renderer.postprocessor().owrite();
    postprocessor.capturing_camera_mut().set_bloom_config(
        renderer.graphics_device(),
        &mut shader_manager,
        &mut render_attachment_texture_manager,
        config,
    );
}

pub fn set_camera_settings(renderer: &RenderingSystem, settings: CameraSettings) {
    impact_log::info!("Setting camera settings to {settings:?}");
    let mut postprocessor = renderer.postprocessor().owrite();
    *postprocessor.capturing_camera_mut().settings_mut() = settings;
}

pub fn set_average_luminance_computation_config(
    renderer: &RenderingSystem,
    config: AverageLuminanceComputationConfig,
) {
    impact_log::info!("Setting average luminance computation config to {config:?}");
    let gpu_resource_group_manager = renderer.gpu_resource_group_manager().oread();
    let mut postprocessor = renderer.postprocessor().owrite();
    postprocessor
        .capturing_camera_mut()
        .set_average_luminance_computation_config(
            renderer.graphics_device(),
            &gpu_resource_group_manager,
            config,
        );
}

pub fn set_tone_mapping_method(
    renderer: &RenderingSystem,
    to: ToToneMappingMethod,
) -> ToneMappingMethod {
    impact_log::info!("Setting tone mapping method to {to:?}");
    postprocessing::set_tone_mapping_method(&mut renderer.postprocessor().owrite(), to)
}

pub fn set_dynamic_range_compression_config(
    renderer: &RenderingSystem,
    config: DynamicRangeCompressionConfig,
) {
    impact_log::info!("Setting dynamic range compression config to {config:?}");
    let mut postprocessor = renderer.postprocessor().owrite();
    *postprocessor
        .capturing_camera_mut()
        .dynamic_range_compression_config_mut() = config;
}

pub fn set_exposure(renderer: &RenderingSystem, to: ToExposure) {
    impact_log::info!("Setting exposure to {to:?}");
    postprocessing::set_exposure(&mut renderer.postprocessor().owrite(), to);
}

pub fn set_render_attachment_visualization(
    renderer: &RenderingSystem,
    to: ToActiveState,
) -> ModifiedActiveState {
    impact_log::info!("Setting render attachment visualization to {to:?}");
    postprocessing::set_render_attachment_visualization(&mut renderer.postprocessor().owrite(), to)
}

pub fn set_visualized_render_attachment_quantity(
    renderer: &RenderingSystem,
    to: ToRenderAttachmentQuantity,
) -> Result<RenderAttachmentQuantity> {
    impact_log::info!("Setting visualized render attachment quantity to {to:?}");
    postprocessing::set_visualized_render_attachment_quantity(
        &mut renderer.postprocessor().owrite(),
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
