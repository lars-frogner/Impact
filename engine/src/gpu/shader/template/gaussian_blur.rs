//! Shader template for a Gaussian blur pass.

use crate::{
    gpu::{
        push_constant::{PushConstantGroup, PushConstantVariant},
        rendering::postprocessing::gaussian_blur::{
            GaussianBlurDirection, MAX_GAUSSIAN_BLUR_UNIQUE_WEIGHTS,
        },
        resource_group::GPUResourceGroupID,
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

/// Shader template for a Gaussian blur pass, which blurs an input render
/// attachment along the horizontal or vertical axis and writes the result to
/// an output render attachment.
#[derive(Clone, Debug)]
pub struct GaussianBlurShaderTemplate {
    samples_resource_group_id: GPUResourceGroupID,
    input_render_attachment_quantity: RenderAttachmentQuantity,
    direction: GaussianBlurDirection,
    push_constants: PushConstantGroup,
    input_render_attachments: RenderAttachmentInputDescriptionSet,
    output_render_attachments: RenderAttachmentOutputDescriptionSet,
}

static TEMPLATE: LazyLock<ShaderTemplate<'static>> =
    LazyLock::new(|| ShaderTemplate::new(rendering_template_source!("gaussian_blur")).unwrap());

impl GaussianBlurShaderTemplate {
    /// Creates a new shader template for a Gaussian blur pass for the given
    /// input and output render attachment quantities, blending mode and
    /// blur direction, and using the given resource group for the sample
    /// uniform.
    pub fn new(
        samples_resource_group_id: GPUResourceGroupID,
        input_render_attachment_quantity: RenderAttachmentQuantity,
        output_render_attachment_quantity: RenderAttachmentQuantity,
        blending: Blending,
        direction: GaussianBlurDirection,
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
            samples_resource_group_id,
            input_render_attachment_quantity,
            direction,
            push_constants,
            input_render_attachments,
            output_render_attachments,
        }
    }
}

impl SpecificShaderTemplate for GaussianBlurShaderTemplate {
    fn resolve(&self) -> String {
        TEMPLATE
            .resolve(
                [],
                template_replacements!(
                    "direction" => self.direction,
                    "max_samples" => MAX_GAUSSIAN_BLUR_UNIQUE_WEIGHTS,
                    "input_texture_group" => 0,
                    "input_texture_binding" => self.input_render_attachment_quantity.texture_binding(),
                    "input_sampler_binding" => self.input_render_attachment_quantity.sampler_binding(),
                    "samples_group" => 1,
                    "samples_binding" => 0,
                    "position_location" => MeshVertexAttributeLocation::Position as u32,
                ),
            )
            .expect("Shader template resolution failed")
    }
}

impl PostprocessingShaderTemplate for GaussianBlurShaderTemplate {
    fn push_constants(&self) -> PushConstantGroup {
        self.push_constants.clone()
    }

    fn input_render_attachments(&self) -> RenderAttachmentInputDescriptionSet {
        self.input_render_attachments.clone()
    }

    fn output_render_attachments(&self) -> RenderAttachmentOutputDescriptionSet {
        self.output_render_attachments.clone()
    }

    fn gpu_resource_group_id(&self) -> Option<GPUResourceGroupID> {
        Some(self.samples_resource_group_id)
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::validate_template;
    use super::*;
    use impact_math::hash64;

    #[test]
    fn should_resolve_to_valid_wgsl() {
        validate_template(&GaussianBlurShaderTemplate::new(
            GPUResourceGroupID(hash64!("test".to_string())),
            RenderAttachmentQuantity::LuminanceHistory,
            RenderAttachmentQuantity::Luminance,
            Blending::Replace,
            GaussianBlurDirection::Horizontal,
        ));
    }
}
