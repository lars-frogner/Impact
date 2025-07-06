//! Shader template for the luminance histogram average computation pass.

use crate::{
    attachment::RenderAttachmentInputDescriptionSet,
    compute::ComputeShaderTemplate,
    compute_template_source,
    push_constant::{BasicPushConstantGroup, BasicPushConstantVariant},
};
use impact_gpu::{
    resource_group::GPUResourceGroupID,
    shader::{
        ShaderID,
        template::{ShaderTemplate, SpecificShaderTemplate},
    },
    shader_template_replacements,
};
use std::{num::NonZeroU32, sync::LazyLock};

/// Shader template for the luminance histogram average computation pass, which
/// computes the weighted average of the luminance histogram in the storage
/// buffer computed by the
/// [`LuminanceHistogramShaderTemplate`](super::luminance_histogram::LuminanceHistogramShaderTemplate)
/// and writes the result to a storage buffer that can be mapped for reading by
/// the CPU.
#[derive(Clone, Debug)]
pub struct LuminanceHistogramAverageShaderTemplate {
    bin_count: usize,
    gpu_resource_group_id: GPUResourceGroupID,
    push_constants: BasicPushConstantGroup,
}

static TEMPLATE: LazyLock<ShaderTemplate<'static>> = LazyLock::new(|| {
    ShaderTemplate::new(compute_template_source!("luminance_histogram_average")).unwrap()
});

impl LuminanceHistogramAverageShaderTemplate {
    /// Creates a new shader template for the luminance histogram average
    /// computation pass for the given number of histogram bins and using the
    /// given resource group for the parameter uniform, histogram buffer and
    /// result buffer.
    pub fn new(bin_count: usize, gpu_resource_group_id: GPUResourceGroupID) -> Self {
        let push_constants =
            BasicPushConstantGroup::for_compute([BasicPushConstantVariant::PixelCount]);
        Self {
            bin_count,
            gpu_resource_group_id,
            push_constants,
        }
    }
}

impl SpecificShaderTemplate for LuminanceHistogramAverageShaderTemplate {
    fn resolve(&self) -> String {
        TEMPLATE
            .resolve(
                [],
                shader_template_replacements!(
                    "bin_count" => self.bin_count,
                    "params_group" => 0,
                    "params_binding" => 0,
                    "histogram_group" => 0,
                    "histogram_binding" => 1,
                    "average_group" => 0,
                    "average_binding" => 2,
                ),
            )
            .expect("Shader template resolution failed")
    }

    fn shader_id(&self) -> ShaderID {
        ShaderID::from_identifier(&format!(
            "LuminanceHistogramShaderTemplate{{ bin_count = {} }}",
            self.bin_count
        ))
    }
}

impl ComputeShaderTemplate for LuminanceHistogramAverageShaderTemplate {
    fn push_constants(&self) -> BasicPushConstantGroup {
        self.push_constants.clone()
    }

    fn input_render_attachments(&self) -> RenderAttachmentInputDescriptionSet {
        RenderAttachmentInputDescriptionSet::empty()
    }

    fn gpu_resource_group_id(&self) -> GPUResourceGroupID {
        self.gpu_resource_group_id
    }

    fn determine_workgroup_counts(
        &self,
        _surface_width: NonZeroU32,
        _surface_height: NonZeroU32,
    ) -> [u32; 3] {
        [1; 3]
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use impact_gpu::shader::template::validate_template;
    use impact_math::hash64;

    #[test]
    fn should_resolve_to_valid_wgsl() {
        validate_template(&LuminanceHistogramAverageShaderTemplate::new(
            256,
            GPUResourceGroupID(hash64!("test".to_string())),
        ));
    }
}
