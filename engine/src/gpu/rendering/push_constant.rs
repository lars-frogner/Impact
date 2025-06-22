//! Push constants for rendering.

use impact_gpu::push_constant::{PushConstant, PushConstantGroup, PushConstantVariant};
use nalgebra::UnitQuaternion;
use std::mem;

pub type RenderingPushConstant = PushConstant<RenderingPushConstantVariant>;
pub type RenderingPushConstantGroup = PushConstantGroup<RenderingPushConstantVariant>;

/// The meaning of a push constant used for rendering.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RenderingPushConstantVariant {
    InverseWindowDimensions,
    PixelCount,
    LightIdx,
    ShadowMapArrayIdx,
    Exposure,
    InverseExposure,
    FrameCounter,
    CameraRotationQuaternion,
    InstanceIdx,
    ChunkCount,
    GenericVec3f32,
}

impl PushConstantVariant for RenderingPushConstantVariant {
    fn size(&self) -> u32 {
        (match self {
            Self::FrameCounter
            | Self::LightIdx
            | Self::ShadowMapArrayIdx
            | Self::PixelCount
            | Self::InstanceIdx
            | Self::ChunkCount => mem::size_of::<u32>(),
            Self::InverseWindowDimensions => mem::size_of::<[f32; 2]>(),
            Self::Exposure | Self::InverseExposure => mem::size_of::<f32>(),
            Self::CameraRotationQuaternion => mem::size_of::<UnitQuaternion<f32>>(),
            Self::GenericVec3f32 => mem::size_of::<[f32; 3]>(),
        }) as u32
    }
}
