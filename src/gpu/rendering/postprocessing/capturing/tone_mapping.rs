//! Render passes for applying tone mapping.

use crate::{
    gpu::{
        push_constant::{PushConstant, PushConstantVariant},
        rendering::render_command::{
            RenderCommandSpecification, RenderPassHints, RenderPassSpecification,
        },
        shader::{template::SpecificShaderTemplate, ShaderManager},
        texture::attachment::{
            RenderAttachmentInputDescriptionSet, RenderAttachmentOutputDescriptionSet,
            RenderAttachmentQuantity,
        },
        GraphicsDevice,
    },
    mesh::{buffer::VertexBufferable, VertexPosition, SCREEN_FILLING_QUAD_MESH_ID},
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
                Self::None => "None",
                Self::ACES => "ACES",
                Self::KhronosPBRNeutral => "KhronosPBRNeutral",
            }
        )
    }
}

pub(super) fn create_tone_mapping_render_commands(
    graphics_device: &GraphicsDevice,
    shader_manager: &mut ShaderManager,
) -> Vec<RenderCommandSpecification> {
    ToneMapping::all()
        .map(|mapping| {
            create_tone_mapping_render_pass(
                graphics_device,
                shader_manager,
                RenderAttachmentQuantity::Luminance,
                mapping,
            )
        })
        .to_vec()
}

/// Creates a [`RenderCommandSpecification`] for a render pass that applies the
/// given tone mapping to the input attachment and writes the result to the
/// surface attachment.
fn create_tone_mapping_render_pass(
    graphics_device: &GraphicsDevice,
    shader_manager: &mut ShaderManager,
    input_render_attachment_quantity: RenderAttachmentQuantity,
    mapping: ToneMapping,
) -> RenderCommandSpecification {
    let (input_texture_binding, input_sampler_binding) =
        input_render_attachment_quantity.bindings();

    let shader_id = shader_manager
        .get_or_create_rendering_shader_from_template(
            graphics_device,
            SpecificShaderTemplate::ToneMapping,
            &[
                ("tone_mapping_method", mapping.to_string()),
                (
                    "position_location",
                    VertexPosition::BINDING_LOCATION.to_string(),
                ),
                ("input_texture_binding", input_texture_binding.to_string()),
                ("input_sampler_binding", input_sampler_binding.to_string()),
            ],
        )
        .unwrap();

    let input_render_attachments =
        RenderAttachmentInputDescriptionSet::with_defaults(input_render_attachment_quantity.flag());

    RenderCommandSpecification::RenderPass(RenderPassSpecification {
        explicit_mesh_id: Some(*SCREEN_FILLING_QUAD_MESH_ID),
        explicit_shader_id: Some(shader_id),
        input_render_attachments,
        output_render_attachments: RenderAttachmentOutputDescriptionSet::empty(), // We output directly to the surface
        push_constants: PushConstant::new(
            PushConstantVariant::InverseWindowDimensions,
            wgpu::ShaderStages::FRAGMENT,
        )
        .into(),
        hints: RenderPassHints::NO_DEPTH_PREPASS
            .union(RenderPassHints::NO_CAMERA)
            .union(RenderPassHints::WRITES_TO_SURFACE),
        label: format!("Tone mapping pass ({})", mapping),
        ..Default::default()
    })
}
