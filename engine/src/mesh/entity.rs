//! Management of meshes for entities.

use anyhow::Result;
use impact_ecs::{archetype::ArchetypeComponentStorage, setup};
use impact_material::{MaterialLibrary, components::MaterialComp};
use impact_mesh::{
    MeshRepository,
    components::TriangleMeshComp,
    components::{
        BoxMeshComp, CircularFrustumMeshComp, ConeMeshComp, CylinderMeshComp, HemisphereMeshComp,
        RectangleMeshComp, SphereMeshComp,
    },
    entity,
    texture_projection::{PlanarTextureProjection, components::PlanarTextureProjectionComp},
};
use std::sync::RwLock;

/// Checks if the entity-to-be with the given components has a component
/// representing a mesh, and if so, generates the mesh and adds it to the
/// mesh repository if not present, then adds the appropriate mesh component
/// to the entity.
pub fn setup_mesh_for_new_entity(
    mesh_repository: &RwLock<MeshRepository>,
    components: &mut ArchetypeComponentStorage,
    desynchronized: &mut bool,
) -> Result<()> {
    setup!(
        {
            let mut mesh_repository = mesh_repository.write().unwrap();
        },
        components,
        |rectangle_mesh: &RectangleMeshComp,
         planar_projection: Option<&PlanarTextureProjectionComp>|
         -> TriangleMeshComp {
            match (planar_projection,) {
                (Some(planar_projection),) => entity::setup_rectangle_mesh(
                    &mut mesh_repository,
                    rectangle_mesh,
                    Some(&planar_projection.create_projection()),
                    desynchronized,
                ),
                (None,) => entity::setup_rectangle_mesh(
                    &mut mesh_repository,
                    rectangle_mesh,
                    Option::<&PlanarTextureProjection<_>>::None,
                    desynchronized,
                ),
            }
        },
        ![TriangleMeshComp]
    );

    setup!(
        {
            let mut mesh_repository = mesh_repository.write().unwrap();
        },
        components,
        |box_mesh: &BoxMeshComp,
         planar_projection: Option<&PlanarTextureProjectionComp>|
         -> TriangleMeshComp {
            match (planar_projection,) {
                (Some(planar_projection),) => entity::setup_box_mesh(
                    &mut mesh_repository,
                    box_mesh,
                    Some(&planar_projection.create_projection()),
                    desynchronized,
                ),
                (None,) => entity::setup_box_mesh(
                    &mut mesh_repository,
                    box_mesh,
                    Option::<&PlanarTextureProjection<_>>::None,
                    desynchronized,
                ),
            }
        },
        ![TriangleMeshComp]
    );

    setup!(
        {
            let mut mesh_repository = mesh_repository.write().unwrap();
        },
        components,
        |cylinder_mesh: &CylinderMeshComp,
         planar_projection: Option<&PlanarTextureProjectionComp>|
         -> TriangleMeshComp {
            match (planar_projection,) {
                (Some(planar_projection),) => entity::setup_cylinder_mesh(
                    &mut mesh_repository,
                    cylinder_mesh,
                    Some(&planar_projection.create_projection()),
                    desynchronized,
                ),
                (None,) => entity::setup_cylinder_mesh(
                    &mut mesh_repository,
                    cylinder_mesh,
                    Option::<&PlanarTextureProjection<_>>::None,
                    desynchronized,
                ),
            }
        },
        ![TriangleMeshComp]
    );

    setup!(
        {
            let mut mesh_repository = mesh_repository.write().unwrap();
        },
        components,
        |cone_mesh: &ConeMeshComp,
         planar_projection: Option<&PlanarTextureProjectionComp>|
         -> TriangleMeshComp {
            match (planar_projection,) {
                (Some(planar_projection),) => entity::setup_cone_mesh(
                    &mut mesh_repository,
                    cone_mesh,
                    Some(&planar_projection.create_projection()),
                    desynchronized,
                ),
                (None,) => entity::setup_cone_mesh(
                    &mut mesh_repository,
                    cone_mesh,
                    Option::<&PlanarTextureProjection<_>>::None,
                    desynchronized,
                ),
            }
        },
        ![TriangleMeshComp]
    );

    setup!(
        {
            let mut mesh_repository = mesh_repository.write().unwrap();
        },
        components,
        |circular_frustum_mesh: &CircularFrustumMeshComp,
         planar_projection: Option<&PlanarTextureProjectionComp>|
         -> TriangleMeshComp {
            match (planar_projection,) {
                (Some(planar_projection),) => entity::setup_circular_frustum_mesh(
                    &mut mesh_repository,
                    circular_frustum_mesh,
                    Some(&planar_projection.create_projection()),
                    desynchronized,
                ),
                (None,) => entity::setup_circular_frustum_mesh(
                    &mut mesh_repository,
                    circular_frustum_mesh,
                    Option::<&PlanarTextureProjection<_>>::None,
                    desynchronized,
                ),
            }
        },
        ![TriangleMeshComp]
    );

    setup!(
        {
            let mut mesh_repository = mesh_repository.write().unwrap();
        },
        components,
        |sphere_mesh: &SphereMeshComp,
         planar_projection: Option<&PlanarTextureProjectionComp>|
         -> TriangleMeshComp {
            match (planar_projection,) {
                (Some(planar_projection),) => entity::setup_sphere_mesh(
                    &mut mesh_repository,
                    sphere_mesh,
                    Some(&planar_projection.create_projection()),
                    desynchronized,
                ),
                (None,) => entity::setup_sphere_mesh(
                    &mut mesh_repository,
                    sphere_mesh,
                    Option::<&PlanarTextureProjection<_>>::None,
                    desynchronized,
                ),
            }
        },
        ![TriangleMeshComp]
    );

    setup!(
        {
            let mut mesh_repository = mesh_repository.write().unwrap();
        },
        components,
        |hemisphere_mesh: &HemisphereMeshComp,
         planar_projection: Option<&PlanarTextureProjectionComp>|
         -> TriangleMeshComp {
            match (planar_projection,) {
                (Some(planar_projection),) => entity::setup_hemisphere_mesh(
                    &mut mesh_repository,
                    hemisphere_mesh,
                    Some(&planar_projection.create_projection()),
                    desynchronized,
                ),
                (None,) => entity::setup_hemisphere_mesh(
                    &mut mesh_repository,
                    hemisphere_mesh,
                    Option::<&PlanarTextureProjection<_>>::None,
                    desynchronized,
                ),
            }
        },
        ![TriangleMeshComp]
    );

    Ok(())
}

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
        {
            let mut mesh_repository = mesh_repository.write().unwrap();
        },
        components,
        |mesh: &TriangleMeshComp, material: &MaterialComp| {
            let material_specification = material_library
                .get_material_specification(material.material_handle().material_id())
                .expect("Missing material in library for material component");

            let vertex_attribute_requirements =
                material_specification.vertex_attribute_requirements();

            entity::generate_missing_vertex_properties_for_mesh(
                &mut mesh_repository,
                mesh.id,
                vertex_attribute_requirements,
            );
        }
    );
}
