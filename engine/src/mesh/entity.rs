//! Management of meshes for entities.

use anyhow::Result;
use impact_ecs::{archetype::ArchetypeComponentStorage, setup};
use impact_material::{MaterialHandle, MaterialLibrary};
use impact_mesh::{
    MeshRepository, TriangleMeshHandle,
    setup::{
        self, BoxMesh, CircularFrustumMesh, ConeMesh, CylinderMesh, HemisphereMesh, RectangleMesh,
        SphereMesh,
    },
    texture_projection::PlanarTextureProjection,
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
        |rectangle_mesh: &RectangleMesh,
         planar_projection: Option<&setup::PlanarTextureProjection>|
         -> TriangleMeshHandle {
            match (planar_projection,) {
                (Some(planar_projection),) => setup::setup_rectangle_mesh(
                    &mut mesh_repository,
                    rectangle_mesh,
                    Some(&planar_projection.create()),
                    desynchronized,
                ),
                (None,) => setup::setup_rectangle_mesh(
                    &mut mesh_repository,
                    rectangle_mesh,
                    Option::<&PlanarTextureProjection<_>>::None,
                    desynchronized,
                ),
            }
        },
        ![TriangleMeshHandle]
    );

    setup!(
        {
            let mut mesh_repository = mesh_repository.write().unwrap();
        },
        components,
        |box_mesh: &BoxMesh,
         planar_projection: Option<&setup::PlanarTextureProjection>|
         -> TriangleMeshHandle {
            match (planar_projection,) {
                (Some(planar_projection),) => setup::setup_box_mesh(
                    &mut mesh_repository,
                    box_mesh,
                    Some(&planar_projection.create()),
                    desynchronized,
                ),
                (None,) => setup::setup_box_mesh(
                    &mut mesh_repository,
                    box_mesh,
                    Option::<&PlanarTextureProjection<_>>::None,
                    desynchronized,
                ),
            }
        },
        ![TriangleMeshHandle]
    );

    setup!(
        {
            let mut mesh_repository = mesh_repository.write().unwrap();
        },
        components,
        |cylinder_mesh: &CylinderMesh,
         planar_projection: Option<&setup::PlanarTextureProjection>|
         -> TriangleMeshHandle {
            match (planar_projection,) {
                (Some(planar_projection),) => setup::setup_cylinder_mesh(
                    &mut mesh_repository,
                    cylinder_mesh,
                    Some(&planar_projection.create()),
                    desynchronized,
                ),
                (None,) => setup::setup_cylinder_mesh(
                    &mut mesh_repository,
                    cylinder_mesh,
                    Option::<&PlanarTextureProjection<_>>::None,
                    desynchronized,
                ),
            }
        },
        ![TriangleMeshHandle]
    );

    setup!(
        {
            let mut mesh_repository = mesh_repository.write().unwrap();
        },
        components,
        |cone_mesh: &ConeMesh,
         planar_projection: Option<&setup::PlanarTextureProjection>|
         -> TriangleMeshHandle {
            match (planar_projection,) {
                (Some(planar_projection),) => setup::setup_cone_mesh(
                    &mut mesh_repository,
                    cone_mesh,
                    Some(&planar_projection.create()),
                    desynchronized,
                ),
                (None,) => setup::setup_cone_mesh(
                    &mut mesh_repository,
                    cone_mesh,
                    Option::<&PlanarTextureProjection<_>>::None,
                    desynchronized,
                ),
            }
        },
        ![TriangleMeshHandle]
    );

    setup!(
        {
            let mut mesh_repository = mesh_repository.write().unwrap();
        },
        components,
        |circular_frustum_mesh: &CircularFrustumMesh,
         planar_projection: Option<&setup::PlanarTextureProjection>|
         -> TriangleMeshHandle {
            match (planar_projection,) {
                (Some(planar_projection),) => setup::setup_circular_frustum_mesh(
                    &mut mesh_repository,
                    circular_frustum_mesh,
                    Some(&planar_projection.create()),
                    desynchronized,
                ),
                (None,) => setup::setup_circular_frustum_mesh(
                    &mut mesh_repository,
                    circular_frustum_mesh,
                    Option::<&PlanarTextureProjection<_>>::None,
                    desynchronized,
                ),
            }
        },
        ![TriangleMeshHandle]
    );

    setup!(
        {
            let mut mesh_repository = mesh_repository.write().unwrap();
        },
        components,
        |sphere_mesh: &SphereMesh,
         planar_projection: Option<&setup::PlanarTextureProjection>|
         -> TriangleMeshHandle {
            match (planar_projection,) {
                (Some(planar_projection),) => setup::setup_sphere_mesh(
                    &mut mesh_repository,
                    sphere_mesh,
                    Some(&planar_projection.create()),
                    desynchronized,
                ),
                (None,) => setup::setup_sphere_mesh(
                    &mut mesh_repository,
                    sphere_mesh,
                    Option::<&PlanarTextureProjection<_>>::None,
                    desynchronized,
                ),
            }
        },
        ![TriangleMeshHandle]
    );

    setup!(
        {
            let mut mesh_repository = mesh_repository.write().unwrap();
        },
        components,
        |hemisphere_mesh: &HemisphereMesh,
         planar_projection: Option<&setup::PlanarTextureProjection>|
         -> TriangleMeshHandle {
            match (planar_projection,) {
                (Some(planar_projection),) => setup::setup_hemisphere_mesh(
                    &mut mesh_repository,
                    hemisphere_mesh,
                    Some(&planar_projection.create()),
                    desynchronized,
                ),
                (None,) => setup::setup_hemisphere_mesh(
                    &mut mesh_repository,
                    hemisphere_mesh,
                    Option::<&PlanarTextureProjection<_>>::None,
                    desynchronized,
                ),
            }
        },
        ![TriangleMeshHandle]
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
        |mesh: &TriangleMeshHandle, material: &MaterialHandle| {
            let Some(material_specification) =
                material_library.get_material_specification(material.material_id())
            else {
                impact_log::warn!(
                    "Tried to generate missing vertex properties for missing material {}",
                    material.material_id()
                );
                return;
            };

            let vertex_attribute_requirements =
                material_specification.vertex_attribute_requirements();

            setup::generate_missing_vertex_properties_for_mesh(
                &mut mesh_repository,
                mesh.id,
                vertex_attribute_requirements,
            );
        }
    );
}
