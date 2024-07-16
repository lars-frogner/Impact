//! Materials and render passes for applying tone mapping.

use crate::{
    gpu::{
        push_constant::{PushConstant, PushConstantVariant},
        rendering::render_command::{
            OutputAttachmentSampling, RenderCommandSpecification, RenderPassHints,
            RenderPassSpecification,
        },
        shader::{MaterialShaderInput, ToneMappingShaderInput},
        texture::attachment::{RenderAttachmentQuantity, RenderAttachmentQuantitySet},
    },
    material::{MaterialID, MaterialLibrary, MaterialSpecification},
    mesh::{VertexAttributeSet, SCREEN_FILLING_QUAD_MESH_ID},
};
use impact_utils::hash64;
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

pub(super) fn setup_tone_mapping_materials_and_render_commands(
    material_library: &mut MaterialLibrary,
) -> Vec<RenderCommandSpecification> {
    ToneMapping::all()
        .map(|mapping| {
            setup_tone_mapping_material_and_render_pass(
                material_library,
                RenderAttachmentQuantity::Luminance,
                mapping,
            )
        })
        .to_vec()
}

fn setup_tone_mapping_material_and_render_pass(
    material_library: &mut MaterialLibrary,
    input_render_attachment_quantity: RenderAttachmentQuantity,
    mapping: ToneMapping,
) -> RenderCommandSpecification {
    let material_id = MaterialID(hash64!(format!(
        "ToneMappingMaterial{{ mapping: {}, input: {} }}",
        mapping, input_render_attachment_quantity,
    )));
    let specification = material_library
        .material_specification_entry(material_id)
        .or_insert_with(|| create_tone_mapping_material(input_render_attachment_quantity, mapping));
    define_tone_mapping_pass(material_id, specification, mapping)
}

/// Creates a [`MaterialSpecification`] for a material that applies the given
/// tone mapping to the input attachment and writes the result to the output
/// attachment.
fn create_tone_mapping_material(
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

fn define_tone_mapping_pass(
    material_id: MaterialID,
    material_specification: &MaterialSpecification,
    mapping: ToneMapping,
) -> RenderCommandSpecification {
    RenderCommandSpecification::RenderPass(RenderPassSpecification {
        explicit_mesh_id: Some(*SCREEN_FILLING_QUAD_MESH_ID),
        explicit_material_id: Some(material_id),
        vertex_attribute_requirements: material_specification
            .vertex_attribute_requirements_for_shader(),
        input_render_attachment_quantities: material_specification
            .input_render_attachment_quantities(),
        output_render_attachment_quantities: material_specification
            .output_render_attachment_quantities(),
        output_attachment_sampling: OutputAttachmentSampling::Single,
        push_constants: PushConstant::new(
            PushConstantVariant::InverseWindowDimensions,
            wgpu::ShaderStages::FRAGMENT,
        )
        .into(),
        hints: material_specification.render_pass_hints(),
        label: format!("Tone mapping pass ({})", mapping),
        ..Default::default()
    })
}
