//! Shader template for the ambient occlusion computation pass.

use crate::{
    camera::buffer::CameraProjectionUniform,
    gpu::{
        push_constant::{PushConstantGroup, PushConstantVariant},
        rendering::{
            postprocessing::ambient_occlusion::MAX_AMBIENT_OCCLUSION_SAMPLE_COUNT,
            render_command::StencilValue,
        },
        resource_group::GPUResourceGroupID,
        shader::{
            template::{PostprocessingShaderTemplate, ShaderTemplate, SpecificShaderTemplate},
            ShaderID,
        },
        texture::attachment::{
            RenderAttachmentDescription, RenderAttachmentInputDescription,
            RenderAttachmentInputDescriptionSet, RenderAttachmentOutputDescriptionSet,
            RenderAttachmentQuantity::{LinearDepth, NormalVector},
            RenderAttachmentQuantitySet, RenderAttachmentSampler,
        },
    },
    mesh::buffer::MeshVertexAttributeLocation,
    rendering_template_source, template_replacements,
};
use std::sync::LazyLock;

/// Shader template for the ambient occlusion computation pass, which computes
/// the occlusion of ambient light due to nearby geometry and writes it to the
/// occlusion attachment.
#[derive(Clone, Debug)]
pub struct AmbientOcclusionComputationShaderTemplate {
    samples_resource_group_id: GPUResourceGroupID,
    push_constants: PushConstantGroup,
    input_render_attachments: RenderAttachmentInputDescriptionSet,
    output_render_attachments: RenderAttachmentOutputDescriptionSet,
}

static TEMPLATE: LazyLock<ShaderTemplate<'static>> = LazyLock::new(|| {
    ShaderTemplate::new(rendering_template_source!("ambient_occlusion_computation")).unwrap()
});

impl AmbientOcclusionComputationShaderTemplate {
    /// Creates a new ambient occlusion computation shader template using the
    /// given resource group for the ambient occlusion sample uniform.
    pub fn new(samples_resource_group_id: GPUResourceGroupID) -> Self {
        let push_constants = PushConstantGroup::for_fragment([
            PushConstantVariant::InverseWindowDimensions,
            PushConstantVariant::FrameCounter,
        ]);

        let input_render_attachments = RenderAttachmentInputDescriptionSet::new(vec![
            RenderAttachmentInputDescription::default_for(LinearDepth)
                .with_sampler(RenderAttachmentSampler::Filtering),
            RenderAttachmentInputDescription::default_for(NormalVector),
        ]);

        let output_render_attachments = RenderAttachmentOutputDescriptionSet::with_defaults(
            RenderAttachmentQuantitySet::OCCLUSION,
        );

        Self {
            samples_resource_group_id,
            push_constants,
            input_render_attachments,
            output_render_attachments,
        }
    }
}

impl SpecificShaderTemplate for AmbientOcclusionComputationShaderTemplate {
    fn resolve(&self) -> String {
        TEMPLATE
            .resolve(
                [],
                template_replacements!(
                    "max_samples" => MAX_AMBIENT_OCCLUSION_SAMPLE_COUNT,
                    "projection_uniform_group" => 0,
                    "projection_uniform_binding" => CameraProjectionUniform::binding(),
                    "linear_depth_texture_group" => 1,
                    "linear_depth_texture_binding" => LinearDepth.texture_binding(),
                    "linear_depth_sampler_binding" => LinearDepth.sampler_binding(),
                    "normal_vector_texture_group" => 2,
                    "normal_vector_texture_binding" => NormalVector.texture_binding(),
                    "normal_vector_sampler_binding" => NormalVector.sampler_binding(),
                    "samples_group" => 3,
                    "samples_binding" => 0,
                    "position_location" => MeshVertexAttributeLocation::Position as u32,
                ),
            )
            .expect("Shader template resolution failed")
    }

    fn shader_id(&self) -> ShaderID {
        ShaderID::from_identifier("AmbientOcclusionComputationShaderTemplate")
    }
}

impl PostprocessingShaderTemplate for AmbientOcclusionComputationShaderTemplate {
    fn push_constants(&self) -> PushConstantGroup {
        self.push_constants.clone()
    }

    fn input_render_attachments(&self) -> RenderAttachmentInputDescriptionSet {
        self.input_render_attachments.clone()
    }

    fn output_render_attachments(&self) -> RenderAttachmentOutputDescriptionSet {
        self.output_render_attachments.clone()
    }

    fn uses_camera(&self) -> bool {
        true
    }

    fn gpu_resource_group_id(&self) -> Option<GPUResourceGroupID> {
        Some(self.samples_resource_group_id)
    }

    fn stencil_test(&self) -> Option<(wgpu::CompareFunction, StencilValue)> {
        Some((wgpu::CompareFunction::Equal, StencilValue::PhysicalModel))
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::validate_template;
    use super::*;
    use impact_utils::hash64;

    #[test]
    fn should_resolve_to_valid_wgsl() {
        validate_template(&AmbientOcclusionComputationShaderTemplate::new(
            GPUResourceGroupID(hash64!("test".to_string())),
        ));
    }
}
