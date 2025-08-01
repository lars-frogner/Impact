//! Shader template for the shadowable omnidirectional light pass.

use crate::rendering_template_source;
use crate::{
    attachment::{
        Blending, RenderAttachmentDescription, RenderAttachmentInputDescriptionSet,
        RenderAttachmentOutputDescription, RenderAttachmentOutputDescriptionSet,
        RenderAttachmentQuantity::{
            self, LinearDepth, Luminance, MaterialColor, MaterialProperties, NormalVector,
        },
        RenderAttachmentQuantitySet,
    },
    push_constant::{BasicPushConstantGroup, BasicPushConstantVariant},
};
use impact_camera::buffer::CameraProjectionUniform;
use impact_gpu::{
    shader::template::{ShaderTemplate, SpecificShaderTemplate},
    shader_template_replacements,
};
use impact_light::{buffer::LightGPUBufferManager, shadow_map::ShadowCubemapTexture};
use impact_mesh::{
    self, TriangleMeshID, VertexAttributeSet, gpu_resource::MeshVertexAttributeLocation,
};
use std::sync::LazyLock;

/// Shader template for the shadowable omnidirectional light pass, which
/// computes the reflected luminance due to a shadowable omnidirectional light
/// and adds it to the luminance attachment.
#[derive(Clone, Debug)]
pub struct ShadowableOmnidirectionalLightShaderTemplate {
    max_light_count: usize,
}

static TEMPLATE: LazyLock<ShaderTemplate<'static>> = LazyLock::new(|| {
    ShaderTemplate::new(rendering_template_source!(
        "shadowable_omnidirectional_light"
    ))
    .unwrap()
});

impl ShadowableOmnidirectionalLightShaderTemplate {
    /// Creates a new shadowable omnidirectional light shader template for the
    /// given maximum number of shadowable omnidirectional lights.
    pub fn new(max_light_count: usize) -> Self {
        Self { max_light_count }
    }

    /// Returns the group of push constants used by the shader.
    pub fn push_constants() -> BasicPushConstantGroup {
        BasicPushConstantGroup::for_vertex_fragment([
            BasicPushConstantVariant::InverseWindowDimensions,
            BasicPushConstantVariant::LightIdx,
            BasicPushConstantVariant::Exposure,
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
    pub fn light_volume_mesh_id() -> TriangleMeshID {
        impact_mesh::builtin::spherical_light_volume_mesh_id()
    }
}

impl SpecificShaderTemplate for ShadowableOmnidirectionalLightShaderTemplate {
    fn resolve(&self) -> String {
        TEMPLATE
            .resolve(
                ["emulate_area_light_reflection"],
                shader_template_replacements!(
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
        validate_template(&ShadowableOmnidirectionalLightShaderTemplate::new(5));
    }
}
