//! Management of meshes.

use crate::{geometry::TriangleMesh, num::Float};
use anyhow::{anyhow, Result};
use impact_utils::{hash64, stringhash64_newtype};
use std::{
    collections::{hash_map::Entry, HashMap},
    fmt::Debug,
};

stringhash64_newtype!(
    /// Identifier for specific meshes.
    /// Wraps a [`StringHash64`](impact_utils::StringHash64).
    [pub] MeshID
);

/// Repository where [`TriangleMesh`]es are stored under a
/// unique [`MeshID`].
#[derive(Debug, Default)]
pub struct MeshRepository<F: Float> {
    meshes: HashMap<MeshID, TriangleMesh<F>>,
}

impl<F: Float> MeshRepository<F> {
    /// Creates a new empty mesh repository.
    pub fn new() -> Self {
        Self {
            meshes: HashMap::new(),
        }
    }

    /// Returns a reference to the [`TriangleMesh`] with the given ID, or
    /// [`None`] if the mesh is not present.
    pub fn get_mesh(&self, mesh_id: MeshID) -> Option<&TriangleMesh<F>> {
        self.meshes.get(&mesh_id)
    }

    /// Returns a reference to the [`HashMap`] storing all meshes.
    pub fn meshes(&self) -> &HashMap<MeshID, TriangleMesh<F>> {
        &self.meshes
    }

    /// Includes the given mesh in the repository under the given ID.
    ///
    /// # Errors
    /// Returns an error if a mesh with the given ID already exists. The
    /// repository will remain unchanged.
    pub fn add_mesh(&mut self, mesh_id: MeshID, mesh: TriangleMesh<F>) -> Result<()> {
        match self.meshes.entry(mesh_id) {
            Entry::Vacant(entry) => {
                entry.insert(mesh);
                Ok(())
            }
            Entry::Occupied(_) => Err(anyhow!("Mesh {} already present in repository", mesh_id)),
        }
    }

    /// Includes the given mesh in the repository under an ID derived from the
    /// given name.
    ///
    /// # Returns
    /// The ID assigned to the mesh.
    ///
    /// # Errors
    /// Returns an error if a mesh with the given name already exists. The
    /// repository will remain unchanged.
    pub fn add_named_mesh(
        &mut self,
        name: impl AsRef<str>,
        mesh: TriangleMesh<F>,
    ) -> Result<MeshID> {
        let mesh_id = MeshID(hash64!(name.as_ref()));
        self.add_mesh(mesh_id, mesh)?;
        Ok(mesh_id)
    }

    /// Includes the given mesh in the repository under an ID derived from the
    /// given name, unless a mesh with the same ID is already present.
    pub fn add_named_mesh_unless_present(
        &mut self,
        name: impl AsRef<str>,
        mesh: TriangleMesh<F>,
    ) -> MeshID {
        let mesh_id = MeshID(hash64!(name.as_ref()));
        let _ = self.add_mesh(mesh_id, mesh);
        mesh_id
    }
}
