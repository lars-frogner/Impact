//! Particle system.

#[macro_use]
mod macros;

pub mod effect;
pub mod emitter;
pub mod gpu_resource;
pub mod material;
pub mod render_commands;
pub mod shader_templates;
pub mod simulation_commands;

use material::ParticleMaterialConfig;
use std::path::Path;

/// Configuration parameters for the particle system.
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(default)
)]
#[derive(Clone, Debug, Default)]
pub struct ParticleConfig {
    /// If [`None`], the particle system will be disabled.
    pub materials: Option<ParticleMaterialConfig>,
}

impl ParticleConfig {
    /// Resolves all paths in the configuration by prepending the given root
    /// path to all paths.
    pub fn resolve_paths(&mut self, root_path: &Path) {
        if let Some(materials) = &mut self.materials {
            materials.resolve_paths(root_path);
        }
    }
}
