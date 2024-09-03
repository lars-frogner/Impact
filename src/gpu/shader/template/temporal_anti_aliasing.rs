//! Shader template for the temporal anti-aliasing blending pass.

use crate::{
    gpu::{
        push_constant::{PushConstantGroup, PushConstantVariant},
        rendering::render_command::StencilValue,
        resource_group::GPUResourceGroupID,
        shader::{
            template::{PostprocessingShaderTemplate, ShaderTemplate, SpecificShaderTemplate},
            ShaderID,
        },
        texture::attachment::{
            RenderAttachmentDescription, RenderAttachmentInputDescription,
            RenderAttachmentInputDescriptionSet, RenderAttachmentOutputDescriptionSet,
            RenderAttachmentQuantity::{
                LinearDepth, LuminanceAux, MotionVector, PreviousLuminanceHistory,
            },
            RenderAttachmentQuantitySet, RenderAttachmentSampler,
        },
    },
    mesh::buffer::MeshVertexAttributeLocation,
    rendering_template_source, template_replacements,
};
use std::sync::LazyLock;

/// Shader template for the temporal anti-aliasing blending pass, which blends
/// the previous luminance history with the current luminance and writes the
/// result to the luminance history attachment.
#[derive(Clone, Debug)]
pub struct TemporalAntiAliasingShaderTemplate {
    params_resource_group_id: GPUResourceGroupID,
    push_constants: PushConstantGroup,
    input_render_attachments: RenderAttachmentInputDescriptionSet,
    output_render_attachments: RenderAttachmentOutputDescriptionSet,
}

static TEMPLATE: LazyLock<ShaderTemplate<'static>> = LazyLock::new(|| {
    ShaderTemplate::new(rendering_template_source!("temporal_anti_aliasing")).unwrap()
});

impl TemporalAntiAliasingShaderTemplate {
    /// Creates a new temporal anti-aliasing shader template using the given
    /// resource group ID for the temporal anti-aliasing parameters.
    pub fn new(params_resource_group_id: GPUResourceGroupID) -> Self {
        let push_constants =
            PushConstantGroup::for_fragment([PushConstantVariant::InverseWindowDimensions]);

        let input_render_attachments = RenderAttachmentInputDescriptionSet::new(vec![
            RenderAttachmentInputDescription::default_for(LinearDepth),
            RenderAttachmentInputDescription::default_for(MotionVector),
            // The previous pass (bloom) writes to this attachment
            RenderAttachmentInputDescription::default_for(LuminanceAux),
            RenderAttachmentInputDescription::default_for(PreviousLuminanceHistory)
                .with_sampler(RenderAttachmentSampler::Filtering),
        ]);

        let output_render_attachments = RenderAttachmentOutputDescriptionSet::with_defaults(
            RenderAttachmentQuantitySet::LUMINANCE_HISTORY,
        );

        Self {
            params_resource_group_id,
            push_constants,
            input_render_attachments,
            output_render_attachments,
        }
    }
}

impl SpecificShaderTemplate for TemporalAntiAliasingShaderTemplate {
    fn resolve(&self) -> String {
        TEMPLATE
            .resolve(
                [],
                template_replacements!(
                    "linear_depth_texture_group" => 0,
                    "linear_depth_texture_binding" => LinearDepth.texture_binding(),
                    "linear_depth_sampler_binding" => LinearDepth.sampler_binding(),
                    "motion_vector_texture_group" => 1,
                    "motion_vector_texture_binding" => MotionVector.texture_binding(),
                    "motion_vector_sampler_binding" => MotionVector.sampler_binding(),
                    "luminance_texture_group" => 2,
                    "luminance_texture_binding" => LuminanceAux.texture_binding(),
                    "luminance_sampler_binding" => LuminanceAux.sampler_binding(),
                    "previous_luminance_texture_group" => 3,
                    "previous_luminance_texture_binding" => PreviousLuminanceHistory.texture_binding(),
                    "previous_luminance_sampler_binding" => PreviousLuminanceHistory.sampler_binding(),
                    "params_group" => 4,
                    "params_binding" => 0,
                    "position_location" => MeshVertexAttributeLocation::Position as u32,
                ),
            )
            .expect("Shader template resolution failed")
    }

    fn shader_id(&self) -> ShaderID {
        ShaderID::from_identifier("TemporalAntiAliasingShaderTemplate")
    }
}

impl PostprocessingShaderTemplate for TemporalAntiAliasingShaderTemplate {
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
        Some(self.params_resource_group_id)
    }

    fn stencil_test(&self) -> Option<(wgpu::CompareFunction, StencilValue)> {
        Some((wgpu::CompareFunction::NotEqual, StencilValue::Background))
    }
}

#[cfg(test)]
mod test {
    use super::super::test::validate_template;
    use super::*;
    use impact_utils::hash64;

    #[test]
    fn should_resolve_to_valid_wgsl() {
        validate_template(&TemporalAntiAliasingShaderTemplate::new(
            GPUResourceGroupID(hash64!("test".to_string())),
        ));
    }
}
