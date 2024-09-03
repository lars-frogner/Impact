//! Shader template for the luminance histogram average computation pass.

use crate::{
    compute_template_source,
    gpu::{
        push_constant::{PushConstantGroup, PushConstantVariant},
        rendering::surface::RenderingSurface,
        resource_group::GPUResourceGroupID,
        shader::{
            template::{ComputeShaderTemplate, ShaderTemplate, SpecificShaderTemplate},
            ShaderID,
        },
        texture::attachment::RenderAttachmentInputDescriptionSet,
    },
    template_replacements,
};
use std::sync::LazyLock;

/// Shader template for the luminance histogram average computation pass, which
/// computes the weighted average of the luminance histogram in the storage
/// buffer computed by the [`LuminanceHistogramShaderTemplate`] and writes the
/// result to a storage buffer that can be mapped for reading by the CPU.
#[derive(Clone, Debug)]
pub struct LuminanceHistogramAverageShaderTemplate {
    bin_count: usize,
    gpu_resource_group_id: GPUResourceGroupID,
    push_constants: PushConstantGroup,
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
        let push_constants = PushConstantGroup::for_compute([PushConstantVariant::PixelCount]);
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
                template_replacements!(
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
    fn push_constants(&self) -> PushConstantGroup {
        self.push_constants.clone()
    }

    fn input_render_attachments(&self) -> RenderAttachmentInputDescriptionSet {
        RenderAttachmentInputDescriptionSet::empty()
    }

    fn gpu_resource_group_id(&self) -> GPUResourceGroupID {
        self.gpu_resource_group_id
    }

    fn determine_workgroup_counts(&self, _rendering_surface: &RenderingSurface) -> [u32; 3] {
        [1; 3]
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::validate_template;
    use super::*;
    use impact_utils::hash64;

    #[test]
    fn should_resolve_to_valid_wgsl() {
        validate_template(&LuminanceHistogramAverageShaderTemplate::new(
            256,
            GPUResourceGroupID(hash64!("test".to_string())),
        ));
    }
}
