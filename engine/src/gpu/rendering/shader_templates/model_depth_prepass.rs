//! Shader template for the model depth prepass.

use crate::{
    gpu::rendering::push_constant::{RenderingPushConstantGroup, RenderingPushConstantVariant},
    mesh::{VertexAttributeSet, buffer::MeshVertexAttributeLocation},
    model::transform::InstanceModelViewTransformWithPrevious,
    rendering_template_source,
};
use impact_camera::buffer::CameraProjectionUniform;
use impact_gpu::{
    shader::template::{ShaderTemplate, SpecificShaderTemplate},
    shader_template_replacements,
};
use std::sync::LazyLock;

/// Shader template for the model depth prepass, which writes the depth
/// of the rendered model instances to the depth-stencil attachment.
#[derive(Clone, Copy, Debug)]
pub struct ModelDepthPrepassShaderTemplate;

static TEMPLATE: LazyLock<ShaderTemplate<'static>> = LazyLock::new(|| {
    ShaderTemplate::new(rendering_template_source!("model_depth_prepass")).unwrap()
});

impl ModelDepthPrepassShaderTemplate {
    /// Returns the group of push constants used by the shader.
    pub fn push_constants() -> RenderingPushConstantGroup {
        RenderingPushConstantGroup::for_vertex([
            RenderingPushConstantVariant::InverseWindowDimensions,
            RenderingPushConstantVariant::FrameCounter,
        ])
    }

    /// Returns the set of vertex attributes used by the shader.
    pub fn vertex_attributes() -> VertexAttributeSet {
        VertexAttributeSet::POSITION
    }
}

impl SpecificShaderTemplate for ModelDepthPrepassShaderTemplate {
    fn resolve(&self) -> String {
        TEMPLATE
            .resolve(
                [],
                shader_template_replacements!(
                    "jitter_count" => CameraProjectionUniform::jitter_count(),
                    "model_view_transform_rotation_location" => InstanceModelViewTransformWithPrevious::current_rotation_location(),
                    "model_view_transform_translation_location" => InstanceModelViewTransformWithPrevious::current_translation_and_scaling_location(),
                    "previous_model_view_transform_rotation_location" => InstanceModelViewTransformWithPrevious::previous_rotation_location(),
                    "previous_model_view_transform_translation_location" => InstanceModelViewTransformWithPrevious::previous_translation_and_scaling_location(),
                    "projection_uniform_group" => 0,
                    "projection_uniform_binding" => CameraProjectionUniform::binding(),
                    "position_location" => MeshVertexAttributeLocation::Position as u32,
                )
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
        validate_template(&ModelDepthPrepassShaderTemplate);
    }
}
