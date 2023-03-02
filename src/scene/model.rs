//! Management of models.

use crate::scene::{MaterialID, MaterialPropertyTextureSetID, MeshID};
use impact_utils::{self, Hash64};
use std::{
    cmp,
    fmt::{self, Debug},
    hash::{Hash, Hasher},
};

/// Identifier for specific models.
///
/// A model is uniquely defined by its mesh, material type and texture set.
#[derive(Copy, Clone, Debug)]
pub struct ModelID {
    mesh_id: MeshID,
    material_id: MaterialID,
    material_property_texture_set_id: Option<MaterialPropertyTextureSetID>,
    hash: Hash64,
}

impl ModelID {
    /// Creates a new [`ModelID`] for the model comprised of the mesh, material
    /// and, optionally, material property texture set with the given IDs.
    pub fn for_mesh_and_material(
        mesh_id: MeshID,
        material_id: MaterialID,
        material_property_texture_set_id: Option<MaterialPropertyTextureSetID>,
    ) -> Self {
        let mut hash =
            impact_utils::compute_hash_64_of_two_hash_64(mesh_id.0.hash(), material_id.0.hash());

        if let Some(texture_set_id) = material_property_texture_set_id {
            hash = impact_utils::compute_hash_64_of_two_hash_64(hash, texture_set_id.0.hash());
        }

        Self {
            mesh_id,
            material_id,
            material_property_texture_set_id,
            hash,
        }
    }

    /// The ID of the model's mesh.
    pub fn mesh_id(&self) -> MeshID {
        self.mesh_id
    }

    /// The ID of the model's material.
    pub fn material_id(&self) -> MaterialID {
        self.material_id
    }

    /// The ID of the model's material propery texture set, or [`None`] if no
    /// material properties are textured.
    pub fn material_property_texture_set_id(&self) -> Option<MaterialPropertyTextureSetID> {
        self.material_property_texture_set_id
    }
}

impl fmt::Display for ModelID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{{mesh: {}, material: {}}}",
            self.mesh_id, self.material_id
        )
    }
}

impl PartialEq for ModelID {
    fn eq(&self, other: &Self) -> bool {
        self.hash.eq(&other.hash)
    }
}

impl Eq for ModelID {}

impl Ord for ModelID {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.hash.cmp(&other.hash)
    }
}

impl PartialOrd for ModelID {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Hash for ModelID {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.hash.hash(state);
    }
}
