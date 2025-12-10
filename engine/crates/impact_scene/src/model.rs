//! Models defined by a mesh and material.

use bytemuck::Zeroable;
use impact_material::MaterialID;
use impact_math::hash::{Hash64, compute_hash_64_of_two_hash_64};
use impact_mesh::{LineSegmentMeshID, MeshID, TriangleMeshID};
use std::{
    cmp, fmt,
    hash::{Hash, Hasher},
};

/// Identifier for specific models.
///
/// A model is uniquely defined by its mesh and material.
#[derive(Copy, Clone, Debug)]
pub struct ModelID {
    mesh_id: MeshID,
    material_id: MaterialID,
    hash: Hash64,
}

pub type ModelInstanceManager = impact_model::ModelInstanceManager<ModelID>;
pub type ModelInstanceManagerState = impact_model::ModelInstanceManagerState<ModelID>;

pub type ModelInstanceGPUBufferMap = impact_model::gpu_resource::ModelInstanceGPUBufferMap<ModelID>;

impl ModelID {
    /// Creates a new [`ModelID`] for the model comprised of the given triangle
    /// mesh and material.
    pub fn for_triangle_mesh_and_material(
        mesh_id: TriangleMeshID,
        material_id: MaterialID,
    ) -> Self {
        let hash = compute_hash_64_of_two_hash_64(mesh_id.0.hash(), material_id.0.hash());
        Self {
            mesh_id: MeshID::Triangle(mesh_id),
            material_id,
            hash,
        }
    }

    /// Creates a new [`ModelID`] for the model comprised of the given line
    /// segment mesh and material.
    pub fn for_line_segment_mesh_and_material(
        mesh_id: LineSegmentMeshID,
        material_id: MaterialID,
    ) -> Self {
        let hash = compute_hash_64_of_two_hash_64(mesh_id.0.hash(), material_id.0.hash());
        Self {
            mesh_id: MeshID::LineSegment(mesh_id),
            material_id,
            hash,
        }
    }

    /// Creates a new [`ModelID`] with the given hash. The [`ModelID::mesh_id`]
    /// and [`ModelID::material_id`] methods on this `ModelID` will return dummy
    /// values.
    pub fn hash_only(hash: Hash64) -> Self {
        Self {
            mesh_id: MeshID::Triangle(TriangleMeshID::zeroed()),
            material_id: MaterialID::not_applicable(),
            hash,
        }
    }

    /// The ID of the model's mesh.
    pub fn mesh_id(&self) -> MeshID {
        self.mesh_id
    }

    /// The ID of the model's triangle mesh.
    ///
    /// # Panics
    /// If the mesh is not a triangle mesh.
    pub fn triangle_mesh_id(&self) -> TriangleMeshID {
        match self.mesh_id {
            MeshID::Triangle(id) => id,
            MeshID::LineSegment(_) => {
                panic!("Got line segment mesh when expecting triangle mesh in `ModelID`")
            }
        }
    }

    /// The ID of the model's line segment mesh.
    ///
    /// # Panics
    /// If the mesh is not a line segment mesh.
    pub fn line_segment_mesh_id(&self) -> LineSegmentMeshID {
        match self.mesh_id {
            MeshID::LineSegment(id) => id,
            MeshID::Triangle(_) => {
                panic!("Got triangle mesh when expecting line segment mesh in `ModelID`")
            }
        }
    }

    /// The ID of the model's material.
    pub fn material_id(&self) -> MaterialID {
        self.material_id
    }
}

impl fmt::Display for ModelID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{{mesh: {}, material: {}}}",
            self.mesh_id, self.material_id,
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
