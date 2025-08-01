//! Shader template for bloom upsampling and blurring passes.

use crate::attachment::RenderAttachmentQuantity;
use crate::rendering_template_source;
use impact_gpu::{
    shader::{
        ShaderID,
        template::{ShaderTemplate, SpecificShaderTemplate},
    },
    shader_template_replacements,
};
use impact_mesh::gpu_resource::MeshVertexAttributeLocation;
use std::sync::LazyLock;

/// Shader template for bloom upsampling and blurring passes, which successively
/// upsample and blur the mip levels of a render attachment.
#[derive(Clone, Debug)]
pub struct BloomUpsamplingBlurShaderTemplate {
    shader_id: ShaderID,
    render_attachment_quantity: RenderAttachmentQuantity,
    blur_filter_radius: f32,
}

static TEMPLATE: LazyLock<ShaderTemplate<'static>> = LazyLock::new(|| {
    ShaderTemplate::new(rendering_template_source!("bloom_upsampling_blur")).unwrap()
});

impl BloomUpsamplingBlurShaderTemplate {
    /// Creates a new bloom upsampling and blurring shader template for the
    /// given render attachment quantity and blur filter radius.
    pub fn new(
        shader_id: ShaderID,
        render_attachment_quantity: RenderAttachmentQuantity,
        blur_filter_radius: f32,
    ) -> Self {
        assert!(blur_filter_radius > 0.0);
        Self {
            shader_id,
            render_attachment_quantity,
            blur_filter_radius,
        }
    }
}

impl SpecificShaderTemplate for BloomUpsamplingBlurShaderTemplate {
    fn resolve(&self) -> String {
        TEMPLATE
            .resolve(
                [],
                shader_template_replacements!(
                    "blur_filter_radius" => self.blur_filter_radius,
                    "input_texture_binding" => self.render_attachment_quantity.texture_binding(),
                    "input_sampler_binding" => self.render_attachment_quantity.sampler_binding(),
                    "position_location" => MeshVertexAttributeLocation::Position as u32,
                ),
            )
            .expect("Shader template resolution failed")
    }

    fn shader_id(&self) -> ShaderID {
        self.shader_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use impact_gpu::shader::{ShaderID, template::validate_template};

    #[test]
    fn should_resolve_to_valid_wgsl() {
        validate_template(&BloomUpsamplingBlurShaderTemplate::new(
            ShaderID::from_identifier("BloomUpsamplingBlurShaderTemplate"),
            RenderAttachmentQuantity::LuminanceAux,
            0.005,
        ));
    }
}
