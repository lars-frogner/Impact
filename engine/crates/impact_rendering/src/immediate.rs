//! Immediate data for rendering.

use impact_gpu::immediate::{Immediate, ImmediateGroup};
use impact_math::quaternion::UnitQuaternionC;
use std::mem;

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
    CameraRotationQuaternion,
    InstanceIdx,
    GenericVec3f32,
}

impl Immediate for BasicImmediate {
    fn size(&self) -> u32 {
        (match self {
            Self::FrameCounter | Self::LightIdx | Self::ShadowMapArrayIdx | Self::InstanceIdx => {
                mem::size_of::<u32>()
            }
            Self::InverseWindowDimensions => mem::size_of::<[f32; 2]>(),
            Self::Exposure | Self::InverseExposure | Self::PixelCount => mem::size_of::<f32>(),
            Self::CameraRotationQuaternion => mem::size_of::<UnitQuaternionC>(),
            Self::GenericVec3f32 => mem::size_of::<[f32; 3]>(),
        }) as u32
    }
}
