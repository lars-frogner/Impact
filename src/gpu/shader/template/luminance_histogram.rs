//! Shader template for the luminance histogram computation pass.

use crate::{
    compute_template_source,
    gpu::{
        push_constant::{PushConstantGroup, PushConstantVariant},
        rendering::surface::RenderingSurface,
        resource_group::GPUResourceGroupID,
        shader::template::{ComputeShaderTemplate, ShaderTemplate, SpecificShaderTemplate},
        texture::attachment::{
            RenderAttachmentDescription, RenderAttachmentInputDescription,
            RenderAttachmentInputDescriptionSet, RenderAttachmentQuantity,
        },
    },
    template_replacements,
};
use std::sync::LazyLock;

/// Shader template for the luminance histogram computation pass, which
/// computes the histogram of the luminances in the luminance attachment and
/// writes it to a storage buffer.
#[derive(Clone, Debug)]
pub struct LuminanceHistogramShaderTemplate {
    threads_per_side: usize,
    gpu_resource_group_id: GPUResourceGroupID,
    push_constants: PushConstantGroup,
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

        let push_constants = PushConstantGroup::for_compute([PushConstantVariant::InverseExposure]);

        let input_render_attachments = RenderAttachmentInputDescriptionSet::single(
            // The previous pass (bloom) writes to this attachment
            RenderAttachmentInputDescription::default_for(RenderAttachmentQuantity::LuminanceAux)
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
                template_replacements!(
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
}

impl ComputeShaderTemplate for LuminanceHistogramShaderTemplate {
    fn push_constants(&self) -> PushConstantGroup {
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
mod test {
    use super::super::test::validate_template;
    use super::*;
    use impact_utils::hash64;

    #[test]
    fn should_resolve_to_valid_wgsl() {
        validate_template(&LuminanceHistogramShaderTemplate::new(
            4,
            GPUResourceGroupID(hash64!("test".to_string())),
        ));
    }
}
