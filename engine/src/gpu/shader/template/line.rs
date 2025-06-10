//! Shader template for rendering lines.

use crate::{
    camera::buffer::CameraProjectionUniform,
    gpu::shader::template::{ShaderTemplate, SpecificShaderTemplate},
    mesh::{VertexAttributeSet, buffer::LineSegmentMeshVertexAttributeLocation},
    model::transform::InstanceModelViewTransform,
    rendering_template_source, template_replacements,
};
use std::sync::LazyLock;

/// Shader template for rendering colored lines in 3D.
#[derive(Clone, Copy, Debug)]
pub struct LineShaderTemplate;

static TEMPLATE: LazyLock<ShaderTemplate<'static>> =
    LazyLock::new(|| ShaderTemplate::new(rendering_template_source!("line")).unwrap());

impl LineShaderTemplate {
    /// Returns the set of vertex attributes used by the shader.
    pub fn vertex_attributes() -> VertexAttributeSet {
        VertexAttributeSet::POSITION | VertexAttributeSet::COLOR
    }
}

impl SpecificShaderTemplate for LineShaderTemplate {
    fn resolve(&self) -> String {
        TEMPLATE
            .resolve(
                [],
                template_replacements!(
                    "model_view_transform_rotation_location" => InstanceModelViewTransform::rotation_location(),
                    "model_view_transform_translation_location" => InstanceModelViewTransform::translation_and_scaling_location(),
                    "projection_uniform_group" => 0,
                    "projection_uniform_binding" => CameraProjectionUniform::binding(),
                    "position_location" => LineSegmentMeshVertexAttributeLocation::Position as u32,
                    "color_location" => LineSegmentMeshVertexAttributeLocation::Color as u32,
                )
            )
            .expect("Shader template resolution failed")
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::validate_template;
    use super::*;

    #[test]
    fn should_resolve_to_valid_wgsl() {
        validate_template(&LineShaderTemplate);
    }
}
