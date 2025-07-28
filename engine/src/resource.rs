//! Resource management.

pub mod import;

use crate::resource::import::ResourceSpecifications;
use anyhow::Result;
use impact_mesh::{LineSegmentMeshRegistry, TriangleMeshRegistry};
use impact_rendering::resource::BasicResourceRegistries;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct ResourceConfig {
    /// Path to a file containing a [`ResourceSpecifications`] object serialized
    /// as RON (Rusty Object Notation). The resources specified in the file will
    /// be automatically loaded on startup.
    pub resource_file_path: Option<PathBuf>,
}

/// Owner and manager of all resource registries.
#[derive(Debug)]
pub struct ResourceManager {
    pub triangle_meshes: TriangleMeshRegistry,
    pub line_segment_meshes: LineSegmentMeshRegistry,
    pub config: ResourceConfig,
}

impl ResourceConfig {
    /// Reads the [`ResourceSpecifications`] from the resource file.
    pub fn read_specifications(&self) -> Result<ResourceSpecifications> {
        let Some(resource_file_path) = self.resource_file_path.as_ref() else {
            return Ok(ResourceSpecifications::default());
        };
        ResourceSpecifications::from_ron_file(resource_file_path)
    }

    /// Resolves all paths in the configuration by prepending the given root
    /// path to all paths.
    pub fn resolve_paths(&mut self, root_path: &Path) {
        if let Some(resource_file_path) = self.resource_file_path.as_mut() {
            *resource_file_path = root_path.join(&resource_file_path);
        }
    }
}

impl Default for ResourceConfig {
    fn default() -> Self {
        Self {
            resource_file_path: None,
        }
    }
}

impl ResourceManager {
    pub fn new(config: ResourceConfig) -> Self {
        Self {
            triangle_meshes: TriangleMeshRegistry::new(),
            line_segment_meshes: LineSegmentMeshRegistry::new(),
            config,
        }
    }

    /// Parses the resource file pointed to in the [`ResourceConfig`] and loads
    /// all resources specified in the file.
    ///
    /// # Returns
    /// The parsed [`ResourceSpecifications`].
    ///
    /// # Errors Returns an error if the resource file does not exist or is
    /// invalid. See also [`Self::load_specified_resources`].
    pub fn load_resources_specified_in_config(&mut self) -> Result<()> {
        let specifications = self.config.read_specifications()?;
        self.load_specified_resources(&specifications)?;
        Ok(())
    }

    /// Loads all builtin (always available) resources.
    pub fn load_builtin_resources(&mut self) -> Result<()> {
        impact_mesh::builtin::load_builtin_meshes(&mut self.triangle_meshes)?;
        Ok(())
    }

    /// Loads all resources in the given specifications.
    pub fn load_specified_resources(
        &mut self,
        resource_specifications: &ResourceSpecifications,
    ) -> Result<()> {
        impact_mesh::import::load_specified_meshes(
            &mut self.triangle_meshes,
            &resource_specifications.triangle_meshes,
        )?;
        Ok(())
    }
}

impl BasicResourceRegistries for ResourceManager {
    fn triangle_mesh(&self) -> &TriangleMeshRegistry {
        &self.triangle_meshes
    }

    fn line_segment_mesh(&self) -> &LineSegmentMeshRegistry {
        &self.line_segment_meshes
    }
}
