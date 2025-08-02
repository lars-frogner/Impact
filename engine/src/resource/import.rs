//! Importing resources.

use anyhow::Result;
use impact_mesh::import::TriangleMeshDeclaration;
use impact_texture::import::ImageTextureDeclaration;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ResourceDeclarations {
    pub triangle_meshes: Vec<TriangleMeshDeclaration>,
    pub textures: Vec<ImageTextureDeclaration>,
}

impl ResourceDeclarations {
    /// Parses the declarations from the RON file at the given path and resolves
    /// any specified paths.
    pub fn from_ron_file(file_path: impl AsRef<Path>) -> Result<Self> {
        let file_path = file_path.as_ref();
        let mut declarations: Self = impact_io::parse_ron_file(file_path)?;
        if let Some(root_path) = file_path.parent() {
            declarations.resolve_paths(root_path);
        }
        Ok(declarations)
    }

    /// Resolves all paths in the declarations by prepending the given root path
    /// to all paths.
    fn resolve_paths(&mut self, root_path: &Path) {
        for declaration in &mut self.triangle_meshes {
            declaration.resolve_paths(root_path);
        }
        for declaration in &mut self.textures {
            declaration.resolve_paths(root_path);
        }
    }
}
