//! Container for geometrical data.

use nalgebra::Isometry3;

use super::{Camera, ColorVertex, Mesh, MeshInstanceGroup, PerspectiveCamera, TextureVertex};
use std::collections::HashMap;

pub type GeomIdent = String;
pub type GeometryMap<T> = HashMap<GeomIdent, T>;

/// Container for all geometrical data in the world.
#[derive(Debug)]
pub struct GeometricalData {
    /// Meshes with vertices that hold color values.
    pub color_meshes: GeometryMap<Mesh<ColorVertex>>,
    /// Meshes with vertices that hold texture coordinates.
    pub texture_meshes: GeometryMap<Mesh<TextureVertex>>,
    /// Groups of instances of the same mesh.
    pub mesh_instance_groups: GeometryMap<MeshInstanceGroup<f32>>,
    /// Cameras using perspective transformations.
    pub perspective_cameras: GeometryMap<PerspectiveCamera<f32>>,
}

impl GeometricalData {
    /// Creates a new empty geometrical data container.
    pub fn new() -> Self {
        Self {
            color_meshes: HashMap::new(),
            texture_meshes: HashMap::new(),
            mesh_instance_groups: HashMap::new(),
            perspective_cameras: HashMap::new(),
        }
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
