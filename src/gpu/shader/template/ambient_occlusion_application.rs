//! Shader template for the ambient occlusion application pass.

use crate::{
    gpu::{
        push_constant::{PushConstantGroup, PushConstantVariant},
        rendering::render_command::StencilValue,
        shader::template::{PostprocessingShaderTemplate, ShaderTemplate, SpecificShaderTemplate},
        texture::attachment::{
            Blending, RenderAttachmentInputDescription, RenderAttachmentInputDescriptionSet,
            RenderAttachmentOutputDescription, RenderAttachmentOutputDescriptionSet,
            RenderAttachmentQuantity::{self, AmbientReflectedLuminance, LinearDepth, Occlusion},
            RenderAttachmentQuantitySet, RenderAttachmentSampler,
        },
    },
    mesh::buffer::MeshVertexAttributeLocation,
    rendering_template_source, template_replacements,
};
use std::sync::LazyLock;

/// Shader template for the ambient occlusion application pass, which uses the
/// occlusion computed by the ambient occlusion computation pass to compute the
/// occluded ambient reflected luminance and adds it to the luminance
/// attachment.
#[derive(Clone, Debug)]
pub struct AmbientOcclusionApplicationShaderTemplate {
    push_constants: PushConstantGroup,
    input_render_attachments: RenderAttachmentInputDescriptionSet,
    output_render_attachments: RenderAttachmentOutputDescriptionSet,
}

static TEMPLATE: LazyLock<ShaderTemplate<'static>> = LazyLock::new(|| {
    ShaderTemplate::new(rendering_template_source!("ambient_occlusion_application")).unwrap()
});

impl AmbientOcclusionApplicationShaderTemplate {
    /// Creates a new ambient occlusion application shader template.
    pub fn new() -> Self {
        let push_constants =
            PushConstantGroup::for_fragment([PushConstantVariant::InverseWindowDimensions]);

        let mut input_render_attachments = RenderAttachmentInputDescriptionSet::with_defaults(
            RenderAttachmentQuantitySet::AMBIENT_REFLECTED_LUMINANCE,
        );
        input_render_attachments.insert_description(
            RenderAttachmentQuantity::LinearDepth,
            RenderAttachmentInputDescription::default()
                .with_sampler(RenderAttachmentSampler::Filtering),
        );
        input_render_attachments.insert_description(
            RenderAttachmentQuantity::Occlusion,
            RenderAttachmentInputDescription::default()
                .with_sampler(RenderAttachmentSampler::Filtering),
        );

        let output_render_attachments = RenderAttachmentOutputDescriptionSet::single(
            RenderAttachmentQuantity::Luminance,
            RenderAttachmentOutputDescription::default().with_blending(Blending::Additive),
        );

        Self {
            push_constants,
            input_render_attachments,
            output_render_attachments,
        }
    }
}

impl Default for AmbientOcclusionApplicationShaderTemplate {
    fn default() -> Self {
        Self::new()
    }
}

impl SpecificShaderTemplate for AmbientOcclusionApplicationShaderTemplate {
    fn resolve(&self) -> String {
        TEMPLATE
            .resolve(
                [],
                template_replacements!(
                    "linear_depth_texture_group" => 0,
                    "linear_depth_texture_binding" => LinearDepth.texture_binding(),
                    "linear_depth_sampler_binding" => LinearDepth.sampler_binding(),
                    "ambient_reflected_luminance_texture_group" => 1,
                    "ambient_reflected_luminance_texture_binding" => AmbientReflectedLuminance.texture_binding(),
                    "ambient_reflected_luminance_sampler_binding" => AmbientReflectedLuminance.sampler_binding(),
                    "occlusion_texture_group" => 2,
                    "occlusion_texture_binding" => Occlusion.texture_binding(),
                    "occlusion_sampler_binding" => Occlusion.sampler_binding(),
                    "position_location" => MeshVertexAttributeLocation::Position as u32,
                ),
            )
            .expect("Shader template resolution failed")
    }
}

impl PostprocessingShaderTemplate for AmbientOcclusionApplicationShaderTemplate {
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
        Some((wgpu::CompareFunction::Equal, StencilValue::PhysicalModel))
    }
}

#[cfg(test)]
mod test {
    use super::super::test::validate_template;
    use super::*;

    #[test]
    fn should_resolve_to_valid_wgsl() {
        validate_template(&AmbientOcclusionApplicationShaderTemplate::new());
    }
}
