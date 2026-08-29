//! Importing particle effects from declarations.

use crate::effect::ParticleEffectID;
use std::path::{Path, PathBuf};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct ParticleEffectDeclaration {
    pub id: ParticleEffectID,
    pub path: PathBuf,
}

impl ParticleEffectDeclaration {
    /// Resolves all paths in the declaration by prepending the given root path
    /// to all paths.
    pub fn resolve_paths(&mut self, root_path: &Path) {
        self.path = root_path.join(&self.path);
    }
}

/// Loads all particle effects in the given declarations and stores them in the
/// particle effect registry.
///
/// # Errors
/// See [`load_declared_particle_effect`].
#[cfg(feature = "ron")]
pub fn load_declared_particle_effects(
    registry: &mut crate::effect::ParticleEffectRegistry,
    declarations: &[ParticleEffectDeclaration],
) -> anyhow::Result<()> {
    for declaration in declarations {
        if let Err(error) = load_declared_particle_effect(registry, declaration) {
            // Failing to load a particle effect is not fatal, since we might
            // not need it
            log::error!(
                "Failed to load particle effect {}: {error:#}",
                declaration.id
            );
        }
    }
    Ok(())
}

/// Loads the particle effect in the given declaration and stores it in the
/// particle effect registry.
///
/// # Errors
/// Returns an error if:
/// - Another particle effect with the same name is already loaded.
/// - The particle effect file can not be found or is invalid.
#[cfg(feature = "ron")]
pub fn load_declared_particle_effect(
    registry: &mut crate::effect::ParticleEffectRegistry,
    declaration: &ParticleEffectDeclaration,
) -> anyhow::Result<ParticleEffectID> {
    use anyhow::Context;

    let id = declaration.id;
    let path = &declaration.path;

    log::debug!("Loading particle effect `{id}` from {}", path.display());

    if registry.contains(id) {
        anyhow::bail!("Tried to load particle effect under already existing ID: {id}");
    }

    let effect = impact_io::parse_ron_file(path)
        .with_context(|| format!("Failed to load particle effect from {}", path.display()))?;

    registry.insert(id, effect);

    Ok(id)
}
