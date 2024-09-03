//! Shader template for the voxel geometry pass.

use crate::{
    camera::buffer::CameraProjectionUniform,
    gpu::{
        push_constant::{PushConstantGroup, PushConstantVariant},
        shader::template::{ShaderTemplate, SpecificShaderTemplate},
    },
    model::transform::InstanceModelViewTransformWithPrevious,
    rendering_template_source, template_replacements,
    voxel::buffer::VoxelMeshVertexAttributeLocation,
};
use std::sync::LazyLock;

/// Shader template for the voxel geometry pass, which extracts the relevant
/// geometrical information and material properties from the visible voxel
/// chunks and writes them to the corresponding render attachments (the
/// G-buffer).
#[derive(Clone, Copy, Debug)]
pub struct VoxelGeometryShaderTemplate;

static TEMPLATE: LazyLock<ShaderTemplate<'static>> =
    LazyLock::new(|| ShaderTemplate::new(rendering_template_source!("voxel_geometry")).unwrap());

impl VoxelGeometryShaderTemplate {
    /// Returns the group of push constants used by the shader.
    pub fn push_constants() -> PushConstantGroup {
        PushConstantGroup::for_vertex_fragment([
            PushConstantVariant::InverseWindowDimensions,
            PushConstantVariant::FrameCounter,
            PushConstantVariant::Exposure,
        ])
    }
}

impl SpecificShaderTemplate for VoxelGeometryShaderTemplate {
    fn resolve(&self) -> String {
        TEMPLATE
            .resolve(
                [],
                template_replacements!(
                    "jitter_count" => CameraProjectionUniform::jitter_count(),
                    "model_view_transform_rotation_location" => InstanceModelViewTransformWithPrevious::current_rotation_location(),
                    "model_view_transform_translation_location" => InstanceModelViewTransformWithPrevious::current_translation_and_scaling_location(),
                    "previous_model_view_transform_rotation_location" => InstanceModelViewTransformWithPrevious::previous_rotation_location(),
                    "previous_model_view_transform_translation_location" => InstanceModelViewTransformWithPrevious::previous_translation_and_scaling_location(),
                    "projection_uniform_group" => 0,
                    "projection_uniform_binding" => CameraProjectionUniform::binding(),
                    "position_location" => VoxelMeshVertexAttributeLocation::Position as u32,
                    "normal_vector_location" => VoxelMeshVertexAttributeLocation::NormalVector as u32,
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
        validate_template(&VoxelGeometryShaderTemplate);
    }
}
