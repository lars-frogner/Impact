//! Models defined by a mesh and material.

use impact_material::MaterialHandle;
use impact_math::Hash64;
use impact_mesh::{LineSegmentMeshHandle, MeshHandle, TriangleMeshHandle};
use std::{
    cmp, fmt,
    hash::{Hash, Hasher},
};

/// Identifier for specific models.
///
/// A model is uniquely defined by its mesh and material. If the material has an
/// associated prepass material, that will also be part of the model definition.
#[derive(Copy, Clone, Debug)]
pub struct ModelID {
    mesh_handle: MeshHandle,
    material_handle: MaterialHandle,
    hash: Hash64,
}

pub type InstanceFeatureManager = impact_model::InstanceFeatureManager<ModelID>;
pub type InstanceFeatureManagerState = impact_model::InstanceFeatureManagerState<ModelID>;

impl ModelID {
    /// Creates a new [`ModelID`] for the model comprised of the given triangle
    /// mesh and material.
    pub fn for_triangle_mesh_and_material(
        mesh_handle: TriangleMeshHandle,
        material_handle: MaterialHandle,
    ) -> Self {
        let hash = impact_math::compute_hash_64_of_two_hash_64(
            mesh_handle.compute_hash(),
            material_handle.compute_hash(),
        );
        Self {
            mesh_handle: MeshHandle::Triangle(mesh_handle),
            material_handle,
            hash,
        }
    }

    /// Creates a new [`ModelID`] for the model comprised of the given line
    /// segment mesh and material.
    pub fn for_line_segment_mesh_and_material(
        mesh_handle: LineSegmentMeshHandle,
        material_handle: MaterialHandle,
    ) -> Self {
        let hash = impact_math::compute_hash_64_of_two_hash_64(
            mesh_handle.compute_hash(),
            material_handle.compute_hash(),
        );
        Self {
            mesh_handle: MeshHandle::LineSegment(mesh_handle),
            material_handle,
            hash,
        }
    }

    /// Creates a new [`ModelID`] with the given hash. The
    /// [`ModelID::mesh_handle`] and [`ModelID::material_handle`] methods on
    /// this `ModelID` will return invalid dummy values.
    pub fn hash_only(hash: Hash64) -> Self {
        Self {
            mesh_handle: MeshHandle::Triangle(TriangleMeshHandle::dummy()),
            material_handle: MaterialHandle::not_applicable(),
            hash,
        }
    }

    /// The handle to the model's mesh.
    pub fn mesh_handle(&self) -> MeshHandle {
        self.mesh_handle
    }

    /// The handle to the model's triangle mesh.
    ///
    /// # Panics
    /// If the mesh is not a triangle mesh.
    pub fn triangle_mesh_handle(&self) -> TriangleMeshHandle {
        match self.mesh_handle {
            MeshHandle::Triangle(handle) => handle,
            MeshHandle::LineSegment(_) => {
                panic!("Got line segment mesh when expecting triangle mesh in `ModelID`")
            }
        }
    }

    /// The handle to the model's line segment mesh.
    ///
    /// # Panics
    /// If the mesh is not a line segment mesh.
    pub fn line_segment_mesh_handle(&self) -> LineSegmentMeshHandle {
        match self.mesh_handle {
            MeshHandle::LineSegment(handle) => handle,
            MeshHandle::Triangle(_) => {
                panic!("Got triangle mesh when expecting line segment mesh in `ModelID`")
            }
        }
    }

    /// The handle for the model's material.
    pub fn material_handle(&self) -> &MaterialHandle {
        &self.material_handle
    }
}

impl fmt::Display for ModelID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{{mesh: {}, material: {}}}",
            self.mesh_handle, self.material_handle,
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
