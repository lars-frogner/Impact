//! Mesh data and representation.

use crate::{
    geometry::{CollectionChange, CollectionChangeTracker},
    num::Float,
};
use bytemuck::{Pod, Zeroable};
use nalgebra::{Matrix4, Point3, Vector2, Vector3};
use std::{
    fmt::Debug,
    sync::atomic::{AtomicUsize, Ordering},
};

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

/// A container for instances of the same mesh.
///
/// The container maintains a buffer for instances
/// that is grown on demand, but never shrunk. Instead,
/// a counter keeps track of the position of the last valid
/// instance in the buffer, and the counter is reset to
/// zero when the container is cleared. This allows the
/// container to be filled and emptied repeatedly without
/// unneccesary allocations.
#[derive(Debug)]
pub struct MeshInstanceContainer<F> {
    instance_buffer: Vec<MeshInstance<F>>,
    n_valid_instances: AtomicUsize,
}

/// An instance of a mesh with a certain transformation
/// applied to it.
///
/// Used to represent multiple versions of the same basic mesh.
#[repr(transparent)]
#[derive(Copy, Clone, Debug)]
pub struct MeshInstance<F> {
    transform_matrix: Matrix4<F>,
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

impl<F: Float> MeshInstanceContainer<F> {
    /// Creates a new empty container for mesh instances.
    pub fn new() -> Self {
        Self {
            instance_buffer: Vec::new(),
            n_valid_instances: AtomicUsize::new(0),
        }
    }

    /// Returns the current number of valid instances in the container.
    pub fn n_valid_instances(&self) -> usize {
        self.n_valid_instances.load(Ordering::Acquire)
    }

    /// Returns a slice with all the instances in the container,
    /// including invalid ones.
    ///
    /// # Warning
    /// Only the elements below
    /// [`n_valid_instances`](Self::n_valid_instances) are
    /// considered to have valid values.
    pub fn instance_buffer(&self) -> &[MeshInstance<F>] {
        &self.instance_buffer
    }

    /// Returns a slice with the valid instances in the container.
    pub fn valid_instances(&self) -> &[MeshInstance<F>] {
        &self.instance_buffer[0..self.n_valid_instances()]
    }

    /// Inserts the given instance into the container.
    pub fn add_instance(&mut self, instance: MeshInstance<F>) {
        let instance_buffer_length = self.instance_buffer.len();
        let idx = self.n_valid_instances.fetch_add(1, Ordering::SeqCst);
        assert!(idx <= instance_buffer_length);

        // If the buffer is full, grow it first
        if idx == instance_buffer_length {
            self.grow_instance_buffer();
        }

        self.instance_buffer[idx] = instance;
    }

    /// Empties the container for instances.
    ///
    /// Does not actually drop anything, just resets the count of
    /// valid instances to zero.
    pub fn clear(&self) {
        self.n_valid_instances.store(0, Ordering::Release);
    }

    fn grow_instance_buffer(&mut self) {
        let old_buffer_length = self.instance_buffer.len();

        // Add one before doubling to avoid getting stuck at zero
        let new_buffer_length = (old_buffer_length + 1).checked_mul(2).unwrap();

        let mut new_buffer = vec![MeshInstance::new(); new_buffer_length];
        new_buffer[0..old_buffer_length].copy_from_slice(&self.instance_buffer);

        self.instance_buffer = new_buffer;
    }
}

impl<F: Float> Default for MeshInstanceContainer<F> {
    fn default() -> Self {
        Self::new()
    }
}

impl<F: Float> MeshInstance<F> {
    /// Creates a new mesh instance with no transform.
    pub fn new() -> Self {
        Self::with_transform(Matrix4::identity())
    }

    /// Creates a new mesh instance with the given transform.
    pub fn with_transform(transform_matrix: Matrix4<F>) -> Self {
        Self { transform_matrix }
    }

    /// Returns the transform matrix describing the configuration of
    /// this mesh instance in relation to the default configuration of
    /// the mesh.
    pub fn transform_matrix(&self) -> &Matrix4<F> {
        &self.transform_matrix
    }
}

impl<F: Float> Default for MeshInstance<F> {
    fn default() -> Self {
        Self::new()
    }
}

// Since `MeshInstance` is `#[repr(transparent)]`, it will be
// `Zeroable` and `Pod` as long as its field, `Matrix4`, is so.
unsafe impl<F> Zeroable for MeshInstance<F> where Matrix4<F>: Zeroable {}

unsafe impl<F> Pod for MeshInstance<F>
where
    F: Float,
    Matrix4<F>: Pod,
{
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
