//! Immediate data for rendering.

use impact_gpu::immediate::{Immediate, ImmediateGroup};

pub type BasicImmediateGroup = ImmediateGroup<BasicImmediate>;

/// The meaning of a piece of immediate data used for rendering.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BasicImmediate {
    InverseWindowDimensions,
    PixelCount,
    LightIdx,
    ShadowMapArrayIdx,
    Exposure,
    InverseExposure,
    FrameCounter,
    InstanceIdx,
    WorldToCameraRotation,
    WorldToCameraTranslation,
    PreviousWorldToCameraRotation,
    PreviousWorldToCameraTranslation,
}

impl Immediate for BasicImmediate {
    #[allow(clippy::match_same_arms)]
    fn size_and_alignment(&self) -> (u32, u32) {
        match self {
            Self::FrameCounter | Self::LightIdx | Self::ShadowMapArrayIdx | Self::InstanceIdx => {
                (4, 4) // u32
            }
            Self::Exposure | Self::InverseExposure | Self::PixelCount => (4, 4), // f32
            Self::InverseWindowDimensions => (8, 8),                             // vec2f
            Self::WorldToCameraRotation | Self::PreviousWorldToCameraRotation => (16, 16), // vec4f
            Self::WorldToCameraTranslation | Self::PreviousWorldToCameraTranslation => (12, 16), // vec3f
        }
    }
}
