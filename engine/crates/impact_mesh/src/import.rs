//! Importing meshes based on mesh declarations.

use crate::{
    TriangleMesh, TriangleMeshDirtyMask, TriangleMeshID, TriangleMeshRegistry,
    io::TriangleMeshFileFormat,
    setup::{TriangleMeshTemplate, setup_triangle_mesh_from_template},
    texture_projection::{PlanarTextureProjection, TextureProjectionDeclaration},
};
use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct TriangleMeshDeclaration {
    pub id: TriangleMeshID,
    pub source: TriangleMeshSource,
    pub texture_projection: Option<TextureProjectionDeclaration>,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub enum TriangleMeshSource {
    File(PathBuf),
    Template(TriangleMeshTemplate),
}

impl TriangleMeshDeclaration {
    /// Resolves all paths in the declaration by prepending the given root path
    /// to all paths.
    pub fn resolve_paths(&mut self, root_path: &Path) {
        if let TriangleMeshSource::File(file_path) = &mut self.source {
            *file_path = root_path.join(&*file_path);
        }
    }
}

/// Loads all meshes in the given declarations and stores them in the registry.
///
/// # Errors
/// See [`load_declared_triangle_mesh`].
pub fn load_declared_meshes(
    registry: &mut TriangleMeshRegistry,
    triangle_mesh_declarations: &[TriangleMeshDeclaration],
) -> Result<()> {
    for declaration in triangle_mesh_declarations {
        if let Err(error) = load_declared_triangle_mesh(registry, declaration) {
            // Failing to load a mesh is not fatal, since we might not need it
            log::error!("Failed to load triangle mesh {}: {error:#}", declaration.id);
        }
    }
    Ok(())
}

/// Loads the triangle mesh in the given declaration and stores it in the
/// registry.
///
/// # Errors
/// Returns an error if:
/// - Another mesh with the same name is already loaded.
/// - The texture projection is not valid.
/// - The file format is not supported (if the source is a file).
/// - The file can not be found or loaded as a mesh (if the source is a file).
pub fn load_declared_triangle_mesh(
    registry: &mut TriangleMeshRegistry,
    declaration: &TriangleMeshDeclaration,
) -> Result<TriangleMeshID> {
    match &declaration.source {
        TriangleMeshSource::File(file_path) => load_triangle_mesh_from_file(
            registry,
            declaration.id,
            file_path,
            declaration.texture_projection.as_ref(),
        ),
        TriangleMeshSource::Template(template) => load_triangle_mesh_from_template(
            registry,
            declaration.id,
            template,
            declaration.texture_projection.as_ref(),
        ),
    }
}

fn load_triangle_mesh_from_file(
    registry: &mut TriangleMeshRegistry,
    mesh_id: TriangleMeshID,
    file_path: &Path,
    texture_projection: Option<&TextureProjectionDeclaration>,
) -> Result<TriangleMeshID> {
    log::debug!(
        "Loading triangle mesh `{mesh_id}` from {}",
        file_path.display()
    );

    let file_format = TriangleMeshFileFormat::from_path(file_path)?;

    if registry.contains(mesh_id) {
        bail!("Tried to load triangle mesh under already existing ID: {mesh_id}");
    }

    let mut mesh: TriangleMesh = match file_format {
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
        Some(TextureProjectionDeclaration::Planar {
            origin,
            u_vector,
            v_vector,
        }) => {
            let projection =
                PlanarTextureProjection::new(origin.unpack(), u_vector.unpack(), v_vector.unpack())
                    .with_context(|| {
                        format!("Invalid planar texture projection for triangle mesh `{mesh_id}`")
                    })?;
            mesh.generate_texture_coords(&projection, &mut dirty_flags);
        }
    }

    registry.insert(mesh_id, mesh);

    Ok(mesh_id)
}

fn load_triangle_mesh_from_template(
    registry: &mut TriangleMeshRegistry,
    mesh_id: TriangleMeshID,
    template: &TriangleMeshTemplate,
    texture_projection: Option<&TextureProjectionDeclaration>,
) -> Result<TriangleMeshID> {
    log::debug!("Loading triangle mesh `{mesh_id}` from template");

    if registry.contains(mesh_id) {
        bail!("Tried to load triangle mesh under already existing name: {mesh_id}");
    }

    match texture_projection {
        None => {
            setup_triangle_mesh_from_template(
                registry,
                template,
                Some(mesh_id),
                Option::<&PlanarTextureProjection>::None,
            );
        }
        Some(TextureProjectionDeclaration::Planar {
            origin,
            u_vector,
            v_vector,
        }) => {
            let projection =
                PlanarTextureProjection::new(origin.unpack(), u_vector.unpack(), v_vector.unpack())
                    .with_context(|| {
                        format!("Invalid planar texture projection for triangle mesh `{mesh_id}`")
                    })?;

            setup_triangle_mesh_from_template(registry, template, Some(mesh_id), Some(&projection));
        }
    }

    Ok(mesh_id)
}
