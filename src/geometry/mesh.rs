//! Mesh data and representation.

use crate::{
    geometry::{CollectionChange, CollectionChangeTracker},
    num::Float,
};
use bytemuck::{Pod, Zeroable};
use nalgebra::{Point3, Vector2, Vector3};
use std::{collections::HashMap, fmt::Debug};

stringhash_newtype!(
    /// Identifier for specific meshes.
    /// Wraps a [`StringHash`](crate::hash::StringHash).
    [pub] MeshID
);

/// Repository where [`Mesh`]es are stored under a
/// unique [`MeshID`] (the ID is allowed to be the
/// same for separate types of meshes).
#[derive(Debug, Default)]
pub struct MeshRepository<F: Float> {
    /// Meshes with vertices that hold color values.
    pub color_meshes: HashMap<MeshID, Mesh<ColorVertex<F>>>,
    /// Meshes with vertices that hold texture coordinates.
    pub texture_meshes: HashMap<MeshID, Mesh<TextureVertex<F>>>,
}

/// A 3D mesh represented by vertices and indices.
///
/// The vertices are unique and store their position
/// and other properties. Each index refers to a vertex,
/// and the sequence of indices describes the triangles
/// making up the mesh faces.
#[derive(Debug)]
pub struct Mesh<V> {
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
}

impl<V> Mesh<V> {
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
