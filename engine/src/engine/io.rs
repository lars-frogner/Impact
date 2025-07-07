//! Conveniences for loading data into the engine.

use super::Engine;
use anyhow::Result;
use impact_mesh::{TriangleMeshID, texture_projection::TextureProjection};
use std::{fmt, path::Path};

impl Engine {
    /// Reads the Wavefront OBJ file at the given path and adds the contained
    /// mesh to the mesh repository if it does not already exist. If there
    /// are multiple meshes in the file, they are merged into a single mesh.
    ///
    /// # Returns
    /// The [`TriangleMeshHandle`] to the mesh.
    ///
    /// # Errors
    /// Returns an error if the file can not be found or loaded as a mesh.
    #[cfg(feature = "obj")]
    pub fn load_mesh_from_obj_file<P>(&self, obj_file_path: P) -> Result<TriangleMeshID>
    where
        P: AsRef<Path> + fmt::Debug,
    {
        let scene = self.scene.read().unwrap();
        let mut mesh_repository = scene.mesh_repository().write().unwrap();
        impact_mesh::io::obj::load_mesh_from_obj_file(&mut mesh_repository, obj_file_path)
    }

    /// Reads the Wavefront OBJ file at the given path and adds the contained
    /// mesh to the mesh repository if it does not already exist, after
    /// generating texture coordinates for the mesh using the given
    /// projection. If there are multiple meshes in the file, they are
    /// merged into a single mesh.
    ///
    /// # Returns
    /// The [`TriangleMeshHandle`] to the mesh.
    ///
    /// # Errors
    /// Returns an error if the file can not be found or loaded as a mesh.
    #[cfg(feature = "obj")]
    pub fn load_mesh_from_obj_file_with_projection<P>(
        &self,
        obj_file_path: P,
        projection: &impl TextureProjection<f32>,
    ) -> Result<TriangleMeshID>
    where
        P: AsRef<Path> + fmt::Debug,
    {
        let scene = self.scene.read().unwrap();
        let mut mesh_repository = scene.mesh_repository().write().unwrap();
        impact_mesh::io::obj::load_mesh_from_obj_file_with_projection(
            &mut mesh_repository,
            obj_file_path,
            projection,
        )
    }

    /// Reads the PLY (Polygon File Format, also called Stanford Triangle
    /// Format) file at the given path and adds the contained mesh to the mesh
    /// repository if it does not already exist.
    ///
    /// # Returns
    /// The [`TriangleMeshHandle`] to the mesh.
    ///
    /// # Errors
    /// Returns an error if the file can not be found or loaded as a mesh.
    #[cfg(feature = "ply")]
    pub fn load_mesh_from_ply_file<P>(&self, ply_file_path: P) -> Result<TriangleMeshID>
    where
        P: AsRef<Path> + fmt::Debug,
    {
        let scene = self.scene.read().unwrap();
        let mut mesh_repository = scene.mesh_repository().write().unwrap();
        impact_mesh::io::ply::load_mesh_from_ply_file(&mut mesh_repository, ply_file_path)
    }

    /// Reads the PLY (Polygon File Format, also called Stanford Triangle
    /// Format) file at the given path and adds the contained mesh to the
    /// mesh repository if it does not already exist, after generating
    /// texture coordinates for the mesh using the given projection.
    ///
    /// # Returns
    /// The [`TriangleMeshHandle`] to the mesh.
    ///
    /// # Errors
    /// Returns an error if the file can not be found or loaded as a mesh.
    #[cfg(feature = "ply")]
    pub fn load_mesh_from_ply_file_with_projection<P>(
        &self,
        ply_file_path: P,
        projection: &impl TextureProjection<f32>,
    ) -> Result<TriangleMeshID>
    where
        P: AsRef<Path> + fmt::Debug,
    {
        let scene = self.scene.read().unwrap();
        let mut mesh_repository = scene.mesh_repository().write().unwrap();
        impact_mesh::io::ply::load_mesh_from_ply_file_with_projection(
            &mut mesh_repository,
            ply_file_path,
            projection,
        )
    }
}
