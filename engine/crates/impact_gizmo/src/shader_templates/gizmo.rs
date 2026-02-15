//! Shader template for rendering basic gizmos.

use crate::{model::GizmoInstanceModelViewTransform, rendering_template_source};
use impact_camera::gpu_resource::CameraProjectionUniform;
use impact_gpu::{
    shader::template::{ShaderTemplate, SpecificShaderTemplate},
    shader_template_replacements,
};
use impact_mesh::{VertexAttributeSet, gpu_resource::MeshVertexAttributeLocation};
use std::sync::LazyLock;

/// Shader template for rendering basic gizmos.
#[derive(Clone, Copy, Debug)]
pub struct GizmoShaderTemplate;

static TEMPLATE: LazyLock<ShaderTemplate<'static>> =
    LazyLock::new(|| ShaderTemplate::new(rendering_template_source!("gizmo")).unwrap());

impl GizmoShaderTemplate {
    /// Returns the set of vertex attributes used by the shader.
    pub fn vertex_attributes() -> VertexAttributeSet {
        VertexAttributeSet::POSITION | VertexAttributeSet::COLOR
    }
}

impl SpecificShaderTemplate for GizmoShaderTemplate {
    fn resolve(&self) -> String {
        TEMPLATE
            .resolve(
                &[],
                shader_template_replacements!(
                    "model_view_transform_rotation_location" => GizmoInstanceModelViewTransform::rotation_location(),
                    "model_view_transform_translation_location" => GizmoInstanceModelViewTransform::translation_location(),
                    "model_view_transform_scaling_location" => GizmoInstanceModelViewTransform::scaling_location(),
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
        validate_template(&GizmoShaderTemplate);
    }
}
