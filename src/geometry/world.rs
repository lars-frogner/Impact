//! Container for geometrical data.

use super::{Camera, ColorVertex, Mesh, MeshInstanceContainer, PerspectiveCamera, TextureVertex};
use crate::hash::StringHash;
use nalgebra::Isometry3;
use std::collections::HashMap;

pub type GeometryID = StringHash;
pub type GeometryMap<T> = HashMap<GeometryID, T>;

type ColorMesh = Mesh<ColorVertex<f32>>;
type TextureMesh = Mesh<TextureVertex<f32>>;

/// Container for all geometrical data in the world.
#[derive(Debug)]
pub struct GeometricalData {
    /// Cameras using perspective transformations.
    perspective_cameras: GeometryMap<PerspectiveCamera<f32>>,
    /// Meshes with vertices that hold color values.
    color_meshes: GeometryMap<ColorMesh>,
    /// Meshes with vertices that hold texture coordinates.
    texture_meshes: GeometryMap<TextureMesh>,
    /// Containers for instances of the same mesh.
    mesh_instance_containers: GeometryMap<MeshInstanceContainer<f32>>,
}

impl GeometricalData {
    /// Creates a new empty geometrical data container.
    pub fn new() -> Self {
        Self {
            perspective_cameras: HashMap::new(),
            color_meshes: HashMap::new(),
            texture_meshes: HashMap::new(),
            mesh_instance_containers: HashMap::new(),
        }
    }

    /// Returns a reference to the collection of all cameras using
    /// perspective transformations.
    pub fn perspective_cameras(&self) -> &GeometryMap<PerspectiveCamera<f32>> {
        &self.perspective_cameras
    }

    /// Returns a reference to the collection of all meshes with
    /// vertices that hold color values.
    pub fn color_meshes(&self) -> &GeometryMap<ColorMesh> {
        &self.color_meshes
    }

    /// Returns a reference to the collection of all meshes with
    /// vertices that hold texture coordinates.
    pub fn texture_meshes(&self) -> &GeometryMap<ColorMesh> {
        &self.color_meshes
    }

    /// Returns a reference to the collection of all containers
    /// for instances of the same mesh.
    pub fn mesh_instance_containers(&self) -> &GeometryMap<MeshInstanceContainer<f32>> {
        &self.mesh_instance_containers
    }

    /// Stores the given perspective camera under the given ID.
    ///
    /// # Panics
    /// If a perspective camera with the given ID already exists.
    pub fn add_perspective_camera(
        &mut self,
        camera_id: GeometryID,
        camera: PerspectiveCamera<f32>,
    ) {
        let existing_camera = self.perspective_cameras.insert(camera_id, camera);
        assert!(existing_camera.is_none());
    }

    /// Stores the given color mesh under the given ID.
    ///
    /// # Panics
    /// If a mesh (regardless of type) with the given ID
    /// already exists.
    pub fn add_color_mesh(&mut self, mesh_id: GeometryID, mesh: ColorMesh) {
        assert!(!self.texture_meshes.contains_key(&mesh_id));
        let existing_mesh = self.color_meshes.insert(mesh_id, mesh);
        assert!(existing_mesh.is_none());
        self.mesh_instance_containers
            .insert(mesh_id, MeshInstanceContainer::new());
    }

    /// Stores the given texture mesh under the given ID.
    ///
    /// # Panics
    /// If a mesh (regardless of type) with the given ID
    /// already exists.
    pub fn add_texture_mesh(&mut self, mesh_id: GeometryID, mesh: TextureMesh) {
        assert!(!self.color_meshes.contains_key(&mesh_id));
        let existing_mesh = self.texture_meshes.insert(mesh_id, mesh);
        assert!(existing_mesh.is_none());
        self.mesh_instance_containers
            .insert(mesh_id, MeshInstanceContainer::new());
    }

    /// Applies the given transform to all cameras.
    pub fn transform_cameras(&mut self, transform: &Isometry3<f32>) {
        self.perspective_cameras
            .values_mut()
            .for_each(|camera| camera.config_mut().transform(transform));
    }
}

impl Default for GeometricalData {
    fn default() -> Self {
        Self::new()
    }
}
