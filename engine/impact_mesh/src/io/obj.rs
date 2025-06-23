//! Input/output of mesh data in Wavefront OBJ format.

use crate::{
    MeshID, MeshRepository, VertexNormalVector, VertexPosition, VertexTextureCoords,
    components::TriangleMeshComp, texture_projection::TextureProjection, triangle::TriangleMesh,
};
use anyhow::{Result, bail};
use impact_math::hash64;
use nalgebra::{UnitVector3, point, vector};
use std::{fmt::Debug, path::Path};
use tobj::{GPU_LOAD_OPTIONS, Mesh as ObjMesh};

/// Reads the Wavefront OBJ file at the given path and creates a corresponding
/// `TriangleMesh`. If there are multiple meshes in the file, they are merged
/// into a single mesh.
///
/// # Errors
/// Returns an error if the file can not be found or loaded as a mesh.
pub fn read_mesh_from_obj_file(file_path: impl AsRef<Path>) -> Result<TriangleMesh<f32>> {
    let file_path = file_path.as_ref();

    let (mut models, _) = tobj::load_obj(file_path, &GPU_LOAD_OPTIONS)?;

    if models.is_empty() {
        bail!("File {} does not contain any meshes", file_path.display());
    }

    let mut mesh = create_mesh_from_tobj_mesh(models.pop().unwrap().mesh);

    for model in models {
        mesh.merge_with(&create_mesh_from_tobj_mesh(model.mesh));
    }

    Ok(mesh)
}

/// Reads the Wavefront OBJ file at the given path and adds the contained mesh
/// to the mesh repository if it does not already exist. If there are multiple
/// meshes in the file, they are merged into a single mesh.
///
/// # Returns
/// The [`TriangleMeshComp`] representing the mesh.
///
/// # Errors
/// Returns an error if the file can not be found or loaded as a mesh.
pub fn load_mesh_from_obj_file<P>(
    mesh_repository: &mut MeshRepository,
    obj_file_path: P,
) -> Result<TriangleMeshComp>
where
    P: AsRef<Path> + Debug,
{
    let obj_file_path = obj_file_path.as_ref();
    let obj_file_path_string = obj_file_path.to_string_lossy();

    let (mut models, _) = tobj::load_obj(obj_file_path, &GPU_LOAD_OPTIONS)?;

    if models.is_empty() {
        bail!("File {} does not contain any meshes", obj_file_path_string);
    }

    let mesh_id = MeshID(hash64!(obj_file_path_string));

    if !mesh_repository.has_triangle_mesh(mesh_id) {
        let mut mesh = create_mesh_from_tobj_mesh(models.pop().unwrap().mesh);

        for model in models {
            mesh.merge_with(&create_mesh_from_tobj_mesh(model.mesh));
        }

        mesh_repository.add_triangle_mesh_unless_present(mesh_id, mesh);
    }

    Ok(TriangleMeshComp { id: mesh_id })
}

/// Reads the Wavefront OBJ file at the given path and adds the contained mesh
/// to the mesh repository if it does not already exist, after generating
/// texture coordinates for the mesh using the given projection. If there are
/// multiple meshes in the file, they are merged into a single mesh.
///
/// # Returns
/// The [`TriangleMeshComp`] representing the mesh.
///
/// # Errors
/// Returns an error if the file can not be found or loaded as a mesh.
pub fn load_mesh_from_obj_file_with_projection<P>(
    mesh_repository: &mut MeshRepository,
    obj_file_path: P,
    projection: &impl TextureProjection<f32>,
) -> Result<TriangleMeshComp>
where
    P: AsRef<Path> + Debug,
{
    let obj_file_path = obj_file_path.as_ref();
    let obj_file_path_string = obj_file_path.to_string_lossy();

    let (mut models, _) = tobj::load_obj(obj_file_path, &GPU_LOAD_OPTIONS)?;

    if models.is_empty() {
        bail!("File {} does not contain any meshes", obj_file_path_string);
    }

    let mesh_id = MeshID(hash64!(format!(
        "{} (projection = {})",
        obj_file_path_string,
        projection.identifier()
    )));

    if !mesh_repository.has_triangle_mesh(mesh_id) {
        let mut mesh = create_mesh_from_tobj_mesh(models.pop().unwrap().mesh);

        for model in models {
            mesh.merge_with(&create_mesh_from_tobj_mesh(model.mesh));
        }

        mesh.generate_texture_coords(projection);

        mesh_repository.add_triangle_mesh_unless_present(mesh_id, mesh);
    }

    Ok(TriangleMeshComp { id: mesh_id })
}

fn create_mesh_from_tobj_mesh(mesh: ObjMesh) -> TriangleMesh<f32> {
    fn aggregate_3<T>(values: &[f32], aggregator: impl Fn(f32, f32, f32) -> T) -> Vec<T> {
        values
            .iter()
            .step_by(3)
            .zip(values.iter().skip(1).step_by(3))
            .zip(values.iter().skip(2).step_by(3))
            .map(|((&x, &y), &z)| aggregator(x, y, z))
            .collect()
    }

    fn aggregate_2<T>(values: &[f32], aggregator: impl Fn(f32, f32) -> T) -> Vec<T> {
        values
            .iter()
            .step_by(2)
            .zip(values.iter().skip(1).step_by(2))
            .map(|(&x, &y)| aggregator(x, y))
            .collect()
    }

    let positions = aggregate_3(&mesh.positions, |x, y, z| VertexPosition(point![x, y, z]));

    let normal_vectors = aggregate_3(&mesh.normals, |nx, ny, nz| {
        VertexNormalVector(UnitVector3::new_normalize(vector![nx, ny, nz]))
    });

    let texture_coords = aggregate_2(&mesh.texcoords, |u, v| VertexTextureCoords(vector![u, v]));

    TriangleMesh::new(
        positions,
        normal_vectors,
        texture_coords,
        Vec::new(),
        Vec::new(),
        mesh.indices,
    )
}
