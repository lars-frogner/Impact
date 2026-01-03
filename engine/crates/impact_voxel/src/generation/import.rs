//! Importing voxel object generators from declarations.

use crate::generation::VoxelGeneratorID;
use std::path::{Path, PathBuf};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct VoxelGeneratorDeclaration {
    pub id: VoxelGeneratorID,
    pub path: PathBuf,
}

impl VoxelGeneratorDeclaration {
    /// Resolves all paths in the declaration by prepending the given root path
    /// to all paths.
    pub fn resolve_paths(&mut self, root_path: &Path) {
        self.path = root_path.join(&self.path);
    }
}

/// Loads all voxel generators in the given declarations and stores them in the
/// voxel generator registry.
///
/// # Errors
/// See [`load_declared_voxel_generator`].
#[cfg(feature = "ron")]
pub fn load_declared_voxel_generators(
    registry: &mut crate::generation::VoxelGeneratorRegistry,
    declarations: &[VoxelGeneratorDeclaration],
) -> anyhow::Result<()> {
    for declaration in declarations {
        if let Err(error) = load_declared_voxel_generator(registry, declaration) {
            // Failing to load a voxel generator is not fatal, since we might not need it
            log::error!(
                "Failed to load voxel generator {}: {error:#}",
                declaration.id
            );
        }
    }
    Ok(())
}

/// Loads the voxel generator in the given declaration and stores it in the
/// voxel generator registry.
///
/// # Errors
/// Returns an error if:
/// - Another voxel generator with the same name is already loaded.
/// - The voxel generator file can not be found or is invalid.
#[cfg(feature = "ron")]
pub fn load_declared_voxel_generator(
    registry: &mut crate::generation::VoxelGeneratorRegistry,
    declaration: &VoxelGeneratorDeclaration,
) -> anyhow::Result<VoxelGeneratorID> {
    use anyhow::Context;

    let id = declaration.id;
    let path = &declaration.path;

    log::debug!("Loading voxel generator `{id}` from {}", path.display());

    if registry.contains(id) {
        anyhow::bail!("Tried to load voxel generator under already existing ID: {id}");
    }

    let generator = impact_io::parse_ron_file(path)
        .with_context(|| format!("Failed to load voxel generator from {}", path.display()))?;

    registry.insert(id, generator);

    Ok(id)
}
