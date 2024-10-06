//! Shader template for bloom upsampling and blurring passes.

use crate::{
    gpu::{
        shader::template::{ShaderTemplate, SpecificShaderTemplate},
        texture::attachment::RenderAttachmentQuantity,
    },
    mesh::buffer::MeshVertexAttributeLocation,
    rendering_template_source, template_replacements,
};
use std::sync::LazyLock;

/// Shader template for bloom upsampling and blurring passes, which successively
/// upsample and blur the mip levels of a render attachment.
#[derive(Clone, Debug)]
pub struct BloomUpsamplingBlurShaderTemplate {
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
        render_attachment_quantity: RenderAttachmentQuantity,
        blur_filter_radius: f32,
    ) -> Self {
        assert!(blur_filter_radius > 0.0);
        Self {
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
                template_replacements!(
                    "blur_filter_radius" => self.blur_filter_radius,
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
    use super::super::tests::validate_template;
    use super::*;

    #[test]
    fn should_resolve_to_valid_wgsl() {
        validate_template(&BloomUpsamplingBlurShaderTemplate::new(
            RenderAttachmentQuantity::LuminanceAux,
            0.005,
        ));
    }
}
