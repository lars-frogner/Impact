//! Commands for screen capture.

use crate::gpu::rendering::screen_capture::ScreenCapturer;
use roc_integration::roc;

#[roc(parents = "Command")]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CaptureCommand {
    SaveScreenshot,
    SaveShadowMaps(SaveShadowMapsFor),
}

#[roc(parents = "Command")]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SaveShadowMapsFor {
    OmnidirectionalLight,
    UnidirectionalLight,
}

pub fn request_screenshot_save(screen_capturer: &ScreenCapturer) {
    impact_log::info!("Requesting screenshot save");
    screen_capturer.request_screenshot_save();
}

pub fn request_shadow_map_saves(screen_capturer: &ScreenCapturer, save_for: SaveShadowMapsFor) {
    impact_log::info!("Requesting shadow map saves for {save_for:?}");
    match save_for {
        SaveShadowMapsFor::OmnidirectionalLight => {
            screen_capturer.request_omnidirectional_light_shadow_map_save();
        }
        SaveShadowMapsFor::UnidirectionalLight => {
            screen_capturer.request_unidirectional_light_shadow_map_save();
        }
    }
}
