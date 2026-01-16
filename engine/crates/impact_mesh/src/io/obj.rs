//! Input/output of mesh data in Wavefront OBJ format.

use crate::{
    TriangleMesh, TriangleMeshDirtyMask, VertexNormalVector, VertexPosition, VertexTextureCoords,
};
use anyhow::{Result, bail};
use impact_math::{
    point::Point3C,
    vector::{UnitVector3C, Vector2, Vector3C},
};
use std::path::Path;
use tobj::{GPU_LOAD_OPTIONS, Mesh as ObjMesh};

/// Reads the Wavefront OBJ file at the given path and creates a corresponding
/// `TriangleMesh`. If there are multiple meshes in the file, they are merged
/// into a single mesh.
///
/// # Errors
/// Returns an error if the file can not be found or loaded as a mesh.
pub fn read_mesh_from_obj_file(file_path: impl AsRef<Path>) -> Result<TriangleMesh> {
    let file_path = file_path.as_ref();

    let (mut models, _) = tobj::load_obj(file_path, &GPU_LOAD_OPTIONS)?;

    if models.is_empty() {
        bail!("File {} does not contain any meshes", file_path.display());
    }

    let mut mesh = create_mesh_from_tobj_mesh(models.pop().unwrap().mesh);
    let mut dirty_mask = TriangleMeshDirtyMask::empty();

    for model in models {
        mesh.merge_with(&create_mesh_from_tobj_mesh(model.mesh), &mut dirty_mask);
    }

    Ok(mesh)
}

fn create_mesh_from_tobj_mesh(mesh: ObjMesh) -> TriangleMesh {
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

    let positions = aggregate_3(&mesh.positions, |x, y, z| {
        VertexPosition(Point3C::new(x, y, z))
    });

    let normal_vectors = aggregate_3(&mesh.normals, |nx, ny, nz| {
        VertexNormalVector(UnitVector3C::normalized_from(Vector3C::new(nx, ny, nz)))
    });

    let texture_coords = aggregate_2(&mesh.texcoords, |u, v| {
        VertexTextureCoords(Vector2::new(u, v))
    });

    TriangleMesh::new(
        positions,
        normal_vectors,
        texture_coords,
        Vec::new(),
        Vec::new(),
        mesh.indices,
    )
}
