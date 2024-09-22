//! Shader template for the voxel geometry pass.

use crate::{
    camera::buffer::CameraProjectionUniform,
    gpu::{
        push_constant::{PushConstantGroup, PushConstantVariant},
        shader::template::{ShaderTemplate, SpecificShaderTemplate},
    },
    model::transform::InstanceModelViewTransformWithPrevious,
    rendering_template_source, template_replacements,
    voxel::resource::{VoxelMaterialGPUResourceManager, VoxelMeshVertexAttributeLocation},
};
use std::sync::LazyLock;

/// Shader template for the voxel geometry pass, which extracts the relevant
/// geometrical information and material properties from the visible voxel
/// chunks and writes them to the corresponding render attachments (the
/// G-buffer).
#[derive(Clone, Copy, Debug)]
pub struct VoxelGeometryShaderTemplate {
    n_voxel_types: usize,
    texture_frequency: f64,
}

static TEMPLATE: LazyLock<ShaderTemplate<'static>> =
    LazyLock::new(|| ShaderTemplate::new(rendering_template_source!("voxel_geometry")).unwrap());

impl VoxelGeometryShaderTemplate {
    /// Creates a new voxel geometry shader template for the given number of
    /// registered voxel types and the given frequency factor determining the
    /// spatial extent of features in the textures.
    pub fn new(n_voxel_types: usize, texture_frequency: f64) -> Self {
        assert!(n_voxel_types > 0);
        Self {
            n_voxel_types,
            texture_frequency,
        }
    }

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
                    "texture_frequency" => self.texture_frequency,
                    "voxel_type_count" => self.n_voxel_types,
                    "model_view_transform_rotation_location" => InstanceModelViewTransformWithPrevious::current_rotation_location(),
                    "model_view_transform_translation_location" => InstanceModelViewTransformWithPrevious::current_translation_and_scaling_location(),
                    "previous_model_view_transform_rotation_location" => InstanceModelViewTransformWithPrevious::previous_rotation_location(),
                    "previous_model_view_transform_translation_location" => InstanceModelViewTransformWithPrevious::previous_translation_and_scaling_location(),
                    "projection_uniform_group" => 0,
                    "projection_uniform_binding" => CameraProjectionUniform::binding(),
                    "material_group" => 1,
                    "fixed_material_uniform_binding" => VoxelMaterialGPUResourceManager::fixed_properties_binding(),
                    "color_texture_array_binding" => VoxelMaterialGPUResourceManager::color_texture_array_binding(),
                    "sampler_binding" => VoxelMaterialGPUResourceManager::sampler_binding(),
                    "position_and_normal_group" => 2,
                    "position_buffer_binding" => 0,
                    "normal_buffer_binding" => 1,
                    "index_location" => VoxelMeshVertexAttributeLocation::Indices as u32,
                    "material_indices_location" => VoxelMeshVertexAttributeLocation::MaterialIndices as u32,
                    "material_weights_location" => VoxelMeshVertexAttributeLocation::MaterialWeights as u32,
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
        validate_template(&VoxelGeometryShaderTemplate::new(5, 1.0));
    }
}
