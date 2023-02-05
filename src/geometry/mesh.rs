//! Mesh data and representation.

use crate::{
    geometry::{CollectionChange, CollectionChangeTracker, Sphere},
    num::Float,
};
use bytemuck::{Pod, Zeroable};
use nalgebra::{point, Point3, UnitVector3, Vector2, Vector3, Vector4};
use std::fmt::Debug;

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
    pub color: Vector4<F>,
}

/// Vertices that have associated texture coordinates.
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct TextureVertex<F: Float> {
    pub position: Point3<F>,
    pub texture_coords: Vector2<F>,
}

/// Vertices that have an associated normal vector.
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct NormalVectorVertex<F: Float> {
    pub position: Point3<F>,
    pub normal_vector: UnitVector3<F>,
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

impl<F: Float> Vertex<F> for NormalVectorVertex<F> {
    fn position(&self) -> &Point3<F> {
        &self.position
    }
}

// Since `ColorVertex` is `#[repr(C)]`, it will be `Zeroable` and `Pod` as long
// as its fields, `Point3` and `Vector2`, are so and there is no padding. We
// know there will be no padding since both fields will have the same alignment
// (the alignment of `F`).
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

// Since `TextureVertex` is `#[repr(C)]`, it will be `Zeroable` and `Pod` as
// long as its fields, `Point3` and `Vector3`, are so and there is no padding.
// We know there will be no padding since both fields will have the same
// alignment (the alignment of `F`).
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

// Since `ColorVertex` is `#[repr(C)]`, it will be `Zeroable` and `Pod` as long
// as its fields, `Point3` and `UnitVector3`, are so and there is no padding. We
// know there will be no padding since both fields will have the same alignment
// (the alignment of `F`).
unsafe impl<F> Zeroable for NormalVectorVertex<F>
where
    F: Float,
    Point3<F>: Zeroable,
    UnitVector3<F>: Zeroable,
{
}

unsafe impl<F> Pod for NormalVectorVertex<F>
where
    F: Float,
    Point3<F>: Pod,
    UnitVector3<F>: Pod,
{
}
