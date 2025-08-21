//! Shader template for the skybox rendering pass.

use crate::rendering_template_source;
use crate::{
    attachment::{
        Blending, RenderAttachmentDescription, RenderAttachmentOutputDescription,
        RenderAttachmentOutputDescriptionSet, RenderAttachmentQuantity,
    },
    push_constant::{BasicPushConstantGroup, BasicPushConstantVariant},
};
use impact_camera::gpu_resource::CameraProjectionUniform;
use impact_gpu::{
    shader::template::{ShaderTemplate, SpecificShaderTemplate},
    shader_template_replacements,
};
use impact_mesh::{VertexAttributeSet, gpu_resource::MeshVertexAttributeLocation};
use impact_scene::skybox::gpu_resource::SkyboxGPUResource;
use std::sync::LazyLock;

/// Shader template for the skybox rendering pass, which writes the emitted
/// luminance sampled from a skybox cubemap to the appropriate parts of the
/// luminance attachment.
#[derive(Clone, Debug)]
pub struct SkyboxShaderTemplate;

static TEMPLATE: LazyLock<ShaderTemplate<'static>> =
    LazyLock::new(|| ShaderTemplate::new(rendering_template_source!("skybox")).unwrap());

impl SkyboxShaderTemplate {
    /// Returns the group of push constants used by the shader.
    pub fn push_constants() -> BasicPushConstantGroup {
        BasicPushConstantGroup::for_vertex_fragment([
            BasicPushConstantVariant::CameraRotationQuaternion,
            BasicPushConstantVariant::Exposure,
        ])
    }

    /// Returns the set of vertex attributes used by the shader.
    pub fn vertex_attributes() -> VertexAttributeSet {
        VertexAttributeSet::POSITION
    }

    /// Returns the render attachment quantity that the shader will write to.
    pub fn output_render_attachment_quantity() -> RenderAttachmentQuantity {
        RenderAttachmentQuantity::Luminance
    }

    /// Returns the descriptions of the render attachments that the shader will
    /// write to.
    pub fn output_render_attachments() -> RenderAttachmentOutputDescriptionSet {
        RenderAttachmentOutputDescriptionSet::single(
            RenderAttachmentOutputDescription::default_for(
                Self::output_render_attachment_quantity(),
            )
            .with_blending(Blending::Additive),
        )
    }
}

impl SpecificShaderTemplate for SkyboxShaderTemplate {
    fn resolve(&self) -> String {
        TEMPLATE
            .resolve(
                &[],
                shader_template_replacements!(
                    "projection_uniform_group" => 0,
                    "projection_uniform_binding" => CameraProjectionUniform::binding(),
                    "skybox_properties_group" => 1,
                    "skybox_properties_binding" => SkyboxGPUResource::properties_uniform_binding(),
                    "skybox_texture_group" => 1,
                    "skybox_texture_binding" => SkyboxGPUResource::texture_binding(),
                    "skybox_sampler_binding" => SkyboxGPUResource::sampler_binding(),
                    "position_location" => MeshVertexAttributeLocation::Position as u32,
                ),
            )
            .expect("Shader template resolution failed")
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use impact_gpu::shader::template::validate_template;

    #[test]
    fn should_resolve_to_valid_wgsl() {
        validate_template(&SkyboxShaderTemplate);
    }
}
