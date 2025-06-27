//! Management of meshes for entities.

use crate::{
    MeshID, MeshRepository, TriangleMesh, VertexAttributeSet,
    components::{
        BoxMeshComp, CircularFrustumMeshComp, ConeMeshComp, CylinderMeshComp, HemisphereMeshComp,
        RectangleMeshComp, SphereMeshComp, TriangleMeshComp,
    },
    texture_projection::TextureProjection,
};

pub fn setup_rectangle_mesh(
    mesh_repository: &mut MeshRepository,
    rectangle_mesh: &RectangleMeshComp,
    projection: Option<&impl TextureProjection<f32>>,
    desynchronized: &mut bool,
) -> TriangleMeshComp {
    let mesh_id = rectangle_mesh.generate_id(create_projection_label(projection));

    if !mesh_repository.has_triangle_mesh(mesh_id) {
        let mut mesh =
            TriangleMesh::create_rectangle(rectangle_mesh.extent_x, rectangle_mesh.extent_z);

        if let Some(projection) = projection {
            mesh.generate_texture_coords(projection);
        }

        mesh_repository.add_triangle_mesh_unless_present(mesh_id, mesh);

        *desynchronized = true;
    }

    TriangleMeshComp::new(mesh_id)
}

pub fn setup_box_mesh(
    mesh_repository: &mut MeshRepository,
    box_mesh: &BoxMeshComp,
    projection: Option<&impl TextureProjection<f32>>,
    desynchronized: &mut bool,
) -> TriangleMeshComp {
    let mesh_id = box_mesh.generate_id(create_projection_label(projection));

    if !mesh_repository.has_triangle_mesh(mesh_id) {
        let mut mesh = TriangleMesh::create_box(
            box_mesh.extent_x,
            box_mesh.extent_y,
            box_mesh.extent_z,
            box_mesh.front_face_side(),
        );

        if let Some(projection) = projection {
            mesh.generate_texture_coords(projection);
        }

        mesh_repository.add_triangle_mesh_unless_present(mesh_id, mesh);

        *desynchronized = true;
    }

    TriangleMeshComp::new(mesh_id)
}

pub fn setup_cylinder_mesh(
    mesh_repository: &mut MeshRepository,
    cylinder_mesh: &CylinderMeshComp,
    projection: Option<&impl TextureProjection<f32>>,
    desynchronized: &mut bool,
) -> TriangleMeshComp {
    let mesh_id = cylinder_mesh.generate_id(create_projection_label(projection));

    if !mesh_repository.has_triangle_mesh(mesh_id) {
        let mut mesh = TriangleMesh::create_cylinder(
            cylinder_mesh.length,
            cylinder_mesh.diameter,
            cylinder_mesh.n_circumference_vertices as usize,
        );

        if let Some(projection) = projection {
            mesh.generate_texture_coords(projection);
        }

        mesh_repository.add_triangle_mesh_unless_present(mesh_id, mesh);

        *desynchronized = true;
    }

    TriangleMeshComp::new(mesh_id)
}

pub fn setup_cone_mesh(
    mesh_repository: &mut MeshRepository,
    cone_mesh: &ConeMeshComp,
    projection: Option<&impl TextureProjection<f32>>,
    desynchronized: &mut bool,
) -> TriangleMeshComp {
    let mesh_id = cone_mesh.generate_id(create_projection_label(projection));

    if !mesh_repository.has_triangle_mesh(mesh_id) {
        let mut mesh = TriangleMesh::create_cone(
            cone_mesh.length,
            cone_mesh.max_diameter,
            cone_mesh.n_circumference_vertices as usize,
        );

        if let Some(projection) = projection {
            mesh.generate_texture_coords(projection);
        }

        mesh_repository.add_triangle_mesh_unless_present(mesh_id, mesh);

        *desynchronized = true;
    }

    TriangleMeshComp::new(mesh_id)
}

