//! Push constants for rendering.

use impact_gpu::push_constant::{PushConstant, PushConstantGroup, PushConstantVariant};
use impact_math::quaternion::UnitQuaternion;
use std::mem;

pub type BasicPushConstant = PushConstant<BasicPushConstantVariant>;
pub type BasicPushConstantGroup = PushConstantGroup<BasicPushConstantVariant>;

/// The meaning of a push constant used for rendering.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BasicPushConstantVariant {
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

impl PushConstantVariant for BasicPushConstantVariant {
    fn size(&self) -> u32 {
        (match self {
            Self::FrameCounter
            | Self::LightIdx
            | Self::ShadowMapArrayIdx
            | Self::PixelCount
            | Self::InstanceIdx => mem::size_of::<u32>(),
            Self::InverseWindowDimensions => mem::size_of::<[f32; 2]>(),
            Self::Exposure | Self::InverseExposure => mem::size_of::<f32>(),
            Self::CameraRotationQuaternion => mem::size_of::<UnitQuaternion>(),
            Self::GenericVec3f32 => mem::size_of::<[f32; 3]>(),
        }) as u32
    }
}
