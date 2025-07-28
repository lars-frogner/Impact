//! Importing meshes based on mesh specifications.

use crate::{
    TriangleMesh, TriangleMeshDirtyMask, TriangleMeshHandle, TriangleMeshID, TriangleMeshRegistry,
    io::TriangleMeshFileFormat,
    setup::{TriangleMeshTemplate, setup_triangle_mesh_from_template},
    texture_projection::{PlanarTextureProjection, TextureProjectionSpecification},
};
use anyhow::{Context, Result, bail};
use impact_math::hash64;
use std::path::{Path, PathBuf};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct TriangleMeshSpecification {
    pub name: String,
    pub source: TriangleMeshSource,
    pub texture_projection: Option<TextureProjectionSpecification>,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub enum TriangleMeshSource {
    File(PathBuf),
    Template(TriangleMeshTemplate),
}

impl TriangleMeshSpecification {
    /// Resolves all paths in the specification by prepending the given root
    /// path to all paths.
    pub fn resolve_paths(&mut self, root_path: &Path) {
        if let TriangleMeshSource::File(file_path) = &mut self.source {
            *file_path = root_path.join(&*file_path);
        }
    }
}

/// Loads all meshes in the given specifications and stores them in the
/// registry.
///
/// # Errors
/// See [`load_specified_triangle_mesh`].
pub fn load_specified_meshes(
    registry: &mut TriangleMeshRegistry,
    triangle_mesh_specifications: &[TriangleMeshSpecification],
) -> Result<()> {
    for specification in triangle_mesh_specifications {
        load_specified_triangle_mesh(registry, specification)?;
    }
    Ok(())
}

/// Loads the triangle mesh in the given specification and stores it in the
/// registry.
///
/// # Errors
/// Returns an error if:
/// - Another mesh with the same name is already loaded.
/// - The texture projection is not valid.
/// - The file format is not supported (if the source is a file).
/// - The file can not be found or loaded as a mesh (if the source is a file).
pub fn load_specified_triangle_mesh(
    registry: &mut TriangleMeshRegistry,
    specification: &TriangleMeshSpecification,
) -> Result<TriangleMeshHandle> {
    match &specification.source {
        TriangleMeshSource::File(file_path) => load_triangle_mesh_from_file(
            registry,
            &specification.name,
            file_path,
            specification.texture_projection.as_ref(),
        ),
        TriangleMeshSource::Template(template) => load_triangle_mesh_from_template(
            registry,
            &specification.name,
            template,
            specification.texture_projection.as_ref(),
        ),
    }
}

fn load_triangle_mesh_from_file(
    registry: &mut TriangleMeshRegistry,
    name: &str,
    file_path: &Path,
    texture_projection: Option<&TextureProjectionSpecification>,
) -> Result<TriangleMeshHandle> {
    impact_log::debug!(
        "Loading triangle mesh `{name}` from {}",
        file_path.display(),
    );

    let mesh_id = TriangleMeshID(hash64!(&name));

    let file_format = TriangleMeshFileFormat::from_path(file_path)?;

    if registry.contains_resource_with_pid(mesh_id) {
        bail!("Tried to load triangle mesh under already existing name: {name}");
    }

    let mut mesh: TriangleMesh<f32> = match file_format {
        #[cfg(feature = "obj")]
        TriangleMeshFileFormat::Obj => crate::io::obj::read_mesh_from_obj_file(file_path),
        #[cfg(not(feature = "obj"))]
        TriangleMeshFileFormat::Obj => Err(anyhow::anyhow!(
            "Please enable the `obj` feature in order to read .obj files"
        )),
        #[cfg(feature = "ply")]
        TriangleMeshFileFormat::Ply => crate::io::ply::read_mesh_from_ply_file(file_path),
        #[cfg(not(feature = "ply"))]
        TriangleMeshFileFormat::Ply => Err(anyhow::anyhow!(
            "Please enable the `ply` feature in order to read .ply files"
        )),
    }
    .with_context(|| format!("Failed to load triangle mesh from {}", file_path.display()))?;

    let mut dirty_flags = TriangleMeshDirtyMask::empty();

    match texture_projection {
        None => {}
        Some(TextureProjectionSpecification::Planar {
            origin,
            u_vector,
            v_vector,
        }) => {
            let projection = PlanarTextureProjection::new(*origin, *u_vector, *v_vector)
                .with_context(|| {
                    format!("Invalid planar texture projection for triangle mesh `{name}`")
                })?;
            mesh.generate_texture_coords(&projection, &mut dirty_flags);
        }
    }

    Ok(registry.insert_resource_with_pid(mesh_id, mesh))
}

fn load_triangle_mesh_from_template(
    registry: &mut TriangleMeshRegistry,
    name: &str,
    template: &TriangleMeshTemplate,
    texture_projection: Option<&TextureProjectionSpecification>,
) -> Result<TriangleMeshHandle> {
    impact_log::debug!("Loading triangle mesh `{name}` from template");

    let mesh_id = TriangleMeshID(hash64!(&name));

    if registry.contains_resource_with_pid(mesh_id) {
        bail!("Tried to load triangle mesh under already existing name: {name}");
    }

    Ok(match texture_projection {
        None => setup_triangle_mesh_from_template(
            registry,
            template,
            Some(mesh_id),
            Option::<&PlanarTextureProjection<f32>>::None,
        ),
        Some(TextureProjectionSpecification::Planar {
            origin,
            u_vector,
            v_vector,
        }) => {
            let projection = PlanarTextureProjection::new(*origin, *u_vector, *v_vector)
                .with_context(|| {
                    format!("Invalid planar texture projection for triangle mesh `{name}`")
                })?;

            setup_triangle_mesh_from_template(registry, template, Some(mesh_id), Some(&projection))
        }
    })
}
