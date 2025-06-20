//! Management of meshes for entities.

use crate::{
    material::{MaterialLibrary, components::MaterialComp},
    mesh::{
        MeshRepository, TriangleMesh, VertexAttributeSet,
        components::{
            BoxMeshComp, CircularFrustumMeshComp, ConeMeshComp, CylinderMeshComp,
            HemisphereMeshComp, RectangleMeshComp, SphereMeshComp, TriangleMeshComp,
        },
        texture_projection::{
            PlanarTextureProjection, TextureProjection, components::PlanarTextureProjectionComp,
        },
    },
    scene::RenderResourcesDesynchronized,
};
use anyhow::Result;
use impact_ecs::{archetype::ArchetypeComponentStorage, setup};
use std::sync::RwLock;

/// Checks if the entity-to-be with the given components has a component
/// representing a mesh, and if so, generates the mesh and adds it to the
/// mesh repository if not present, then adds the appropriate mesh component
/// to the entity.
pub fn setup_mesh_for_new_entity(
    mesh_repository: &RwLock<MeshRepository>,
    components: &mut ArchetypeComponentStorage,
    desynchronized: &mut RenderResourcesDesynchronized,
) -> Result<()> {
    fn create_projection_label(projection: Option<&impl TextureProjection<f32>>) -> String {
        projection
            .as_ref()
            .map_or("None".to_string(), |projection| projection.identifier())
    }

    fn execute_setup_for_rectangle_mesh(
        mesh_repository: &RwLock<MeshRepository>,
        desynchronized: &mut RenderResourcesDesynchronized,
        rectangle_mesh: &RectangleMeshComp,
        projection: Option<&impl TextureProjection<f32>>,
    ) -> TriangleMeshComp {
        let mesh_id = rectangle_mesh.generate_id(create_projection_label(projection));

        if !mesh_repository.read().unwrap().has_triangle_mesh(mesh_id) {
            let mut mesh =
                TriangleMesh::create_rectangle(rectangle_mesh.extent_x, rectangle_mesh.extent_z);

            if let Some(projection) = projection {
                mesh.generate_texture_coords(projection);
            }

            mesh_repository
                .write()
                .unwrap()
                .add_triangle_mesh_unless_present(mesh_id, mesh);

            desynchronized.set_yes();
        }

        TriangleMeshComp::new(mesh_id)
    }

    fn execute_setup_for_box_mesh(
        mesh_repository: &RwLock<MeshRepository>,
        desynchronized: &mut RenderResourcesDesynchronized,
        box_mesh: &BoxMeshComp,
        projection: Option<&impl TextureProjection<f32>>,
    ) -> TriangleMeshComp {
        let mesh_id = box_mesh.generate_id(create_projection_label(projection));

        if !mesh_repository.read().unwrap().has_triangle_mesh(mesh_id) {
            let mut mesh = TriangleMesh::create_box(
                box_mesh.extent_x,
                box_mesh.extent_y,
                box_mesh.extent_z,
                box_mesh.front_face_side(),
            );

            if let Some(projection) = projection {
                mesh.generate_texture_coords(projection);
            }

            mesh_repository
                .write()
                .unwrap()
                .add_triangle_mesh_unless_present(mesh_id, mesh);

            desynchronized.set_yes();
        }

        TriangleMeshComp::new(mesh_id)
    }

    fn execute_setup_for_cylinder_mesh(
        mesh_repository: &RwLock<MeshRepository>,
        desynchronized: &mut RenderResourcesDesynchronized,
        cylinder_mesh: &CylinderMeshComp,
        projection: Option<&impl TextureProjection<f32>>,
    ) -> TriangleMeshComp {
        let mesh_id = cylinder_mesh.generate_id(create_projection_label(projection));

        if !mesh_repository.read().unwrap().has_triangle_mesh(mesh_id) {
            let mut mesh = TriangleMesh::create_cylinder(
                cylinder_mesh.length,
                cylinder_mesh.diameter,
                cylinder_mesh.n_circumference_vertices as usize,
            );

            if let Some(projection) = projection {
                mesh.generate_texture_coords(projection);
            }

            mesh_repository
                .write()
                .unwrap()
                .add_triangle_mesh_unless_present(mesh_id, mesh);

            desynchronized.set_yes();
        }

        TriangleMeshComp::new(mesh_id)
    }

    fn execute_setup_for_cone_mesh(
        mesh_repository: &RwLock<MeshRepository>,
        desynchronized: &mut RenderResourcesDesynchronized,
        cone_mesh: &ConeMeshComp,
        projection: Option<&impl TextureProjection<f32>>,
    ) -> TriangleMeshComp {
        let mesh_id = cone_mesh.generate_id(create_projection_label(projection));

        if !mesh_repository.read().unwrap().has_triangle_mesh(mesh_id) {
            let mut mesh = TriangleMesh::create_cone(
                cone_mesh.length,
                cone_mesh.max_diameter,
                cone_mesh.n_circumference_vertices as usize,
            );

            if let Some(projection) = projection {
                mesh.generate_texture_coords(projection);
            }

            mesh_repository
                .write()
                .unwrap()
                .add_triangle_mesh_unless_present(mesh_id, mesh);

            desynchronized.set_yes();
        }

        TriangleMeshComp::new(mesh_id)
    }

    fn execute_setup_for_circular_frustum_mesh(
        mesh_repository: &RwLock<MeshRepository>,
        desynchronized: &mut RenderResourcesDesynchronized,
        circular_frustum_mesh: &CircularFrustumMeshComp,
        projection: Option<&impl TextureProjection<f32>>,
    ) -> TriangleMeshComp {
        let mesh_id = circular_frustum_mesh.generate_id(create_projection_label(projection));

        if !mesh_repository.read().unwrap().has_triangle_mesh(mesh_id) {
            let mut mesh = TriangleMesh::create_circular_frustum(
                circular_frustum_mesh.length,
                circular_frustum_mesh.bottom_diameter,
                circular_frustum_mesh.top_diameter,
                circular_frustum_mesh.n_circumference_vertices as usize,
            );

            if let Some(projection) = projection {
                mesh.generate_texture_coords(projection);
            }

            mesh_repository
                .write()
                .unwrap()
                .add_triangle_mesh_unless_present(mesh_id, mesh);

            desynchronized.set_yes();
        }

        TriangleMeshComp::new(mesh_id)
    }

    fn execute_setup_for_sphere_mesh(
        mesh_repository: &RwLock<MeshRepository>,
        desynchronized: &mut RenderResourcesDesynchronized,
        sphere_mesh: &SphereMeshComp,
        projection: Option<&impl TextureProjection<f32>>,
    ) -> TriangleMeshComp {
        let mesh_id = sphere_mesh.generate_id(create_projection_label(projection));

        if !mesh_repository.read().unwrap().has_triangle_mesh(mesh_id) {
            let mut mesh = TriangleMesh::create_sphere(sphere_mesh.n_rings as usize);

            if let Some(projection) = projection {
                mesh.generate_texture_coords(projection);
            }

            mesh_repository
                .write()
                .unwrap()
                .add_triangle_mesh_unless_present(mesh_id, mesh);

            desynchronized.set_yes();
        }

        TriangleMeshComp::new(mesh_id)
    }

    fn execute_setup_for_hemisphere_mesh(
        mesh_repository: &RwLock<MeshRepository>,
        desynchronized: &mut RenderResourcesDesynchronized,
        hemisphere_mesh: &HemisphereMeshComp,
        projection: Option<&impl TextureProjection<f32>>,
    ) -> TriangleMeshComp {
        let mesh_id = hemisphere_mesh.generate_id(create_projection_label(projection));

        if !mesh_repository.read().unwrap().has_triangle_mesh(mesh_id) {
            let mut mesh = TriangleMesh::create_hemisphere(hemisphere_mesh.n_rings as usize);

            if let Some(projection) = projection {
                mesh.generate_texture_coords(projection);
            }

            mesh_repository
                .write()
                .unwrap()
                .add_triangle_mesh_unless_present(mesh_id, mesh);

            desynchronized.set_yes();
        }

        TriangleMeshComp::new(mesh_id)
    }

    setup!(
        components,
        |rectangle_mesh: &RectangleMeshComp,
         planar_projection: Option<&PlanarTextureProjectionComp>|
         -> TriangleMeshComp {
            match (planar_projection,) {
                (Some(planar_projection),) => execute_setup_for_rectangle_mesh(
                    mesh_repository,
                    desynchronized,
                    rectangle_mesh,
                    Some(&planar_projection.create_projection()),
                ),
                (None,) => execute_setup_for_rectangle_mesh(
                    mesh_repository,
                    desynchronized,
                    rectangle_mesh,
                    Option::<&PlanarTextureProjection<_>>::None,
                ),
            }
        },
        ![TriangleMeshComp]
    );

    setup!(
        components,
        |box_mesh: &BoxMeshComp,
         planar_projection: Option<&PlanarTextureProjectionComp>|
         -> TriangleMeshComp {
            match (planar_projection,) {
                (Some(planar_projection),) => execute_setup_for_box_mesh(
                    mesh_repository,
                    desynchronized,
                    box_mesh,
                    Some(&planar_projection.create_projection()),
                ),
                (None,) => execute_setup_for_box_mesh(
                    mesh_repository,
                    desynchronized,
                    box_mesh,
                    Option::<&PlanarTextureProjection<_>>::None,
                ),
            }
        },
        ![TriangleMeshComp]
    );

    setup!(
        components,
        |cylinder_mesh: &CylinderMeshComp,
         planar_projection: Option<&PlanarTextureProjectionComp>|
         -> TriangleMeshComp {
            match (planar_projection,) {
                (Some(planar_projection),) => execute_setup_for_cylinder_mesh(
                    mesh_repository,
                    desynchronized,
                    cylinder_mesh,
                    Some(&planar_projection.create_projection()),
                ),
                (None,) => execute_setup_for_cylinder_mesh(
                    mesh_repository,
                    desynchronized,
                    cylinder_mesh,
                    Option::<&PlanarTextureProjection<_>>::None,
                ),
            }
        },
        ![TriangleMeshComp]
    );

    setup!(
        components,
        |cone_mesh: &ConeMeshComp,
         planar_projection: Option<&PlanarTextureProjectionComp>|
         -> TriangleMeshComp {
            match (planar_projection,) {
                (Some(planar_projection),) => execute_setup_for_cone_mesh(
                    mesh_repository,
                    desynchronized,
                    cone_mesh,
                    Some(&planar_projection.create_projection()),
                ),
                (None,) => execute_setup_for_cone_mesh(
                    mesh_repository,
                    desynchronized,
                    cone_mesh,
                    Option::<&PlanarTextureProjection<_>>::None,
                ),
            }
        },
        ![TriangleMeshComp]
    );

    setup!(
        components,
        |circular_frustum_mesh: &CircularFrustumMeshComp,
         planar_projection: Option<&PlanarTextureProjectionComp>|
         -> TriangleMeshComp {
            match (planar_projection,) {
                (Some(planar_projection),) => execute_setup_for_circular_frustum_mesh(
                    mesh_repository,
                    desynchronized,
                    circular_frustum_mesh,
                    Some(&planar_projection.create_projection()),
                ),
                (None,) => execute_setup_for_circular_frustum_mesh(
                    mesh_repository,
                    desynchronized,
                    circular_frustum_mesh,
                    Option::<&PlanarTextureProjection<_>>::None,
                ),
            }
        },
        ![TriangleMeshComp]
    );

    setup!(
        components,
        |sphere_mesh: &SphereMeshComp,
         planar_projection: Option<&PlanarTextureProjectionComp>|
         -> TriangleMeshComp {
            match (planar_projection,) {
                (Some(planar_projection),) => execute_setup_for_sphere_mesh(
                    mesh_repository,
                    desynchronized,
                    sphere_mesh,
                    Some(&planar_projection.create_projection()),
                ),
                (None,) => execute_setup_for_sphere_mesh(
                    mesh_repository,
                    desynchronized,
                    sphere_mesh,
                    Option::<&PlanarTextureProjection<_>>::None,
                ),
            }
        },
        ![TriangleMeshComp]
    );

    setup!(
        components,
        |hemisphere_mesh: &HemisphereMeshComp,
         planar_projection: Option<&PlanarTextureProjectionComp>|
         -> TriangleMeshComp {
            match (planar_projection,) {
                (Some(planar_projection),) => execute_setup_for_hemisphere_mesh(
                    mesh_repository,
                    desynchronized,
                    hemisphere_mesh,
                    Some(&planar_projection.create_projection()),
                ),
                (None,) => execute_setup_for_hemisphere_mesh(
                    mesh_repository,
                    desynchronized,
                    hemisphere_mesh,
                    Option::<&PlanarTextureProjection<_>>::None,
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
        components,
        |mesh: &TriangleMeshComp, material: &MaterialComp| {
            let material_specification = material_library
                .get_material_specification(material.material_handle().material_id())
                .expect("Missing material in library for material component");

            let vertex_attribute_requirements =
                material_specification.vertex_attribute_requirements();

            if vertex_attribute_requirements.contains(VertexAttributeSet::NORMAL_VECTOR) {
                let mesh_repository_readonly = mesh_repository.read().unwrap();
                let mesh_readonly = mesh_repository_readonly
                    .get_triangle_mesh(mesh.id)
                    .expect("Missing mesh in repository for mesh component");

                if !mesh_readonly.has_normal_vectors() {
                    log::info!("Generating normal vectors for mesh {}", mesh.id);

                    drop(mesh_repository_readonly); // Release read lock
                    let mut mesh_repository_writable = mesh_repository.write().unwrap();

                    mesh_repository_writable
                        .get_triangle_mesh_mut(mesh.id)
                        .unwrap()
                        .generate_smooth_normal_vectors();
                }
            }

            if vertex_attribute_requirements.contains(VertexAttributeSet::TANGENT_SPACE_QUATERNION)
            {
                let mesh_repository_readonly = mesh_repository.read().unwrap();
                let mesh_readonly = mesh_repository_readonly
                    .get_triangle_mesh(mesh.id)
                    .expect("Missing mesh in repository for mesh component");

                if !mesh_readonly.has_tangent_space_quaternions() {
                    log::info!("Generating tangent space quaternions for mesh {}", mesh.id);

                    drop(mesh_repository_readonly); // Release read lock
                    let mut mesh_repository_writable = mesh_repository.write().unwrap();

                    mesh_repository_writable
                        .get_triangle_mesh_mut(mesh.id)
                        .unwrap()
                        .generate_smooth_tangent_space_quaternions();
                }
            }
        }
    );
}
