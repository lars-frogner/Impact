//! Shader template for unidirectional light shadow map update passes.

use crate::{
    gpu::{
        push_constant::{PushConstantGroup, PushConstantVariant},
        shader::template::{ShaderTemplate, SpecificShaderTemplate},
    },
    light::{MAX_SHADOW_MAP_CASCADES, buffer::LightGPUBufferManager},
    mesh::buffer::TriangleMeshVertexAttributeLocation,
    model::transform::InstanceModelLightTransform,
    rendering_template_source, template_replacements,
};
use std::sync::LazyLock;

/// Shader template for unidirectional light shadow map update passes, which
/// write the depths of shadow casting model instances along the light direction
/// within one of multiple bounding boxes (cascades) covering parts of the
/// visible scene to the shadow map cascade textures of a unidirectional light.
#[derive(Clone, Debug)]
pub struct UnidirectionalLightShadowMapShaderTemplate {
    max_light_count: usize,
}

static TEMPLATE: LazyLock<ShaderTemplate<'static>> = LazyLock::new(|| {
    ShaderTemplate::new(rendering_template_source!(
        "unidirectional_light_shadow_map"
    ))
    .unwrap()
});

impl UnidirectionalLightShadowMapShaderTemplate {
    /// Creates a new unidirectional light shadow map shader template for the
    /// given maximum number of unidirectional lights.
    pub fn new(max_light_count: usize) -> Self {
        Self { max_light_count }
    }

    /// Returns the group of push constants used by the shader.
    pub fn push_constants() -> PushConstantGroup {
        PushConstantGroup::for_vertex([
            PushConstantVariant::LightIdx,
            PushConstantVariant::ShadowMapArrayIdx,
        ])
    }
}

impl SpecificShaderTemplate for UnidirectionalLightShadowMapShaderTemplate {
    fn resolve(&self) -> String {
        TEMPLATE
            .resolve(
                [],
                template_replacements!(
                    "max_light_count" => self.max_light_count,
                    "cascade_count" => MAX_SHADOW_MAP_CASCADES,
                    "model_light_transform_rotation_location" => InstanceModelLightTransform::rotation_location(),
                    "model_light_transform_translation_location" => InstanceModelLightTransform::translation_and_scaling_location(),
                    "light_uniform_group" => 0,
                    "light_uniform_binding" => LightGPUBufferManager::light_binding(),
                    "position_location" => TriangleMeshVertexAttributeLocation::Position as u32,
                )
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
        validate_template(&UnidirectionalLightShadowMapShaderTemplate::new(5));
    }
}
