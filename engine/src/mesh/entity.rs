//! Management of meshes for entities.

use impact_ecs::{archetype::ArchetypeComponentStorage, setup};
use impact_material::{MaterialLibrary, components::MaterialComp};
use impact_mesh::{MeshRepository, components::TriangleMeshComp, entity};
use std::sync::RwLock;

/// Checks if the entity-to-be with the given components has a material
/// component and a component for a mesh that misses vertex attributes
/// required by the material, and if so, generates the missing vertex
/// attributes if possible.
pub fn generate_missing_vertex_properties_for_new_entity_mesh(
    mesh_repository: &RwLock<MeshRepository>,
    material_library: &MaterialLibrary,
    components: &ArchetypeComponentStorage,
) {
    setup!(
        components,
        |mesh: &TriangleMeshComp, material: &MaterialComp| {
            let material_specification = material_library
                .get_material_specification(material.material_handle().material_id())
                .expect("Missing material in library for material component");

            let vertex_attribute_requirements =
                material_specification.vertex_attribute_requirements();

            entity::generate_missing_vertex_properties_for_mesh(
                mesh_repository,
                mesh.id,
                vertex_attribute_requirements,
            );
        }
    );
}
