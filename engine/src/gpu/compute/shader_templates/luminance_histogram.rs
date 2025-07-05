//! Shader template for the luminance histogram computation pass.

use crate::{
    compute_template_source,
    gpu::{
        compute::ComputeShaderTemplate,
        rendering::{
            attachment::{
                RenderAttachmentDescription, RenderAttachmentInputDescription,
                RenderAttachmentInputDescriptionSet, RenderAttachmentQuantity,
            },
            push_constant::{BasicPushConstantGroup, BasicPushConstantVariant},
            surface::RenderingSurface,
        },
    },
};
use impact_gpu::{
    resource_group::GPUResourceGroupID,
    shader::{
        ShaderID,
        template::{ShaderTemplate, SpecificShaderTemplate},
    },
    shader_template_replacements,
};
use std::sync::LazyLock;

/// Shader template for the luminance histogram computation pass, which
/// computes the histogram of the luminances in the luminance attachment and
/// writes it to a storage buffer.
#[derive(Clone, Debug)]
pub struct LuminanceHistogramShaderTemplate {
    threads_per_side: usize,
    gpu_resource_group_id: GPUResourceGroupID,
    push_constants: BasicPushConstantGroup,
    input_render_attachments: RenderAttachmentInputDescriptionSet,
}

static TEMPLATE: LazyLock<ShaderTemplate<'static>> =
    LazyLock::new(|| ShaderTemplate::new(compute_template_source!("luminance_histogram")).unwrap());

impl LuminanceHistogramShaderTemplate {
    /// Creates a new shader template for the luminance histogram computation
    /// pass using the specified number of threads per side of the quadratic
    /// region of the texture that one workgroup covers, and using the given
    /// resource group for the parameter uniform and histogram buffer.
    pub fn new(log2_threads_per_side: usize, gpu_resource_group_id: GPUResourceGroupID) -> Self {
        let threads_per_side = 1 << log2_threads_per_side;

        let push_constants =
            BasicPushConstantGroup::for_compute([BasicPushConstantVariant::InverseExposure]);

        let input_render_attachments = RenderAttachmentInputDescriptionSet::single(
            RenderAttachmentInputDescription::default_for(RenderAttachmentQuantity::Luminance)
                .with_visibility(wgpu::ShaderStages::COMPUTE),
        );

        Self {
            threads_per_side,
            gpu_resource_group_id,
            push_constants,
            input_render_attachments,
        }
    }

    fn min_workgroups_to_cover_texture_extent(&self, extent: u32) -> u32 {
        (f64::from(extent) / self.threads_per_side as f64).ceil() as u32
    }
}

impl SpecificShaderTemplate for LuminanceHistogramShaderTemplate {
    fn resolve(&self) -> String {
        TEMPLATE
            .resolve(
                [],
                shader_template_replacements!(
                    "threads_per_side" => self.threads_per_side,
                    "texture_group" => 0,
                    "texture_binding" => RenderAttachmentQuantity::Luminance.texture_binding(),
                    "params_group" => 1,
                    "params_binding" => 0,
                    "histogram_group" => 1,
                    "histogram_binding" => 1,
                ),
            )
            .expect("Shader template resolution failed")
    }

    fn shader_id(&self) -> ShaderID {
        ShaderID::from_identifier(&format!(
            "LuminanceHistogramShaderTemplate{{ threads_per_side = {} }}",
            self.threads_per_side
        ))
    }
}

impl ComputeShaderTemplate for LuminanceHistogramShaderTemplate {
    fn push_constants(&self) -> BasicPushConstantGroup {
        self.push_constants.clone()
    }

    fn input_render_attachments(&self) -> RenderAttachmentInputDescriptionSet {
        self.input_render_attachments.clone()
    }

    fn gpu_resource_group_id(&self) -> GPUResourceGroupID {
        self.gpu_resource_group_id
    }

    fn determine_workgroup_counts(&self, rendering_surface: &RenderingSurface) -> [u32; 3] {
        let (width, height) = rendering_surface.surface_dimensions();

        let workgroup_count_across_width =
            self.min_workgroups_to_cover_texture_extent(width.into());
        let workgroup_count_across_height =
            self.min_workgroups_to_cover_texture_extent(height.into());

        [
            workgroup_count_across_width,
            workgroup_count_across_height,
            1,
        ]
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use impact_gpu::shader::template::validate_template;
    use impact_math::hash64;

    #[test]
    fn should_resolve_to_valid_wgsl() {
        validate_template(&LuminanceHistogramShaderTemplate::new(
            4,
            GPUResourceGroupID(hash64!("test".to_string())),
        ));
    }
}
