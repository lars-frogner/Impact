//! Shader template for voxel chunk culling compute passes.

use crate::{
    compute_template_source,
    gpu::rendering::push_constant::RenderingPushConstantVariant,
    voxel::resource::{VoxelPushConstantGroup, VoxelPushConstantVariant},
};
use impact_gpu::{
    shader::template::{ShaderTemplate, SpecificShaderTemplate},
    shader_template_replacements,
};
use std::sync::LazyLock;

/// Shader template for a voxel chunk culling compute pass, which determines
/// which of the chunks in a voxel object lie outside a frustum and updates an
/// indirect draw parameter buffer so that those chunks will be excluded in a
/// subsequent indirect draw call.
#[derive(Clone, Debug)]
pub struct VoxelChunkCullingShaderTemplate {
    /// Whether the draw call arguments to update are for indexed draw calls.
    pub for_indexed_draw_calls: bool,
}

static TEMPLATE: LazyLock<ShaderTemplate<'static>> =
    LazyLock::new(|| ShaderTemplate::new(compute_template_source!("voxel_chunk_culling")).unwrap());

impl VoxelChunkCullingShaderTemplate {
    /// Returns the workgroup size for the shader (the workgroup is 1D).
    pub const fn workgroup_size() -> u32 {
        16
    }

    /// Returns the workgroup counts to use when invoking the compute shader
    /// based on the given number of chunks in the chunk buffer.
    pub const fn determine_workgroup_counts(chunk_count: u32) -> [u32; 3] {
        [chunk_count.div_ceil(Self::workgroup_size()), 1, 1]
    }

    /// Returns the group of push constants used by the shader.
    pub fn push_constants() -> VoxelPushConstantGroup {
        VoxelPushConstantGroup::for_compute([
            VoxelPushConstantVariant::CullingFrustum,
            VoxelPushConstantVariant::ChunkCount,
            VoxelPushConstantVariant::Rendering(RenderingPushConstantVariant::InstanceIdx),
        ])
    }
}

impl SpecificShaderTemplate for VoxelChunkCullingShaderTemplate {
    fn resolve(&self) -> String {
        let flags_to_set: &[&str] = if self.for_indexed_draw_calls {
            &["for_indexed_draw_calls"] as _
        } else {
            &[]
        };
        TEMPLATE
            .resolve(
                flags_to_set.iter().copied(),
                shader_template_replacements!(
                    "chunk_submesh_group" => 0,
                    "chunk_submesh_binding" => 0,
                    "indirect_draw_group" => 0,
                    "indirect_draw_binding" => 1,
                    "workgroup_size" => Self::workgroup_size(),
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
        validate_template(&VoxelChunkCullingShaderTemplate {
            for_indexed_draw_calls: false,
        });
    }
}
