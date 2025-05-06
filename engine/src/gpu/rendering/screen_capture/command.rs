//! Commands for screen capture.

use roc_codegen::roc;

#[roc(prefix = "Engine")]
#[derive(Clone, Debug)]
pub enum CaptureCommand {
    SaveScreenshot,
    SaveShadowMaps(SaveShadowMapsFor),
}

#[roc(prefix = "Engine")]
#[derive(Clone, Debug)]
pub enum SaveShadowMapsFor {
    OmnidirectionalLight,
    UnidirectionalLight,
}
