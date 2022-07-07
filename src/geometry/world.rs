//! Container for geometrical data.

use super::{ColorVertex, Mesh, MeshInstanceGroup, PerspectiveCamera, TextureVertex};
use std::collections::HashMap;

pub type WorldIdent = String;
pub type WorldObjMap<T> = HashMap<WorldIdent, T>;

/// Container for all geometrical data in the world.
pub struct WorldData {
    /// Meshes with vertices that hold color values.
    pub color_meshes: WorldObjMap<Mesh<ColorVertex>>,
    /// Meshes with vertices that hold texture coordinates.
    pub texture_meshes: WorldObjMap<Mesh<TextureVertex>>,
    /// Groups of instances of the same mesh.
    pub mesh_instance_groups: WorldObjMap<MeshInstanceGroup>,
    /// Cameras using perspective transformations.
    pub perspective_cameras: WorldObjMap<PerspectiveCamera<f32>>,
}

impl WorldData {
    /// Creates a new empty world data container.
    pub fn new() -> Self {
        Self {
            color_meshes: HashMap::new(),
            texture_meshes: HashMap::new(),
            mesh_instance_groups: HashMap::new(),
            perspective_cameras: HashMap::new(),
        }
    }
}
