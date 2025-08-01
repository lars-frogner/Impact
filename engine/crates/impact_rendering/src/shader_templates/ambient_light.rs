//! Shader template for the ambient light pass.

use crate::{
    attachment::{
        Blending, RenderAttachmentDescription, RenderAttachmentInputDescriptionSet,
        RenderAttachmentOutputDescription, RenderAttachmentOutputDescriptionSet,
        RenderAttachmentQuantity::{
            LinearDepth, Luminance, LuminanceAux, MaterialColor, MaterialProperties, NormalVector,
        },
        RenderAttachmentQuantitySet,
    },
    push_constant::{BasicPushConstantGroup, BasicPushConstantVariant},
    rendering_template_source,
};
use impact_camera::buffer::CameraProjectionUniform;
use impact_gpu::{
    push_constant::PushConstantGroup,
    shader::template::{ShaderTemplate, SpecificShaderTemplate},
    shader_template_replacements,
};
use impact_light::buffer::LightGPUBufferManager;
use impact_mesh::{
    self, TriangleMeshID, VertexAttributeSet, gpu_resource::MeshVertexAttributeLocation,
};
use std::sync::LazyLock;

/// Shader template for the ambient light pass, which computes the reflected
/// luminance due to ambient light and writes it to the auxiliary luminance
/// attachment.
///
/// This shader is also responsible for writing the emissive luminance to the
/// luminance attachment (simply because it happens to have access to all the
/// required information and is only evoked once).
#[derive(Clone, Debug)]
pub struct AmbientLightShaderTemplate {
    max_light_count: usize,
}

static TEMPLATE: LazyLock<ShaderTemplate<'static>> =
    LazyLock::new(|| ShaderTemplate::new(rendering_template_source!("ambient_light")).unwrap());

impl AmbientLightShaderTemplate {
    /// Creates a new ambient light shader template for the given maximum number
    /// of ambient lights.
    pub fn new(max_light_count: usize) -> Self {
        Self { max_light_count }
    }

    /// Returns the group of push constants used by the shader.
    pub fn push_constants() -> BasicPushConstantGroup {
        PushConstantGroup::for_fragment([
            BasicPushConstantVariant::InverseWindowDimensions,
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

    /// Returns the descriptions of the render attachments that the shader will
    /// write to.
    pub fn output_render_attachments() -> RenderAttachmentOutputDescriptionSet {
        RenderAttachmentOutputDescriptionSet::new(vec![
            RenderAttachmentOutputDescription::default_for(Luminance)
                .with_blending(Blending::Additive),
            RenderAttachmentOutputDescription::default_for(LuminanceAux)
                .with_blending(Blending::Additive),
        ])
    }

    /// Returns the ID of the light volume mesh used by the shader (a
    /// screen-filling quad).
    pub fn light_volume_mesh_id() -> TriangleMeshID {
        impact_mesh::builtin::screen_filling_quad_mesh_id()
    }
}

impl SpecificShaderTemplate for AmbientLightShaderTemplate {
    fn resolve(&self) -> String {
        TEMPLATE
            .resolve(
                [],
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
                    "specular_reflectance_lookup_texture_group" => 6,
                    "specular_reflectance_lookup_texture_binding" => 0,
                    "specular_reflectance_lookup_sampler_binding" => 1,
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
        validate_template(&AmbientLightShaderTemplate::new(5));
    }
}
