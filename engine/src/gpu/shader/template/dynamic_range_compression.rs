//! Shader template for the dynamic range compression pass.

use crate::{
    gpu::{
        push_constant::{PushConstantGroup, PushConstantVariant},
        rendering::postprocessing::capturing::dynamic_range_compression::ToneMappingMethod,
        shader::template::{PostprocessingShaderTemplate, ShaderTemplate, SpecificShaderTemplate},
        texture::attachment::{
            RenderAttachmentInputDescriptionSet, RenderAttachmentOutputDescriptionSet,
            RenderAttachmentQuantity,
        },
    },
    mesh::buffer::TriangleMeshVertexAttributeLocation,
    rendering_template_source, template_replacements,
};
use std::sync::LazyLock;

/// Shader template for the dynamic range compression pass, which compresses the
/// linear luminance of an input render attachment to the [0, 1] range through
/// tone mapping and gamma correction and writes the result to the display
/// surface.
#[derive(Clone, Debug)]
pub struct DynamicRangeCompressionShaderTemplate {
    input_render_attachment_quantity: RenderAttachmentQuantity,
    tone_mapping_method: ToneMappingMethod,
    push_constants: PushConstantGroup,
    input_render_attachments: RenderAttachmentInputDescriptionSet,
}

static TEMPLATE: LazyLock<ShaderTemplate<'static>> = LazyLock::new(|| {
    ShaderTemplate::new(rendering_template_source!("dynamic_range_compression")).unwrap()
});

impl DynamicRangeCompressionShaderTemplate {
    /// Creates a new dynamic range compression shader template for the given
    /// input (luminance) render attachment quantity, using the given tone
    /// mapping method.
    pub fn new(
        input_render_attachment_quantity: RenderAttachmentQuantity,
        tone_mapping_method: ToneMappingMethod,
    ) -> Self {
        let push_constants =
            PushConstantGroup::for_fragment([PushConstantVariant::InverseWindowDimensions]);

        let input_render_attachments = RenderAttachmentInputDescriptionSet::with_defaults(
            input_render_attachment_quantity.flag(),
        );

        Self {
            input_render_attachment_quantity,
            tone_mapping_method,
            push_constants,
            input_render_attachments,
        }
    }
}

impl SpecificShaderTemplate for DynamicRangeCompressionShaderTemplate {
    fn resolve(&self) -> String {
        TEMPLATE
            .resolve(
                [],
                template_replacements!(
                    "tone_mapping_method" => self.tone_mapping_method,
                    "input_texture_binding" => self.input_render_attachment_quantity.texture_binding(),
                    "input_sampler_binding" => self.input_render_attachment_quantity.sampler_binding(),
                    "position_location" => TriangleMeshVertexAttributeLocation::Position as u32,
                ),
            )
            .expect("Shader template resolution failed")
    }
}

impl PostprocessingShaderTemplate for DynamicRangeCompressionShaderTemplate {
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
mod tests {
    use super::super::tests::validate_template;
    use super::*;

    #[test]
    fn should_resolve_to_valid_wgsl() {
        validate_template(&DynamicRangeCompressionShaderTemplate::new(
            RenderAttachmentQuantity::Luminance,
            ToneMappingMethod::ACES,
        ));
    }
}
