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
    use impact_math::random::Rng;

    #[test]
    fn should_resolve_to_valid_wgsl() {
        validate_template(&BasicParticleSimulationShaderTemplate::new(5));
    }

    #[derive(Default)]
    struct RateOverTime {
        t0: f32,
        r0: f32,
        t1: f32,
        r1: f32,
        t2: f32,
        r2: f32,
        t3: f32,
        r3: f32,
    }

    const MAX_F32: f32 = 3.4028235e38;

    const PARTICLE_COUNT: u32 = 64;

    fn select<T>(fv: T, tv: T, c: bool) -> T {
        if c { tv } else { fv }
    }

    #[allow(clippy::needless_return, non_snake_case)]
    fn hashToNormF32(hash: u32) -> f32 {
        // Divide by 2^32
        return hash as f32 * 2.3283064e-10;
    }

    #[allow(clippy::needless_return, non_snake_case)]
    fn generateSpawningTime(rot: RateOverTime, hash: u32) -> f32 {
        let A1 = 0.5 * (rot.t1 - rot.t0) * (rot.r0 + rot.r1);
        let A2 = A1 + 0.5 * (rot.t2 - rot.t1) * (rot.r1 + rot.r2);
        let A3 = A2 + 0.5 * (rot.t3 - rot.t2) * (rot.r2 + rot.r3);

        let t = [rot.t0, rot.t1, rot.t2, rot.t3, MAX_F32];
        let r = [rot.r0, rot.r1, rot.r2, rot.r3, MAX_F32];
        let A = [0.0, A1, A2, A3];

        let R = hashToNormF32(hash);

        let k = select(select(select(3, 2, R <= A3), 1, R <= A2), 0, R <= A1);

        let dR = R - A[k];
        let s = select(
            0.0,
            (r[k + 1] - r[k]) / (t[k + 1] - t[k]),
            k < 3 && t[k + 1] > t[k],
        );
        let disc = f32::max(0.0, r[k] * r[k] + 2.0 * s * dR);

        let T = t[k] + 2.0 * dR / (r[k] + f32::sqrt(disc) + 1e-9);

        let spawningTime = T * PARTICLE_COUNT as f32;

        return spawningTime;
    }

    #[test]
    fn feature() {
        let rot = RateOverTime {
            r0: 1e30,
            t1: 1e-3,
            r1: 1e30,
            ..RateOverTime::default()
        };
        let mut rng = Rng::with_seed(0);
        let hash = rng.random_u32_in_range(..);
        dbg!(generateSpawningTime(rot, hash));
    }
}
