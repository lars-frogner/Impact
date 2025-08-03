//! Resource management.

pub mod import;

use crate::resource::import::ResourceDeclarations;
use anyhow::Result;
use impact_material::{MaterialRegistry, MaterialTemplateRegistry, MaterialTextureGroupRegistry};
use impact_mesh::{LineSegmentMeshRegistry, TriangleMeshRegistry};
use impact_rendering::resource::BasicResourceRegistries;
use impact_texture::{SamplerRegistry, TextureRegistry, lookup_table::LookupTableRegistry};
use impact_voxel::{gpu_resource::VoxelResourceRegistries, voxel_types::VoxelTypeRegistry};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct ResourceConfig {
    /// Path to a file containing a [`ResourceDeclarations`] object serialized
    /// as RON (Rusty Object Notation). The resources specified in the file will
    /// be automatically loaded on startup.
    pub resource_file_path: Option<PathBuf>,
    /// Path to the folder where automatically computed lookup tables should be
    /// stored.
    pub lookup_table_dir: PathBuf,
}

/// Owner and manager of all resource registries.
#[derive(Debug)]
pub struct ResourceManager {
    pub triangle_meshes: TriangleMeshRegistry,
    pub line_segment_meshes: LineSegmentMeshRegistry,
    pub textures: TextureRegistry,
    pub samplers: SamplerRegistry,
    pub lookup_tables: LookupTableRegistry,
    pub materials: MaterialRegistry,
    pub material_templates: MaterialTemplateRegistry,
    pub material_texture_groups: MaterialTextureGroupRegistry,
    pub voxel_types: VoxelTypeRegistry,
    pub config: ResourceConfig,
}

impl ResourceConfig {
    /// Reads the [`ResourceDeclarations`] from the resource file.
    pub fn read_declarations(&self) -> Result<ResourceDeclarations> {
        let Some(resource_file_path) = self.resource_file_path.as_ref() else {
            return Ok(ResourceDeclarations::default());
        };
        ResourceDeclarations::from_ron_file(resource_file_path)
    }

    /// Resolves all paths in the configuration by prepending the given root
    /// path to all paths.
    pub fn resolve_paths(&mut self, root_path: &Path) {
        if let Some(resource_file_path) = self.resource_file_path.as_mut() {
            *resource_file_path = root_path.join(&resource_file_path);
        }
        self.lookup_table_dir = root_path.join(&self.lookup_table_dir);
    }
}

impl Default for ResourceConfig {
    fn default() -> Self {
        Self {
            resource_file_path: None,
            lookup_table_dir: PathBuf::from("resources/lookup_tables"),
        }
    }
}

impl ResourceManager {
    pub fn new(
        config: ResourceConfig,
        textures: TextureRegistry,
        samplers: SamplerRegistry,
        voxel_types: VoxelTypeRegistry,
    ) -> Self {
        Self {
            triangle_meshes: TriangleMeshRegistry::new(),
            line_segment_meshes: LineSegmentMeshRegistry::new(),
            textures,
            samplers,
            lookup_tables: LookupTableRegistry::new(),
            materials: MaterialRegistry::new(),
            material_templates: MaterialTemplateRegistry::new(),
            material_texture_groups: MaterialTextureGroupRegistry::new(),
            voxel_types,
            config,
        }
    }

    /// Parses the resource file pointed to in the [`ResourceConfig`] and loads
    /// all resources declared in the file.
    ///
    /// # Returns
    /// The parsed [`ResourceDeclarations`].
    ///
    /// # Errors
    /// Returns an error if the resource file does not exist or is invalid. See
    /// also [`Self::load_declared_resources`].
    pub fn load_resources_declared_in_config(&mut self) -> Result<()> {
        let declarations = self.config.read_declarations()?;
        self.load_declared_resources(&declarations)?;
        Ok(())
    }

    /// Loads all builtin (always available) resources.
    pub fn load_builtin_resources(&mut self) -> Result<()> {
        impact_mesh::builtin::load_builtin_meshes(&mut self.triangle_meshes)?;

        impact_rendering::lookup_tables::initialize_default_lookup_tables(
            &mut self.textures,
            &mut self.samplers,
            &mut self.lookup_tables,
            &self.config.lookup_table_dir,
        )?;

        Ok(())
    }

    /// Loads all resources in the given declarations.
    pub fn load_declared_resources(
        &mut self,
        resource_declarations: &ResourceDeclarations,
    ) -> Result<()> {
        impact_mesh::import::load_declared_meshes(
            &mut self.triangle_meshes,
            &resource_declarations.triangle_meshes,
        )?;
        impact_texture::import::load_declared_image_textures(
            &mut self.textures,
            &mut self.samplers,
            &resource_declarations.textures,
        )?;
        impact_material::import::load_declared_materials(
            &self.textures,
            &self.samplers,
            &mut self.materials,
            &mut self.material_templates,
            &mut self.material_texture_groups,
            &resource_declarations.materials,
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

    fn texture(&self) -> &TextureRegistry {
        &self.textures
    }

    fn sampler(&self) -> &SamplerRegistry {
        &self.samplers
    }

    fn lookup_table(&self) -> &LookupTableRegistry {
        &self.lookup_tables
    }

    fn material(&self) -> &MaterialRegistry {
        &self.materials
    }

    fn material_template(&self) -> &MaterialTemplateRegistry {
        &self.material_templates
    }

    fn material_texture_group(&self) -> &MaterialTextureGroupRegistry {
        &self.material_texture_groups
    }
}

impl VoxelResourceRegistries for ResourceManager {
    fn voxel_type(&self) -> &VoxelTypeRegistry {
        &self.voxel_types
    }
}
