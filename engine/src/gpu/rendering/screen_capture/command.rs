//! Commands for screen capture.

use roc_codegen::roc;

#[roc(parents = "Command")]
#[derive(Clone, Debug)]
pub enum CaptureCommand {
    SaveScreenshot,
    SaveShadowMaps(SaveShadowMapsFor),
}

#[roc(parents = "Command")]
#[derive(Clone, Debug)]
pub enum SaveShadowMapsFor {
    OmnidirectionalLight,
    UnidirectionalLight,
}
