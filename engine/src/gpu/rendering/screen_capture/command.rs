//! Commands for screen capture.

use roc_codegen::roc;

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
