//! Mesh data and representation.

use crate::{
    geometry::{CollectionChange, CollectionChangeTracker, Sphere},
    num::Float,
};
use anyhow::{anyhow, bail, Result};
use bytemuck::{Pod, Zeroable};
use nalgebra::{point, Point3, Vector2, Vector3};
use std::{
    collections::{hash_map::Entry, HashMap},
    fmt::Debug,
};

stringhash_newtype!(
    /// Identifier for specific meshes.
    /// Wraps a [`StringHash`](crate::hash::StringHash).
    [pub] MeshID
);

/// Repository where [`Mesh`]es are stored under a
/// unique [`MeshID`].
#[derive(Debug, Default)]
pub struct MeshRepository<F: Float> {
    /// Meshes with vertices that hold color values.
    color_meshes: HashMap<MeshID, TriangleMesh<ColorVertex<F>>>,
    /// Meshes with vertices that hold texture coordinates.
    texture_meshes: HashMap<MeshID, TriangleMesh<TextureVertex<F>>>,
}

/// Represents a 3D polygonial mesh.
pub trait Mesh<F: Float> {
    /// Returns a sphere enclosing all vertices in the mesh,
    /// or [`None`] if the mesh has no vertices.
    fn bounding_sphere(&self) -> Option<Sphere<F>>;
}

/// Represents a vertex of a polygon in a 3D mesh.
pub trait Vertex<F: Float> {
    /// Returns the position of the vertex.
    fn position(&self) -> &Point3<F>;
}

/// A 3D mesh of triangles represented by vertices and
/// indices.
///
/// The vertices are unique and store their position
/// and other properties. Each index refers to a vertex,
/// and the sequence of indices describes the triangles
/// making up the mesh faces.
#[derive(Debug)]
pub struct TriangleMesh<V> {
    vertices: Vec<V>,
    indices: Vec<u16>,
    vertex_change_tracker: CollectionChangeTracker,
    index_change_tracker: CollectionChangeTracker,
}

/// Vertices that have an associated color.
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ColorVertex<F: Float> {
    pub position: Point3<F>,
    pub color: Vector3<F>,
}

/// Vertices that have a associated texture coordinates.
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct TextureVertex<F: Float> {
    pub position: Point3<F>,
    pub texture_coords: Vector2<F>,
}

impl<F: Float> MeshRepository<F> {
    /// Creates a new empty mesh repository.
    pub fn new() -> Self {
        Self {
            color_meshes: HashMap::new(),
            texture_meshes: HashMap::new(),
        }
    }

    /// Returns a trait object representing the [`Mesh`] with
    /// the given ID, or [`None`] if the mesh is not present.
    pub fn get_mesh(&self, mesh_id: MeshID) -> Option<&dyn Mesh<F>> {
        match self.texture_meshes.get(&mesh_id) {
            Some(mesh) => Some(mesh),
            None => match self.color_meshes.get(&mesh_id) {
                Some(mesh) => Some(mesh),
                None => None,
            },
        }
    }

    /// Returns a reference to the [`HashMap`] storing all
    /// color meshes.
    pub fn color_meshes(&self) -> &HashMap<MeshID, TriangleMesh<ColorVertex<F>>> {
        &self.color_meshes
    }

    /// Returns a reference to the [`HashMap`] storing all
    /// texture meshes.
    pub fn texture_meshes(&self) -> &HashMap<MeshID, TriangleMesh<TextureVertex<F>>> {
        &self.texture_meshes
    }

    /// Includes the given color mesh in the repository
    /// under the given ID.
    ///
    /// # Errors
    /// Returns an error if a mesh with the given ID already
    /// exists. The repository will remain unchanged.
    pub fn add_color_mesh(
        &mut self,
        mesh_id: MeshID,
        mesh: TriangleMesh<ColorVertex<F>>,
    ) -> Result<()> {
        if self.texture_meshes().contains_key(&mesh_id) {
            bail!(
                "Mesh {} already present in repository as a texture mesh",
                mesh_id
            )
        }

        match self.color_meshes.entry(mesh_id) {
            Entry::Vacant(entry) => {
                entry.insert(mesh);
                Ok(())
            }
            Entry::Occupied(_) => Err(anyhow!(
                "Mesh {} already present in repository as a color mesh",
                mesh_id
            )),
        }
    }

