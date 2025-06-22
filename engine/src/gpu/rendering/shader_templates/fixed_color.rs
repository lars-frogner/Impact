//! Shader template for rendering geometry with fixed vertex colors.

use crate::{
    mesh::{VertexAttributeSet, buffer::MeshVertexAttributeLocation},
    model::transform::InstanceModelViewTransform,
    rendering_template_source,
};
use impact_camera::buffer::CameraProjectionUniform;
use impact_gpu::{
    shader::template::{ShaderTemplate, SpecificShaderTemplate},
    shader_template_replacements,
};
use std::sync::LazyLock;

/// Shader template for rendering geometry with fixed vertex colors.
#[derive(Clone, Copy, Debug)]
pub struct FixedColorShaderTemplate;

static TEMPLATE: LazyLock<ShaderTemplate<'static>> =
    LazyLock::new(|| ShaderTemplate::new(rendering_template_source!("fixed_color")).unwrap());

impl FixedColorShaderTemplate {
    /// Returns the set of vertex attributes used by the shader.
    pub fn vertex_attributes() -> VertexAttributeSet {
        VertexAttributeSet::POSITION | VertexAttributeSet::COLOR
    }
}

impl SpecificShaderTemplate for FixedColorShaderTemplate {
    fn resolve(&self) -> String {
        TEMPLATE
            .resolve(
                [],
                shader_template_replacements!(
                    "model_view_transform_rotation_location" => InstanceModelViewTransform::rotation_location(),
                    "model_view_transform_translation_location" => InstanceModelViewTransform::translation_and_scaling_location(),
                    "projection_uniform_group" => 0,
                    "projection_uniform_binding" => CameraProjectionUniform::binding(),
                    "position_location" => MeshVertexAttributeLocation::Position as u32,
                    "color_location" => MeshVertexAttributeLocation::Color as u32,
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
        validate_template(&FixedColorShaderTemplate);
    }
}
