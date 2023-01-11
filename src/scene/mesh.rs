//! Management of meshes.

use crate::{
    geometry::{ColorVertex, Mesh, TextureVertex, TriangleMesh},
    num::Float,
};
use anyhow::{anyhow, bail, Result};
use impact_utils::stringhash64_newtype;
use std::{
    collections::{hash_map::Entry, HashMap},
    fmt::Debug,
};

stringhash64_newtype!(
    /// Identifier for specific meshes.
    /// Wraps a [`StringHash64`](impact_utils::StringHash64).
    [pub] MeshID
);

/// Repository where [`Mesh`]es are stored under a
/// unique [`MeshID`].
#[derive(Debug, Default)]
pub struct MeshRepository<F: Float> {
    /// Meshes with vertices that hold color values.
    color_meshes: HashMap<MeshID, TriangleMesh<ColorVertex<F>>>,
    /// Meshes with vertices that hold texture coordinates.
    texture_meshes: HashMap<MeshID, TriangleMesh<TextureVertex<F>>>,
}

impl<F: Float> MeshRepository<F> {
    /// Creates a new empty mesh repository.
    pub fn new() -> Self {
        Self {
            color_meshes: HashMap::new(),
            texture_meshes: HashMap::new(),
        }
    }

    /// Returns a trait object representing the [`Mesh`] with
    /// the given ID, or [`None`] if the mesh is not present.
    pub fn get_mesh(&self, mesh_id: MeshID) -> Option<&dyn Mesh<F>> {
        match self.texture_meshes.get(&mesh_id) {
            Some(mesh) => Some(mesh),
            None => match self.color_meshes.get(&mesh_id) {
                Some(mesh) => Some(mesh),
                None => None,
            },
        }
    }

    /// Returns a reference to the [`HashMap`] storing all
    /// color meshes.
    pub fn color_meshes(&self) -> &HashMap<MeshID, TriangleMesh<ColorVertex<F>>> {
        &self.color_meshes
    }

    /// Returns a reference to the [`HashMap`] storing all
    /// texture meshes.
    pub fn texture_meshes(&self) -> &HashMap<MeshID, TriangleMesh<TextureVertex<F>>> {
        &self.texture_meshes
    }

    /// Includes the given color mesh in the repository
    /// under the given ID.
    ///
    /// # Errors
    /// Returns an error if a mesh with the given ID already
    /// exists. The repository will remain unchanged.
    pub fn add_color_mesh(
        &mut self,
        mesh_id: MeshID,
        mesh: TriangleMesh<ColorVertex<F>>,
    ) -> Result<()> {
        if self.texture_meshes().contains_key(&mesh_id) {
            bail!(
                "Mesh {} already present in repository as a texture mesh",
                mesh_id
            )
        }

        match self.color_meshes.entry(mesh_id) {
            Entry::Vacant(entry) => {
                entry.insert(mesh);
                Ok(())
            }
            Entry::Occupied(_) => Err(anyhow!(
                "Mesh {} already present in repository as a color mesh",
                mesh_id
            )),
        }
    }

    /// Includes the given texture mesh in the repository
    /// under the given ID.
    ///
    /// # Errors
    /// Returns an error if a mesh with the given ID already
    /// exists. The repository will remain unchanged.
    pub fn add_texture_mesh(
        &mut self,
        mesh_id: MeshID,
        mesh: TriangleMesh<TextureVertex<F>>,
    ) -> Result<()> {
        if self.color_meshes().contains_key(&mesh_id) {
            bail!(
                "Mesh {} already present in repository as a color mesh",
                mesh_id
            )
        }

        match self.texture_meshes.entry(mesh_id) {
            Entry::Vacant(entry) => {
                entry.insert(mesh);
                Ok(())
            }
            Entry::Occupied(_) => Err(anyhow!(
                "Mesh {} already present in repository as a texture mesh",
                mesh_id
            )),
        }
    }
}
