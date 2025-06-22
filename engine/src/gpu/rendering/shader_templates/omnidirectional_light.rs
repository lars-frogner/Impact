//! Shader template for the omnidirectional light pass.

use crate::{
    camera::buffer::CameraProjectionUniform,
    gpu::rendering::{
        attachment::{
            Blending, RenderAttachmentDescription, RenderAttachmentInputDescriptionSet,
            RenderAttachmentOutputDescription, RenderAttachmentOutputDescriptionSet,
            RenderAttachmentQuantity::{
                self, LinearDepth, Luminance, MaterialColor, MaterialProperties, NormalVector,
            },
            RenderAttachmentQuantitySet,
        },
        push_constant::{RenderingPushConstantGroup, RenderingPushConstantVariant},
    },
    light::buffer::LightGPUBufferManager,
    mesh::{self, MeshID, VertexAttributeSet, buffer::MeshVertexAttributeLocation},
    rendering_template_source,
};
use impact_gpu::{
    shader::template::{ShaderTemplate, SpecificShaderTemplate},
    shader_template_replacements,
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
    pub fn push_constants() -> RenderingPushConstantGroup {
        RenderingPushConstantGroup::for_vertex_fragment([
            RenderingPushConstantVariant::InverseWindowDimensions,
            RenderingPushConstantVariant::LightIdx,
            RenderingPushConstantVariant::Exposure,
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
        validate_template(&OmnidirectionalLightShaderTemplate::new(5));
    }
}