pub fn setup_circular_frustum_mesh(
    mesh_repository: &mut MeshRepository,
    circular_frustum_mesh: &CircularFrustumMeshComp,
    projection: Option<&impl TextureProjection<f32>>,
    desynchronized: &mut bool,
) -> TriangleMeshComp {
    let mesh_id = circular_frustum_mesh.generate_id(create_projection_label(projection));

    if !mesh_repository.has_triangle_mesh(mesh_id) {
        let mut mesh = TriangleMesh::create_circular_frustum(
            circular_frustum_mesh.length,
            circular_frustum_mesh.bottom_diameter,
            circular_frustum_mesh.top_diameter,
            circular_frustum_mesh.n_circumference_vertices as usize,
        );

        if let Some(projection) = projection {
            mesh.generate_texture_coords(projection);
        }

        mesh_repository.add_triangle_mesh_unless_present(mesh_id, mesh);

        *desynchronized = true;
    }

    TriangleMeshComp::new(mesh_id)
}

pub fn setup_sphere_mesh(
    mesh_repository: &mut MeshRepository,
    sphere_mesh: &SphereMeshComp,
    projection: Option<&impl TextureProjection<f32>>,
    desynchronized: &mut bool,
) -> TriangleMeshComp {
    let mesh_id = sphere_mesh.generate_id(create_projection_label(projection));

    if !mesh_repository.has_triangle_mesh(mesh_id) {
        let mut mesh = TriangleMesh::create_sphere(sphere_mesh.n_rings as usize);

        if let Some(projection) = projection {
            mesh.generate_texture_coords(projection);
        }

        mesh_repository.add_triangle_mesh_unless_present(mesh_id, mesh);

        *desynchronized = true;
    }

    TriangleMeshComp::new(mesh_id)
}

pub fn setup_hemisphere_mesh(
    mesh_repository: &mut MeshRepository,
    hemisphere_mesh: &HemisphereMeshComp,
    projection: Option<&impl TextureProjection<f32>>,
    desynchronized: &mut bool,
) -> TriangleMeshComp {
    let mesh_id = hemisphere_mesh.generate_id(create_projection_label(projection));

    if !mesh_repository.has_triangle_mesh(mesh_id) {
        let mut mesh = TriangleMesh::create_hemisphere(hemisphere_mesh.n_rings as usize);

        if let Some(projection) = projection {
            mesh.generate_texture_coords(projection);
        }

        mesh_repository.add_triangle_mesh_unless_present(mesh_id, mesh);

        *desynchronized = true;
    }

    TriangleMeshComp::new(mesh_id)
}

/// Generates the vertex attributes missing from the giving requirements for the
/// specified mesh, if possible.
pub fn generate_missing_vertex_properties_for_mesh(
    mesh_repository: &mut MeshRepository,
    mesh_id: MeshID,
    vertex_attribute_requirements: VertexAttributeSet,
) {
    if vertex_attribute_requirements.contains(VertexAttributeSet::NORMAL_VECTOR) {
        let mesh = mesh_repository
            .get_triangle_mesh(mesh_id)
            .expect("Missing mesh in repository for mesh component");

        if !mesh.has_normal_vectors() {
            impact_log::info!("Generating normal vectors for mesh {}", mesh_id);

            mesh_repository
                .get_triangle_mesh_mut(mesh_id)
                .unwrap()
                .generate_smooth_normal_vectors();
        }
    }

    if vertex_attribute_requirements.contains(VertexAttributeSet::TANGENT_SPACE_QUATERNION) {
        let mesh = mesh_repository
            .get_triangle_mesh(mesh_id)
            .expect("Missing mesh in repository for mesh component");

        if !mesh.has_tangent_space_quaternions() {
            impact_log::info!("Generating tangent space quaternions for mesh {}", mesh_id);

            mesh_repository
                .get_triangle_mesh_mut(mesh_id)
                .unwrap()
                .generate_smooth_tangent_space_quaternions();
        }
    }
}

fn create_projection_label(projection: Option<&impl TextureProjection<f32>>) -> String {
    projection
        .as_ref()
        .map_or("None".to_string(), |projection| projection.identifier())
}
