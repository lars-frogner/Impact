//! Commands for operating the rendering system.

pub mod postprocessing;

use crate::{
    command::uils::{ModifiedActiveState, ToActiveState},
    lock_order::OrderedRwLock,
    rendering::RenderingSystem,
    scene::Scene,
};
use anyhow::Result;
use impact_light::shadow_map::ShadowMappingConfig;
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
pub enum RenderingAdminCommand {
    SetAmbientOcclusionConfig(AmbientOcclusionConfig),
    SetTemporalAntiAliasingConfig(TemporalAntiAliasingConfig),
    SetBloomConfig(BloomConfig),
    SetCameraSettings(CameraSettings),
    SetAverageLuminanceComputationConfig(AverageLuminanceComputationConfig),
    SetToneMappingMethod(ToToneMappingMethod),
    SetDynamicRangeCompressionConfig(DynamicRangeCompressionConfig),
    SetExposure(ToExposure),
    SetRenderAttachmentVisualization(ToActiveState),
    SetVisualizedRenderAttachmentQuantity(ToRenderAttachmentQuantity),
    SetShadowMappingConfig(ShadowMappingConfig),
    SetWireframeMode(ToActiveState),
    SetRenderPassTimings(ToActiveState),
}

pub fn set_ambient_occlusion_config(renderer: &RenderingSystem, config: AmbientOcclusionConfig) {
    log::info!("Setting ambient occlusion config to {config:?}");
    let gpu_resource_group_manager = renderer.gpu_resource_group_manager().oread();
    let mut postprocessor = renderer.postprocessor().owrite();
    postprocessor.set_ambient_occlusion_config(
        renderer.graphics_device(),
        &gpu_resource_group_manager,
        config,
    );
}

pub fn set_temporal_anti_aliasing_config(
    scene: &RwLock<Scene>,
    renderer: &RenderingSystem,
    to: TemporalAntiAliasingConfig,
) {
    log::info!("Setting temporal anti-aliasing config to {to:?}");
    let gpu_resource_group_manager = renderer.gpu_resource_group_manager().oread();
    let mut postprocessor = renderer.postprocessor().owrite();

    let config = postprocessor.temporal_anti_aliasing_config();

    if to.enabled != config.enabled {
        let scene = scene.oread();
        scene
            .camera_manager()
            .owrite()
            .set_jitter_enabled(to.enabled);
    }

    postprocessor.set_temporal_anti_aliasing_config(
        renderer.graphics_device(),
        &gpu_resource_group_manager,
        to,
    );
}

pub fn set_bloom_config(renderer: &RenderingSystem, config: BloomConfig) {
    log::info!("Setting bloom config to {config:?}");
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
    log::info!("Setting camera settings to {settings:?}");
    let mut postprocessor = renderer.postprocessor().owrite();
    *postprocessor.capturing_camera_mut().settings_mut() = settings;
}

pub fn set_average_luminance_computation_config(
    renderer: &RenderingSystem,
    config: AverageLuminanceComputationConfig,
) {
    log::info!("Setting average luminance computation config to {config:?}");
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
    log::info!("Setting tone mapping method to {to:?}");
    postprocessing::set_tone_mapping_method(&mut renderer.postprocessor().owrite(), to)
}

pub fn set_dynamic_range_compression_config(
    renderer: &RenderingSystem,
    config: DynamicRangeCompressionConfig,
) {
    log::info!("Setting dynamic range compression config to {config:?}");
    let mut postprocessor = renderer.postprocessor().owrite();
    *postprocessor
        .capturing_camera_mut()
        .dynamic_range_compression_config_mut() = config;
}

pub fn set_exposure(renderer: &RenderingSystem, to: ToExposure) {
    log::info!("Setting exposure to {to:?}");
    postprocessing::set_exposure(&mut renderer.postprocessor().owrite(), to);
}

pub fn set_render_attachment_visualization(
    renderer: &RenderingSystem,
    to: ToActiveState,
) -> ModifiedActiveState {
    log::info!("Setting render attachment visualization to {to:?}");
    postprocessing::set_render_attachment_visualization(&mut renderer.postprocessor().owrite(), to)
}

pub fn set_visualized_render_attachment_quantity(
    renderer: &RenderingSystem,
    to: ToRenderAttachmentQuantity,
) -> Result<RenderAttachmentQuantity> {
    log::info!("Setting visualized render attachment quantity to {to:?}");
    postprocessing::set_visualized_render_attachment_quantity(
        &mut renderer.postprocessor().owrite(),
        to,
    )
}

pub fn set_shadow_mapping_config(renderer: &mut RenderingSystem, mut to: ShadowMappingConfig) {
    log::info!("Setting shadow mapping to {to:?}");
    let config = renderer.shadow_mapping_config_mut();

    if to.omnidirectional_light_shadow_map_resolution
        != config.omnidirectional_light_shadow_map_resolution
    {
        log::warn!("Tried to change omnidirectional light shadow map resolution, ignoring");
        to.omnidirectional_light_shadow_map_resolution =
            config.omnidirectional_light_shadow_map_resolution;
    }
    if to.unidirectional_light_shadow_map_resolution
        != config.unidirectional_light_shadow_map_resolution
    {
        log::warn!("Tried to change unidirectional light shadow map resolution, ignoring");
        to.unidirectional_light_shadow_map_resolution =
            config.unidirectional_light_shadow_map_resolution;
    }

    *config = to;
}

pub fn set_wireframe_mode(
    renderer: &mut RenderingSystem,
    to: ToActiveState,
) -> ModifiedActiveState {
    log::info!("Setting wireframe mode to {to:?}");
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
    log::info!("Setting render pass timings to {to:?}");
    let mut enabled = renderer.basic_config().timings_enabled;
    let state = to.set(&mut enabled);
    if state.changed {
        renderer.set_render_pass_timings_enabled(enabled);
    }
    state
}
