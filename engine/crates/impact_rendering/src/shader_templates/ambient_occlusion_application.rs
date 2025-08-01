//! Shader template for the ambient occlusion application pass.

use crate::{
    attachment::{
        Blending, RenderAttachmentDescription, RenderAttachmentInputDescription,
        RenderAttachmentInputDescriptionSet, RenderAttachmentOutputDescription,
        RenderAttachmentOutputDescriptionSet,
        RenderAttachmentQuantity::{LinearDepth, Luminance, LuminanceAux, Occlusion},
        RenderAttachmentSampler,
    },
    postprocessing::PostprocessingShaderTemplate,
    push_constant::{BasicPushConstantGroup, BasicPushConstantVariant},
    render_command::StencilValue,
    rendering_template_source,
};
use impact_gpu::{
    shader::{
        ShaderID,
        template::{ShaderTemplate, SpecificShaderTemplate},
    },
    shader_template_replacements, wgpu,
};
use impact_mesh::gpu_resource::MeshVertexAttributeLocation;
use std::sync::LazyLock;

/// Shader template for the ambient occlusion application pass, which uses the
/// occlusion computed by the ambient occlusion computation pass to compute the
/// occluded ambient reflected luminance and adds it to the luminance
/// attachment.
#[derive(Clone, Debug)]
pub struct AmbientOcclusionApplicationShaderTemplate {
    push_constants: BasicPushConstantGroup,
    input_render_attachments: RenderAttachmentInputDescriptionSet,
    output_render_attachments: RenderAttachmentOutputDescriptionSet,
}

static TEMPLATE: LazyLock<ShaderTemplate<'static>> = LazyLock::new(|| {
    ShaderTemplate::new(rendering_template_source!("ambient_occlusion_application")).unwrap()
});

impl AmbientOcclusionApplicationShaderTemplate {
    /// Creates a new ambient occlusion application shader template.
    pub fn new() -> Self {
        let push_constants = BasicPushConstantGroup::for_fragment([
            BasicPushConstantVariant::InverseWindowDimensions,
        ]);

        let input_render_attachments = RenderAttachmentInputDescriptionSet::new(vec![
            RenderAttachmentInputDescription::default_for(LinearDepth)
                .with_sampler(RenderAttachmentSampler::Filtering),
            RenderAttachmentInputDescription::default_for(LuminanceAux),
            RenderAttachmentInputDescription::default_for(Occlusion)
                .with_sampler(RenderAttachmentSampler::Filtering),
        ]);

        let output_render_attachments = RenderAttachmentOutputDescriptionSet::single(
            RenderAttachmentOutputDescription::default_for(Luminance)
                .with_blending(Blending::Additive),
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
                shader_template_replacements!(
                    "linear_depth_texture_group" => 0,
                    "linear_depth_texture_binding" => LinearDepth.texture_binding(),
                    "linear_depth_sampler_binding" => LinearDepth.sampler_binding(),
                    "ambient_reflected_luminance_texture_group" => 1,
                    "ambient_reflected_luminance_texture_binding" => LuminanceAux.texture_binding(),
                    "ambient_reflected_luminance_sampler_binding" => LuminanceAux.sampler_binding(),
                    "occlusion_texture_group" => 2,
                    "occlusion_texture_binding" => Occlusion.texture_binding(),
                    "occlusion_sampler_binding" => Occlusion.sampler_binding(),
                    "position_location" => MeshVertexAttributeLocation::Position as u32,
                ),
            )
            .expect("Shader template resolution failed")
    }

    fn shader_id(&self) -> ShaderID {
        ShaderID::from_identifier("AmbientOcclusionApplicationShaderTemplate")
    }
}

impl PostprocessingShaderTemplate for AmbientOcclusionApplicationShaderTemplate {
    fn push_constants(&self) -> BasicPushConstantGroup {
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
mod tests {

    use super::*;
    use impact_gpu::shader::template::validate_template;

    #[test]
    fn should_resolve_to_valid_wgsl() {
        validate_template(&AmbientOcclusionApplicationShaderTemplate::new());
    }
}
