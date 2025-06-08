//! Shader template for the bloom blending pass.

use crate::{
    gpu::{
        shader::{
            ShaderID,
            template::{ShaderTemplate, SpecificShaderTemplate},
        },
        texture::attachment::RenderAttachmentQuantity,
    },
    mesh::buffer::TriangleMeshVertexAttributeLocation,
    rendering_template_source, template_replacements,
};
use std::sync::LazyLock;

/// Shader template for the bloom blending pass, which blends the blurred
/// luminance (in mip level 1) with the original luminance to produce the bloom
/// effect.
#[derive(Clone, Debug)]
pub struct BloomBlendingShaderTemplate {
    shader_id: ShaderID,
    luminance_quantity: RenderAttachmentQuantity,
    blurred_luminance_quantity: RenderAttachmentQuantity,
    blurred_luminance_normalization: f32,
    blurred_luminance_weight: f32,
}

static TEMPLATE: LazyLock<ShaderTemplate<'static>> =
    LazyLock::new(|| ShaderTemplate::new(rendering_template_source!("bloom_blending")).unwrap());

impl BloomBlendingShaderTemplate {
    /// Creates a new bloom blending shader template for the given render
    /// attachment quantities holding the original and blurred luminance and the
    /// given normalization factor and blend weight for the blurred luminance.
    pub fn new(
        shader_id: ShaderID,
        luminance_quantity: RenderAttachmentQuantity,
        blurred_luminance_quantity: RenderAttachmentQuantity,
        blurred_luminance_normalization: f32,
        blurred_luminance_weight: f32,
    ) -> Self {
        Self {
            shader_id,
            luminance_quantity,
            blurred_luminance_quantity,
            blurred_luminance_normalization,
            blurred_luminance_weight,
        }
    }
}

impl SpecificShaderTemplate for BloomBlendingShaderTemplate {
    fn resolve(&self) -> String {
        TEMPLATE
            .resolve(
                [],
                template_replacements!(
                    "blurred_luminance_normalization" => self.blurred_luminance_normalization,
                    "blurred_luminance_weight" => self.blurred_luminance_weight,
                    "luminance_texture_group" => 0,
                    "luminance_texture_binding" => self.luminance_quantity.texture_binding(),
                    "luminance_sampler_binding" => self.luminance_quantity.sampler_binding(),
                    "blurred_luminance_texture_group" => 1,
                    "blurred_luminance_texture_binding" => self.blurred_luminance_quantity.texture_binding(),
                    "blurred_luminance_sampler_binding" => self.blurred_luminance_quantity.sampler_binding(),
                    "position_location" => TriangleMeshVertexAttributeLocation::Position as u32,
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
    use super::super::tests::validate_template;
    use super::*;

    #[test]
    fn should_resolve_to_valid_wgsl() {
        validate_template(&BloomBlendingShaderTemplate::new(
            ShaderID::from_identifier("BloomBlendingShaderTemplate"),
            RenderAttachmentQuantity::Luminance,
            RenderAttachmentQuantity::LuminanceAux,
            0.25,
            0.04,
        ));
    }
}
