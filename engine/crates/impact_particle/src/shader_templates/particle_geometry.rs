//! Shader template for the particle geometry pass.

use crate::{
    gpu_resource::{ParticleImmediate, ParticleImmediateGroup, ParticleMaterialGPUResources},
    rendering_template_source,
};
use impact_camera::gpu_resource::CameraProjectionUniform;
use impact_gpu::{
    immediate::ImmediateGroup,
    shader::template::{ShaderTemplate, SpecificShaderTemplate},
    shader_template_replacements,
};
use impact_rendering::immediate::BasicImmediate;
use std::sync::LazyLock;

/// Shader template for the particle geometry pass, which extracts the relevant
/// geometrical information and material properties from the particles and
/// writes them to the corresponding render attachments (the G-buffer).
#[derive(Clone, Copy, Debug)]
pub struct ParticleGeometryShaderTemplate {
    n_particle_materials: u32,
}

static TEMPLATE: LazyLock<ShaderTemplate<'static>> =
    LazyLock::new(|| ShaderTemplate::new(rendering_template_source!("particle_geometry")).unwrap());

impl ParticleGeometryShaderTemplate {
    /// Creates a new particle geometry shader template for the given number of
    /// particle materials.
    pub fn new(n_particle_materials: u32) -> Self {
        Self {
            n_particle_materials,
        }
    }

    /// Returns the group of immediates used by the shader.
    pub fn immediates() -> ParticleImmediateGroup {
        ImmediateGroup::from_iter([
            ParticleImmediate::Rendering(BasicImmediate::WorldToCameraRotation),
            ParticleImmediate::Rendering(BasicImmediate::PreviousWorldToCameraRotation),
            ParticleImmediate::Rendering(BasicImmediate::WorldToCameraTranslation),
            ParticleImmediate::Rendering(BasicImmediate::PreviousWorldToCameraTranslation),
            ParticleImmediate::Rendering(BasicImmediate::InverseWindowDimensions),
            ParticleImmediate::Rendering(BasicImmediate::FrameCounter),
            ParticleImmediate::Rendering(BasicImmediate::Exposure),
            ParticleImmediate::TimeStepDuration,
            ParticleImmediate::MaterialIdx,
        ])
    }
}

impl SpecificShaderTemplate for ParticleGeometryShaderTemplate {
    fn resolve(&self) -> String {
        TEMPLATE
            .resolve(
                &[],
                shader_template_replacements!(
                    "n_particle_materials" => self.n_particle_materials,
                    "jitter_count" => CameraProjectionUniform::jitter_count(),
                    "quad_texture_coords_location" => 0,
                    "projection_uniform_group" => 0,
                    "projection_uniform_binding" => CameraProjectionUniform::binding(),
                    "material_group" => 1,
                    "fixed_material_uniform_binding" => ParticleMaterialGPUResources::fixed_properties_binding(),
                    "normal_and_alpha_texture_array_binding" => ParticleMaterialGPUResources::normal_and_alpha_texture_array_binding(),
                    "normal_and_alpha_sampler_binding" => ParticleMaterialGPUResources::normal_and_alpha_sampler_binding(),
                    "particle_group" => 2,
                    "particle_binding" => 0,
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
        validate_template(&ParticleGeometryShaderTemplate::new(16));
    }
}
