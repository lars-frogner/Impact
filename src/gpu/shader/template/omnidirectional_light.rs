//! Shader template for the omnidirectional light pass.

use crate::{
    camera::buffer::CameraProjectionUniform,
    gpu::{
        push_constant::{PushConstantGroup, PushConstantVariant},
        shader::template::{ShaderTemplate, SpecificShaderTemplate},
        texture::{
            attachment::{
                Blending, RenderAttachmentDescription, RenderAttachmentInputDescriptionSet,
                RenderAttachmentOutputDescription, RenderAttachmentOutputDescriptionSet,
                RenderAttachmentQuantity::{
                    self, LinearDepth, Luminance, MaterialColor, MaterialProperties, NormalVector,
                },
                RenderAttachmentQuantitySet,
            },
            shadow_map::ShadowCubemapTexture,
        },
    },
    light::buffer::LightGPUBufferManager,
    mesh::{self, buffer::MeshVertexAttributeLocation, MeshID, VertexAttributeSet},
    rendering_template_source, template_replacements,
};
use std::sync::LazyLock;

/// Shader template for the omnidirectional light pass, which computes the
/// reflected luminance due to an omnidirectional light and adds it to the
/// luminance attachment.
#[derive(Clone, Debug)]
pub struct OmnidirectionalLightShaderTemplate {
    max_light_count: usize,
}

static TEMPLATE: LazyLock<ShaderTemplate<'static>> = LazyLock::new(|| {
    ShaderTemplate::new(rendering_template_source!("omnidirectional_light")).unwrap()
});

impl OmnidirectionalLightShaderTemplate {
    /// Creates a new omnidirectional light shader template for the given
    /// maximum number of omnidirectional lights.
    pub fn new(max_light_count: usize) -> Self {
        Self { max_light_count }
    }

    /// Returns the group of push constants used by the shader.
    pub fn push_constants() -> PushConstantGroup {
        PushConstantGroup::for_vertex_fragment([
            PushConstantVariant::InverseWindowDimensions,
            PushConstantVariant::LightIdx,
            PushConstantVariant::Exposure,
        ])
    }

    /// Returns the set of vertex attributes used by the shader.
    pub fn vertex_attributes() -> VertexAttributeSet {
        VertexAttributeSet::POSITION
    }

    /// Returns the set of render attachments used as input by the shader.
    pub fn input_render_attachments() -> RenderAttachmentInputDescriptionSet {
        RenderAttachmentInputDescriptionSet::with_defaults(
            RenderAttachmentQuantitySet::LINEAR_DEPTH
                | RenderAttachmentQuantitySet::NORMAL_VECTOR
                | RenderAttachmentQuantitySet::MATERIAL_COLOR
                | RenderAttachmentQuantitySet::MATERIAL_PROPERTIES,
        )
    }

    /// Returns the render attachment quantity that the shader will write to.
    pub fn output_render_attachment_quantity() -> RenderAttachmentQuantity {
        Luminance
    }

    /// Returns the descriptions of the render attachments that the shader will
    /// write to.
    pub fn output_render_attachments() -> RenderAttachmentOutputDescriptionSet {
        RenderAttachmentOutputDescriptionSet::single(
            RenderAttachmentOutputDescription::default_for(
                Self::output_render_attachment_quantity(),
            )
            .with_blending(Blending::Additive),
        )
    }

    /// Returns the ID of the light volume mesh used by the shader (a spherical
    /// mesh).
    pub fn light_volume_mesh_id() -> MeshID {
        mesh::spherical_light_volume_mesh_id()
    }
}

impl SpecificShaderTemplate for OmnidirectionalLightShaderTemplate {
    fn resolve(&self) -> String {
        TEMPLATE
            .resolve(
                ["emulate_area_light_reflection"],
                template_replacements!(
                    "max_light_count" => self.max_light_count,
                    "projection_uniform_group" => 0,
                    "projection_uniform_binding" => CameraProjectionUniform::binding(),
                    "linear_depth_texture_group" => 1,
                    "linear_depth_texture_binding" => LinearDepth.texture_binding(),
                    "linear_depth_sampler_binding" => LinearDepth.sampler_binding(),
                    "normal_vector_texture_group" => 2,
                    "normal_vector_texture_binding" => NormalVector.texture_binding(),
                    "normal_vector_sampler_binding" => NormalVector.sampler_binding(),
                    "material_color_texture_group" => 3,
                    "material_color_texture_binding" => MaterialColor.texture_binding(),
                    "material_color_sampler_binding" => MaterialColor.sampler_binding(),
                    "material_properties_texture_group" => 4,
                    "material_properties_texture_binding" => MaterialProperties.texture_binding(),
                    "material_properties_sampler_binding" => MaterialProperties.sampler_binding(),
                    "light_uniform_group" => 5,
                    "light_uniform_binding" => LightGPUBufferManager::light_binding(),
                    "shadow_map_texture_group" => 6,
                    "shadow_map_texture_binding" => ShadowCubemapTexture::texture_binding(),
                    "shadow_map_sampler_binding" => ShadowCubemapTexture::sampler_binding(),
                    "shadow_map_comparison_sampler_binding" => ShadowCubemapTexture::comparison_sampler_binding(),
                    "position_location" => MeshVertexAttributeLocation::Position as u32,
                ),
            )
            .expect("Shader template resolution failed")
    }
}

#[cfg(test)]
mod test {
    use super::super::test::validate_template;
    use super::*;

    #[test]
    fn should_resolve_to_valid_wgsl() {
        validate_template(&OmnidirectionalLightShaderTemplate::new(5));
    }
}
