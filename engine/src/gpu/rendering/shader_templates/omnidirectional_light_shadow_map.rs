//! Shader template for omnidirectional light shadow map update passes.

use crate::{
    gpu::rendering::push_constant::{RenderingPushConstantGroup, RenderingPushConstantVariant},
    light::buffer::LightGPUBufferManager,
    mesh::buffer::MeshVertexAttributeLocation,
    model::transform::InstanceModelLightTransform,
    rendering_template_source,
};
use impact_gpu::{
    shader::template::{ShaderTemplate, SpecificShaderTemplate},
    shader_template_replacements,
};
use std::sync::LazyLock;

/// Shader template for omnidirectional light shadow map update passes, which
/// write the linear depths of shadow casting model instances from the point of
/// view of an omnidirectional light to the textures representing the faces of
/// the light's shadow cubemap.
#[derive(Clone, Debug)]
pub struct OmnidirectionalLightShadowMapShaderTemplate {
    max_light_count: usize,
}

static TEMPLATE: LazyLock<ShaderTemplate<'static>> = LazyLock::new(|| {
    ShaderTemplate::new(rendering_template_source!(
        "omnidirectional_light_shadow_map"
    ))
    .unwrap()
});

impl OmnidirectionalLightShadowMapShaderTemplate {
    /// Creates a new omnidirectional light shadow map shader template for the
    /// given maximum number of omnidirectional lights.
    pub fn new(max_light_count: usize) -> Self {
        Self { max_light_count }
    }

    /// Returns the group of push constants used by the shader.
    pub fn push_constants() -> RenderingPushConstantGroup {
        RenderingPushConstantGroup::for_fragment([RenderingPushConstantVariant::LightIdx])
    }
}

impl SpecificShaderTemplate for OmnidirectionalLightShadowMapShaderTemplate {
    fn resolve(&self) -> String {
        TEMPLATE
            .resolve(
                [],
                shader_template_replacements!(
                    "max_light_count" => self.max_light_count,
                    "model_light_transform_rotation_location" => InstanceModelLightTransform::rotation_location(),
                    "model_light_transform_translation_location" => InstanceModelLightTransform::translation_and_scaling_location(),
                    "light_uniform_group" => 0,
                    "light_uniform_binding" => LightGPUBufferManager::light_binding(),
                    "position_location" => MeshVertexAttributeLocation::Position as u32,
                )
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
        validate_template(&OmnidirectionalLightShadowMapShaderTemplate::new(5));
    }
}
