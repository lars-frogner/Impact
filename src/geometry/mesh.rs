//! Mesh data and representation.

use crate::geometry::{CollectionChange, CollectionChangeTracker};
use bytemuck::{Pod, Zeroable};
use nalgebra::Matrix4;
use std::fmt::Debug;

/// A 3D mesh represented by vertices and indices.
///
/// The vertices are unique and store their position
/// and other properties. Each index refers to a vertex,
/// and the sequence of indices describes the triangles
/// making up the mesh faces.
#[derive(Clone, Debug)]
pub struct Mesh<V> {
    vertices: Vec<V>,
    indices: Vec<u16>,
    vertex_change_tracker: CollectionChangeTracker,
    index_change_tracker: CollectionChangeTracker,
}

/// A group of instances of the same mesh.
#[derive(Clone, Debug)]
pub struct MeshInstanceGroup {
    instances: Vec<MeshInstance>,
    instance_change_tracker: CollectionChangeTracker,
}

/// An instance of a mesh with a certain transformation
/// applied to it.
///
/// Used to represent multiple versions of the same basic mesh.
#[derive(Clone, Debug)]
pub struct MeshInstance {
    transform_matrix: Matrix4<f32>,
}

/// Vertices that have an associated color.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
pub struct ColorVertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
}

/// Vertices that have a associated texture coordinates.
#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
pub struct TextureVertex {
    pub position: [f32; 3],
    pub texture_coords: [f32; 2],
}

impl<V> Mesh<V> {
    /// Creates a new mesh described by the given vertices and
    /// indices.
    pub fn new(vertices: Vec<V>, indices: Vec<u16>) -> Self {
        Self {
            vertices,
            indices,
            vertex_change_tracker: CollectionChangeTracker::new(),
            index_change_tracker: CollectionChangeTracker::new(),
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
    pub fn reset_vertex_change_tracking(&mut self) {
        self.vertex_change_tracker.reset();
    }

    /// Forgets any recorded changes to the vertices.
    pub fn reset_index_change_tracking(&mut self) {
        self.index_change_tracker.reset();
    }

    /// Forgets any recorded changes to the vertices and indices.
    pub fn reset_vertex_index_change_tracking(&mut self) {
        self.reset_vertex_change_tracking();
        self.reset_index_change_tracking();
    }
}

impl MeshInstanceGroup {
    /// Creates a new group of mesh instances from the given
    /// vector of instances.
    pub fn new(instances: Vec<MeshInstance>) -> Self {
        Self {
            instances,
            instance_change_tracker: CollectionChangeTracker::new(),
        }
    }

    /// Returns the instances making up the mesh instance group.
    pub fn instances(&self) -> &[MeshInstance] {
        &self.instances
    }

    /// Returns the kind of change that has been made to the mesh
    /// instances since the last reset of change tracing.
    pub fn instance_change(&self) -> CollectionChange {
        self.instance_change_tracker.change()
    }

    /// Forgets any recorded changes to the instances.
    pub fn reset_instance_change_tracking(&mut self) {
        self.instance_change_tracker.reset();
    }
}

impl MeshInstance {
    /// Creates a new mesh instance with no transform.
    pub fn new() -> Self {
        Self::with_transform(Matrix4::identity())
    }

    /// Creates a new mesh instance with the given transform.
    pub fn with_transform(transform_matrix: Matrix4<f32>) -> Self {
        Self { transform_matrix }
    }

    /// Returns the transform matrix describing the configuration of
    /// this mesh instance in relation to the default configuration of
    /// the mesh.
    pub fn transform_matrix(&self) -> &Matrix4<f32> {
        &self.transform_matrix
    }
}
