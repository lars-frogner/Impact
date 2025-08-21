//! Shader template for bloom downsampling passes.

use crate::attachment::RenderAttachmentQuantity;
use crate::rendering_template_source;
use impact_gpu::{
    shader::template::{ShaderTemplate, SpecificShaderTemplate},
    shader_template_replacements,
};
use impact_mesh::gpu_resource::MeshVertexAttributeLocation;
use std::sync::LazyLock;

/// Shader template for bloom downsampling passes, which successively downsample
/// the mip levels of a render attachment.
#[derive(Clone, Debug)]
pub struct BloomDownsamplingShaderTemplate {
    render_attachment_quantity: RenderAttachmentQuantity,
}

static TEMPLATE: LazyLock<ShaderTemplate<'static>> = LazyLock::new(|| {
    ShaderTemplate::new(rendering_template_source!("bloom_downsampling")).unwrap()
});

impl BloomDownsamplingShaderTemplate {
    /// Creates a new bloom downsampling shader template for the given render
    /// attachment quantity.
    pub fn new(render_attachment_quantity: RenderAttachmentQuantity) -> Self {
        Self {
            render_attachment_quantity,
        }
    }
}

impl SpecificShaderTemplate for BloomDownsamplingShaderTemplate {
    fn resolve(&self) -> String {
        TEMPLATE
            .resolve(
                &[],
                shader_template_replacements!(
                    "input_texture_binding" => self.render_attachment_quantity.texture_binding(),
                    "input_sampler_binding" => self.render_attachment_quantity.sampler_binding(),
                    "position_location" => MeshVertexAttributeLocation::Position as u32,
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
        validate_template(&BloomDownsamplingShaderTemplate::new(
            RenderAttachmentQuantity::LuminanceAux,
        ));
    }
}