    /// Includes the given texture mesh in the repository
    /// under the given ID.
    ///
    /// # Errors
    /// Returns an error if a mesh with the given ID already
    /// exists. The repository will remain unchanged.
    pub fn add_texture_mesh(
        &mut self,
        mesh_id: MeshID,
        mesh: TriangleMesh<TextureVertex<F>>,
    ) -> Result<()> {
        if self.color_meshes().contains_key(&mesh_id) {
            bail!(
                "Mesh {} already present in repository as a color mesh",
                mesh_id
            )
        }

        match self.texture_meshes.entry(mesh_id) {
            Entry::Vacant(entry) => {
                entry.insert(mesh);
                Ok(())
            }
            Entry::Occupied(_) => Err(anyhow!(
                "Mesh {} already present in repository as a texture mesh",
                mesh_id
            )),
        }
    }
}

impl<V> TriangleMesh<V> {
    /// Creates a new mesh described by the given vertices and
    /// indices.
    pub fn new(vertices: Vec<V>, indices: Vec<u16>) -> Self {
        Self {
            vertices,
            indices,
            vertex_change_tracker: CollectionChangeTracker::default(),
            index_change_tracker: CollectionChangeTracker::default(),
        }
    }

    /// Returns the vertices of the mesh.
    pub fn vertices(&self) -> &[V] {
        &self.vertices
    }

    /// Returns the vertex indices describing the faces of the mesh.
    pub fn indices(&self) -> &[u16] {
        &self.indices
    }

    /// Whether the mesh has any vertices.
    pub fn has_vertices(&self) -> bool {
        !self.vertices.is_empty()
    }

    /// Returns the kind of change that has been made to the mesh
    /// vertices since the last reset of change tracing.
    pub fn vertex_change(&self) -> CollectionChange {
        self.vertex_change_tracker.change()
    }

    /// Returns the kind of change that has been made to the mesh
    /// vertex indices since the last reset of change tracing.
    pub fn index_change(&self) -> CollectionChange {
        self.index_change_tracker.change()
    }

    /// Forgets any recorded changes to the vertices.
    pub fn reset_vertex_change_tracking(&self) {
        self.vertex_change_tracker.reset();
    }

    /// Forgets any recorded changes to the vertices.
    pub fn reset_index_change_tracking(&self) {
        self.index_change_tracker.reset();
    }

    /// Forgets any recorded changes to the vertices and indices.
    pub fn reset_vertex_index_change_tracking(&self) {
        self.reset_vertex_change_tracking();
        self.reset_index_change_tracking();
    }
}

impl<F, V> Mesh<F> for TriangleMesh<V>
where
    F: Float,
    V: Vertex<F>,
{
    fn bounding_sphere(&self) -> Option<Sphere<F>> {
        if !self.has_vertices() {
            return None;
        }
        let (min_point, max_point) = self.vertices().iter().fold(
            (
                point![F::MAX, F::MAX, F::MAX],
                point![F::MIN, F::MIN, F::MIN],
            ),
            |(min_point, max_point), vertex| {
                (
                    vertex.position().inf(&min_point),
                    vertex.position().sup(&max_point),
                )
            },
        );
        Some(Sphere::bounding_sphere_from_aabb_corners(
            &min_point, &max_point,
        ))
    }
}

impl<F: Float> Vertex<F> for ColorVertex<F> {
    fn position(&self) -> &Point3<F> {
        &self.position
    }
}

impl<F: Float> Vertex<F> for TextureVertex<F> {
    fn position(&self) -> &Point3<F> {
        &self.position
    }
}

// Since `ColorVertex` is `#[repr(C)]`, it will be `Zeroable`
// and `Pod` as long as its fields, `Point3` and `Vector2`, are so
// and there is no padding. We know there will be no padding since
// both fields will have the same alignment (the alignment of `F`).
unsafe impl<F> Zeroable for ColorVertex<F>
where
    F: Float,
    Point3<F>: Zeroable,
    Vector3<F>: Zeroable,
{
}

unsafe impl<F> Pod for ColorVertex<F>
where
    F: Float,
    Point3<F>: Pod,
    Vector3<F>: Pod,
{
}

// Since `TextureVertex` is `#[repr(C)]`, it will be `Zeroable`
// and `Pod` as long as its fields, `Point3` and `Vector3`, are so
// and there is no padding. We know there will be no padding since
// both fields will have the same alignment (the alignment of `F`).
unsafe impl<F> Zeroable for TextureVertex<F>
where
    F: Float,
    Point3<F>: Zeroable,
    Vector2<F>: Zeroable,
{
}

unsafe impl<F> Pod for TextureVertex<F>
where
    F: Float,
    Point3<F>: Pod,
    Vector2<F>: Pod,
{
}
