//! Shader template for the tone mapping pass.

use crate::{
    gpu::{
        push_constant::{PushConstantGroup, PushConstantVariant},
        rendering::postprocessing::capturing::tone_mapping::ToneMappingMethod,
        shader::template::{PostprocessingShaderTemplate, ShaderTemplate, SpecificShaderTemplate},
        texture::attachment::{
            RenderAttachmentInputDescriptionSet, RenderAttachmentOutputDescriptionSet,
            RenderAttachmentQuantity,
        },
    },
    mesh::buffer::MeshVertexAttributeLocation,
    rendering_template_source, template_replacements,
};
use std::sync::LazyLock;

/// Shader template for the tone mapping pass, which compresses the linear
/// luminance of an input render attachment to the [0, 1] range and writes the
/// result to the display surface.
#[derive(Clone, Debug)]
pub struct ToneMappingShaderTemplate {
    input_render_attachment_quantity: RenderAttachmentQuantity,
    method: ToneMappingMethod,
    push_constants: PushConstantGroup,
    input_render_attachments: RenderAttachmentInputDescriptionSet,
}

static TEMPLATE: LazyLock<ShaderTemplate<'static>> =
    LazyLock::new(|| ShaderTemplate::new(rendering_template_source!("tone_mapping")).unwrap());

impl ToneMappingShaderTemplate {
    /// Creates a new tone mapping shader template for the given input
    /// (luminance) render attachment quantity, using the given tone mapping
    /// method.
    pub fn new(
        input_render_attachment_quantity: RenderAttachmentQuantity,
        method: ToneMappingMethod,
    ) -> Self {
        let push_constants =
            PushConstantGroup::for_fragment([PushConstantVariant::InverseWindowDimensions]);

        let input_render_attachments = RenderAttachmentInputDescriptionSet::with_defaults(
            input_render_attachment_quantity.flag(),
        );

        Self {
            input_render_attachment_quantity,
            method,
            push_constants,
            input_render_attachments,
        }
    }
}

impl SpecificShaderTemplate for ToneMappingShaderTemplate {
    fn resolve(&self) -> String {
        TEMPLATE
            .resolve(
                [],
                template_replacements!(
                    "tone_mapping_method" => self.method,
                    "input_texture_binding" => self.input_render_attachment_quantity.texture_binding(),
                    "input_sampler_binding" => self.input_render_attachment_quantity.sampler_binding(),
                    "position_location" => MeshVertexAttributeLocation::Position as u32,
                ),
            )
            .expect("Shader template resolution failed")
    }
}

impl PostprocessingShaderTemplate for ToneMappingShaderTemplate {
    fn push_constants(&self) -> PushConstantGroup {
        self.push_constants.clone()
    }

    fn input_render_attachments(&self) -> RenderAttachmentInputDescriptionSet {
        self.input_render_attachments.clone()
    }

    fn output_render_attachments(&self) -> RenderAttachmentOutputDescriptionSet {
        RenderAttachmentOutputDescriptionSet::empty()
    }

    fn writes_to_surface(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod test {
    use super::super::test::validate_template;
    use super::*;

    #[test]
    fn should_resolve_to_valid_wgsl() {
        validate_template(&ToneMappingShaderTemplate::new(
            RenderAttachmentQuantity::Luminance,
            ToneMappingMethod::ACES,
        ));
    }
}
