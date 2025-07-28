//! Importing resources.

use anyhow::Result;
use impact_mesh::import::TriangleMeshSpecification;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ResourceSpecifications {
    pub triangle_meshes: Vec<TriangleMeshSpecification>,
}

impl ResourceSpecifications {
    /// Parses the specifications from the RON file at the given path and
    /// resolves any specified paths.
    pub fn from_ron_file(file_path: impl AsRef<Path>) -> Result<Self> {
        let file_path = file_path.as_ref();
        let mut specs: Self = impact_io::parse_ron_file(file_path)?;
        if let Some(root_path) = file_path.parent() {
            specs.resolve_paths(root_path);
        }
        Ok(specs)
    }

    /// Resolves all paths in the specifications by prepending the given root
    /// path to all paths.
    fn resolve_paths(&mut self, root_path: &Path) {
        for specification in &mut self.triangle_meshes {
            specification.resolve_paths(root_path);
        }
    }
}
