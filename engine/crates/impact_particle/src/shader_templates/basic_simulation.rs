//! Shader template for basic particle simulation compute passes.

use crate::{
    compute_template_source,
    gpu_resource::{ParticleImmediate, ParticleImmediateGroup},
    shader_templates::ParticleSimulationShaderTemplate,
};
use impact_gpu::{
    shader::template::{ShaderTemplate, SpecificShaderTemplate},
    shader_template_replacements,
};
use std::sync::LazyLock;

/// Shader template for a basic particle simulation compute pass.
#[derive(Clone, Debug)]
pub struct BasicParticleSimulationShaderTemplate {
    max_simulation_count: usize,
}

static TEMPLATE: LazyLock<ShaderTemplate<'static>> =
    LazyLock::new(|| ShaderTemplate::new(compute_template_source!("basic_simulation")).unwrap());

impl ParticleSimulationShaderTemplate for BasicParticleSimulationShaderTemplate {
    fn new(max_simulation_count: usize) -> Self {
        Self {
            max_simulation_count,
        }
    }

    fn workgroup_size() -> u32 {
        16
    }

    fn determine_workgroup_counts(particle_count: u32) -> [u32; 3] {
        [particle_count.div_ceil(Self::workgroup_size()), 1, 1]
    }

    fn immediates() -> ParticleImmediateGroup {
        ParticleImmediateGroup::from_iter([
            ParticleImmediate::ParticleRange,
            ParticleImmediate::SimulationIdx,
            ParticleImmediate::TimeStep,
            ParticleImmediate::TimeStepDuration,
        ])
    }
}

impl SpecificShaderTemplate for BasicParticleSimulationShaderTemplate {
    fn resolve(&self) -> String {
        TEMPLATE
            .resolve(
                &[],
                shader_template_replacements!(
                    "max_simulation_count" => self.max_simulation_count,
                    "simulation_and_particle_group" => 0,
                    "simulation_binding" => 0,
                    "particle_binding" => 1,
                    "workgroup_size" => Self::workgroup_size(),
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
        validate_template(&BasicParticleSimulationShaderTemplate::new(5));
    }
}
