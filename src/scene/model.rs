//! Management of models.

use crate::scene::{MaterialHandle, MeshID};
use impact_utils::{self, Hash64};
use std::{
    cmp,
    fmt::{self, Debug},
    hash::{Hash, Hasher},
};

/// Identifier for specific models.
///
/// A model is uniquely defined by its mesh and material. If the material has an
/// associated prepass material, that will also be part of the model definition.
#[derive(Copy, Clone, Debug)]
pub struct ModelID {
    mesh_id: MeshID,
    material_handle: MaterialHandle,
    prepass_material_handle: Option<MaterialHandle>,
    hash: Hash64,
}

impl ModelID {
    /// Creates a new [`ModelID`] for the model comprised of the mesh and
    /// material with an optional prepass material.
    pub fn for_mesh_and_material(
        mesh_id: MeshID,
        material_handle: MaterialHandle,
        prepass_material_handle: Option<MaterialHandle>,
    ) -> Self {
        let mut hash = impact_utils::compute_hash_64_of_two_hash_64(
            mesh_id.0.hash(),
            material_handle.compute_hash(),
        );

        if let Some(prepass_material_handle) = prepass_material_handle {
            hash = impact_utils::compute_hash_64_of_two_hash_64(
                hash,
                prepass_material_handle.compute_hash(),
            );
        }

        Self {
            mesh_id,
            material_handle,
            prepass_material_handle,
            hash,
        }
    }

    /// The ID of the model's mesh.
    pub fn mesh_id(&self) -> MeshID {
        self.mesh_id
    }

    /// The handle for the model's material.
    pub fn material_handle(&self) -> &MaterialHandle {
        &self.material_handle
    }

    /// The handle for the prepass material associated with the model's
    /// material.
    pub fn prepass_material_handle(&self) -> Option<&MaterialHandle> {
        self.prepass_material_handle.as_ref()
    }
}

impl fmt::Display for ModelID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{{mesh: {}, material: {:?}, prepass_material: {:?}}}",
            self.mesh_id, &self.material_handle, &self.prepass_material_handle
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
