//! Shader template for passthrough passes.

use crate::{
    gpu::{
        push_constant::{PushConstantGroup, PushConstantVariant},
        rendering::render_command::StencilValue,
        shader::template::{PostprocessingShaderTemplate, ShaderTemplate, SpecificShaderTemplate},
        texture::attachment::{
            Blending, RenderAttachmentDescription, RenderAttachmentInputDescriptionSet,
            RenderAttachmentOutputDescription, RenderAttachmentOutputDescriptionSet,
            RenderAttachmentQuantity,
        },
    },
    mesh::buffer::MeshVertexAttributeLocation,
    rendering_template_source, template_replacements,
};
use std::sync::LazyLock;

/// Shader template for passthrough passes, which write texels from an input
/// render attachment to an output render attachment, with configurable stencil
/// testing and blending.
#[derive(Clone, Debug)]
pub struct PassthroughShaderTemplate {
    input_render_attachment_quantity: RenderAttachmentQuantity,
    push_constants: PushConstantGroup,
    input_render_attachments: RenderAttachmentInputDescriptionSet,
    output_render_attachments: RenderAttachmentOutputDescriptionSet,
    stencil_test: Option<(wgpu::CompareFunction, StencilValue)>,
}

static TEMPLATE: LazyLock<ShaderTemplate<'static>> =
    LazyLock::new(|| ShaderTemplate::new(rendering_template_source!("passthrough")).unwrap());

impl PassthroughShaderTemplate {
    /// Creates a new passthrough shader template for the given input and output
    /// render attachment quantities, with the given blending and stencil
    /// testing.
    pub fn new(
        input_render_attachment_quantity: RenderAttachmentQuantity,
        output_render_attachment_quantity: RenderAttachmentQuantity,
        blending: Blending,
        stencil_test: Option<(wgpu::CompareFunction, StencilValue)>,
    ) -> Self {
        let push_constants =
            PushConstantGroup::for_fragment([PushConstantVariant::InverseWindowDimensions]);

        let input_render_attachments = RenderAttachmentInputDescriptionSet::with_defaults(
            input_render_attachment_quantity.flag(),
        );

        let output_render_attachments = RenderAttachmentOutputDescriptionSet::single(
            RenderAttachmentOutputDescription::default_for(output_render_attachment_quantity)
                .with_blending(blending),
        );

        Self {
            input_render_attachment_quantity,
            push_constants,
            input_render_attachments,
            output_render_attachments,
            stencil_test,
        }
    }
}

impl SpecificShaderTemplate for PassthroughShaderTemplate {
    fn resolve(&self) -> String {
        TEMPLATE
            .resolve(
                [],
                template_replacements!(
                    "input_texture_binding" => self.input_render_attachment_quantity.texture_binding(),
                    "input_sampler_binding" => self.input_render_attachment_quantity.sampler_binding(),
                    "position_location" => MeshVertexAttributeLocation::Position as u32,
                ),
            )
            .expect("Shader template resolution failed")
    }
}

impl PostprocessingShaderTemplate for PassthroughShaderTemplate {
    fn push_constants(&self) -> PushConstantGroup {
        self.push_constants.clone()
    }

    fn input_render_attachments(&self) -> RenderAttachmentInputDescriptionSet {
        self.input_render_attachments.clone()
    }

    fn output_render_attachments(&self) -> RenderAttachmentOutputDescriptionSet {
        self.output_render_attachments.clone()
    }

    fn stencil_test(&self) -> Option<(wgpu::CompareFunction, StencilValue)> {
        self.stencil_test
    }
}

#[cfg(test)]
mod test {
    use super::super::test::validate_template;
    use super::*;

    #[test]
    fn should_resolve_to_valid_wgsl() {
        validate_template(&PassthroughShaderTemplate::new(
            RenderAttachmentQuantity::LuminanceHistory,
            RenderAttachmentQuantity::Luminance,
            Blending::Replace,
            None,
        ));
    }
}
