//! Management of meshes.

mod components;

pub use components::{BoxMeshComp, CylinderMeshComp, MeshComp, PlaneMeshComp, SphereMeshComp};

use crate::{
    geometry::{TriangleMesh, VertexAttributeSet},
    num::Float,
    rendering::fre,
    scene::{MaterialComp, MaterialLibrary, RenderResourcesDesynchronized},
};
use anyhow::{anyhow, Result};
use impact_ecs::{archetype::ArchetypeComponentStorage, setup};
use impact_utils::stringhash64_newtype;
use std::{
    collections::{hash_map::Entry, HashMap},
    fmt::Debug,
    sync::RwLock,
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

    /// Returns a mutable reference to the [`TriangleMesh`] with the given ID,
    /// or [`None`] if the mesh is not present.
    pub fn get_mesh_mut(&mut self, mesh_id: MeshID) -> Option<&mut TriangleMesh<F>> {
        self.meshes.get_mut(&mesh_id)
    }

    /// Whether a mesh with the given ID exists in the repository.
    pub fn has_mesh(&self, mesh_id: MeshID) -> bool {
        self.meshes.contains_key(&mesh_id)
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

    /// Includes the given mesh in the repository under the given ID, unless a
    /// mesh with the same ID is already present.
    pub fn add_mesh_unless_present(&mut self, mesh_id: MeshID, mesh: TriangleMesh<F>) {
        let _ = self.add_mesh(mesh_id, mesh);
    }
}

impl TriangleMesh<fre> {
    /// Checks if the entity-to-be with the given components has a component
    /// representing a mesh, and if so, adds the appropriate material property
    /// generates or loads the mesh and adds it to the mesh repository if not
    /// present, then adds the appropriate mesh component to the entity.
    pub fn add_mesh_component_for_entity(
        mesh_repository: &RwLock<MeshRepository<fre>>,
        components: &mut ArchetypeComponentStorage,
        desynchronized: &mut RenderResourcesDesynchronized,
    ) -> Result<()> {
        setup!(
            components,
            |plane_mesh: &PlaneMeshComp| -> MeshComp {
                let mesh_id = plane_mesh.generate_id();

                if !mesh_repository.read().unwrap().has_mesh(mesh_id) {
                    let mesh = Self::create_plane(plane_mesh.extent_x, plane_mesh.extent_z);

                    mesh_repository
                        .write()
                        .unwrap()
                        .add_mesh_unless_present(mesh_id, mesh);

                    desynchronized.set_yes();
                }

                MeshComp::new(mesh_id)
            },
            ![MeshComp]
        );

        setup!(
            components,
            |box_mesh: &BoxMeshComp| -> MeshComp {
                let mesh_id = box_mesh.generate_id();

                if !mesh_repository.read().unwrap().has_mesh(mesh_id) {
                    let mesh =
                        Self::create_box(box_mesh.extent_x, box_mesh.extent_y, box_mesh.extent_z);

                    mesh_repository
                        .write()
                        .unwrap()
                        .add_mesh_unless_present(mesh_id, mesh);

                    desynchronized.set_yes();
                }

                MeshComp::new(mesh_id)
            },
            ![MeshComp]
        );

        setup!(
            components,
            |cylinder_mesh: &CylinderMeshComp| -> MeshComp {
                let mesh_id = cylinder_mesh.generate_id();

                if !mesh_repository.read().unwrap().has_mesh(mesh_id) {
                    let mesh = Self::create_cylinder(
                        cylinder_mesh.extent_y,
                        cylinder_mesh.diameter,
                        cylinder_mesh.n_circumference_vertices as usize,
                    );

                    mesh_repository
                        .write()
                        .unwrap()
                        .add_mesh_unless_present(mesh_id, mesh);

                    desynchronized.set_yes();
                }

                MeshComp::new(mesh_id)
            },
            ![MeshComp]
        );

        setup!(
            components,
            |sphere_mesh: &SphereMeshComp| -> MeshComp {
                let mesh_id = sphere_mesh.generate_id();

                if !mesh_repository.read().unwrap().has_mesh(mesh_id) {
                    let mesh = Self::create_sphere(sphere_mesh.n_rings as usize);

                    mesh_repository
                        .write()
                        .unwrap()
                        .add_mesh_unless_present(mesh_id, mesh);

                    desynchronized.set_yes();
                }

                MeshComp::new(mesh_id)
            },
            ![MeshComp]
        );

        Ok(())
    }

    /// Checks if the entity-to-be with the given components has a material
    /// component and a component for a mesh that misses vertex attributes
    /// required by the material, and if so, generates the missing vertex
    /// attributes if possible.
    pub fn generate_missing_vertex_properties(
        mesh_repository: &RwLock<MeshRepository<fre>>,
        material_library: &MaterialLibrary,
        components: &ArchetypeComponentStorage,
    ) {
        setup!(components, |mesh: &MeshComp, material: &MaterialComp| {
            let material_specification = material_library
                .get_material_specification(material.material_id)
                .expect("Missing material in library for material component");

            let vertex_attribute_requirements =
                material_specification.vertex_attribute_requirements();

            if vertex_attribute_requirements.contains(VertexAttributeSet::NORMAL_VECTOR) {
                let mesh_repository_readonly = mesh_repository.read().unwrap();
                let mesh_readonly = mesh_repository_readonly
                    .get_mesh(mesh.id)
                    .expect("Missing mesh in repository for mesh component");

                if !mesh_readonly.has_normal_vectors() {
                    log::info!("Generating normal vectors for mesh {}", mesh.id);

                    drop(mesh_repository_readonly); // Release read lock
                    let mut mesh_repository_writable = mesh_repository.write().unwrap();

                    mesh_repository_writable
                        .get_mesh_mut(mesh.id)
                        .unwrap()
                        .generate_smooth_normal_vectors();
                }
            }
        });
    }
}
