//! Material for applying tone mapping.

use crate::{
    geometry::VertexAttributeSet,
    gpu::{
        rendering::{RenderAttachmentQuantity, RenderAttachmentQuantitySet, RenderPassHints},
        shader::{MaterialShaderInput, ToneMappingShaderInput},
    },
    scene::MaterialSpecification,
};
use std::fmt::Display;

/// The method to use for tone mapping.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ToneMapping {
    None,
    #[default]
    ACES,
    KhronosPBRNeutral,
}

impl ToneMapping {
    /// Returns all available tone mapping methods.
    pub fn all() -> [Self; 3] {
        [Self::None, Self::ACES, Self::KhronosPBRNeutral]
    }
}

impl Display for ToneMapping {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::None => "none",
                Self::ACES => "ACES",
                Self::KhronosPBRNeutral => "Khronos PBR Neutral",
            }
        )
    }
}

/// Creates a [`MaterialSpecification`] for a material that applies the given
/// tone mapping to the input attachment and writes the result to the output
/// attachment.
pub fn create_tone_mapping_material(
    input_render_attachment_quantity: RenderAttachmentQuantity,
    mapping: ToneMapping,
) -> MaterialSpecification {
    MaterialSpecification::new(
        VertexAttributeSet::POSITION,
        VertexAttributeSet::empty(),
        input_render_attachment_quantity.flag(),
        RenderAttachmentQuantitySet::empty(), // We output directly to surface
        None,
        Vec::new(),
        RenderPassHints::NO_DEPTH_PREPASS
            .union(RenderPassHints::NO_CAMERA)
            .union(RenderPassHints::WRITES_TO_SURFACE),
        MaterialShaderInput::ToneMapping(ToneMappingShaderInput {
            mapping,
            input_texture_and_sampler_bindings: input_render_attachment_quantity.bindings(),
        }),
    )
}
