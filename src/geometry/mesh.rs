//! Mesh data and representation.

mod generation;

use crate::{
    geometry::{CollectionChange, CollectionChangeTracker, Sphere},
    num::Float,
};
use bitflags::bitflags;
use bytemuck::{Pod, Zeroable};
use nalgebra::{Point3, UnitVector3, Vector2, Vector3};
use std::fmt::{Debug, Display};

use super::{AxisAlignedBox, Point};

/// Represents a type of attribute associated with a mesh vertex.
pub trait VertexAttribute: Sized {
    /// Index of this attribute when pieces of data associated with each vertex
    /// attribute are stored together.
    const GLOBAL_INDEX: usize;

    /// The [`VertexAttributeSet`] containing only this attribute.
    const FLAG: VertexAttributeSet = VERTEX_ATTRIBUTE_FLAGS[Self::GLOBAL_INDEX];

    /// A string with the name of this attribute.
    const NAME: &'static str = VERTEX_ATTRIBUTE_NAMES[Self::GLOBAL_INDEX];
}

/// The 3D position of a mesh vertex.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct VertexPosition<F: Float>(pub Point3<F>);

/// The RGB color of a mesh at a vertex position.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct VertexColor<F: Float>(pub Vector3<F>);

/// The unit normal vector of a mesh at a vertex position.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct VertexNormalVector<F: Float>(pub UnitVector3<F>);

/// The (u, v) texture coordinates of a mesh at a vertex position.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct VertexTextureCoords<F: Float>(pub Vector2<F>);

bitflags! {
    /// Bitflag encoding a set of [`VertexAttribute`]s.
    pub struct VertexAttributeSet: u32 {
        const POSITION = 0b00000001;
        const COLOR = 0b00000010;
        const NORMAL_VECTOR = 0b00000100;
        const TEXTURE_COORDS = 0b00001000;
    }
}

/// A 3D mesh of triangles represented by vertices and indices.
///
/// The vertices are unique, and they have associated positions and potentially
/// other attributes. Each index refers to a vertex, and the sequence of indices
/// describes the triangles making up the mesh faces.
#[derive(Debug)]
pub struct TriangleMesh<F: Float> {
    positions: Vec<VertexPosition<F>>,
    colors: Vec<VertexColor<F>>,
    normal_vectors: Vec<VertexNormalVector<F>>,
    texture_coords: Vec<VertexTextureCoords<F>>,
    indices: Vec<u32>,
    position_change_tracker: CollectionChangeTracker,
    color_change_tracker: CollectionChangeTracker,
    normal_vector_change_tracker: CollectionChangeTracker,
    texture_coord_change_tracker: CollectionChangeTracker,
    index_change_tracker: CollectionChangeTracker,
}

/// The total number of supported vertex attribute types.
pub const N_VERTEX_ATTRIBUTES: usize = 4;

/// The bitflag of each individual vertex attribute, ordered according to
/// [`VertexAttribute::GLOBAL_INDEX`].
pub const VERTEX_ATTRIBUTE_FLAGS: [VertexAttributeSet; N_VERTEX_ATTRIBUTES] = [
    VertexAttributeSet::POSITION,
    VertexAttributeSet::COLOR,
    VertexAttributeSet::NORMAL_VECTOR,
    VertexAttributeSet::TEXTURE_COORDS,
];

/// The name of each individual vertex attribute, ordered according to
/// [`VertexAttribute::GLOBAL_INDEX`].
pub const VERTEX_ATTRIBUTE_NAMES: [&str; N_VERTEX_ATTRIBUTES] =
    ["position", "color", "normal vector", "texture coords"];

impl<F: Float> VertexAttribute for VertexPosition<F> {
    const GLOBAL_INDEX: usize = 0;
}

impl<F: Float> VertexAttribute for VertexColor<F> {
    const GLOBAL_INDEX: usize = 1;
}

impl<F: Float> VertexAttribute for VertexNormalVector<F> {
    const GLOBAL_INDEX: usize = 2;
}

impl<F: Float> VertexAttribute for VertexTextureCoords<F> {
    const GLOBAL_INDEX: usize = 3;
}

impl Display for VertexAttributeSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{ ")?;
        for (&attribute, name) in VERTEX_ATTRIBUTE_FLAGS
            .iter()
            .zip(VERTEX_ATTRIBUTE_NAMES.iter())
        {
            if self.contains(attribute) {
                write!(f, "`{}` ", name)?;
            }
        }
        write!(f, "}}")
    }
}

impl<F: Float> TriangleMesh<F> {
    /// Creates a new mesh described by the given vertex attributes and indices.
    ///
    /// # Panics
    /// If the length of `colors`, `normal_vectors` and `texture_coords` are
    /// neither zero nor equal to the length of `positions`.
    pub fn new(
        positions: Vec<VertexPosition<F>>,
        colors: Vec<VertexColor<F>>,
        normal_vectors: Vec<VertexNormalVector<F>>,
        texture_coords: Vec<VertexTextureCoords<F>>,
        indices: Vec<u32>,
    ) -> Self {
        let n_vertices = positions.len();

        assert!(
            colors.is_empty() || colors.len() == n_vertices,
            "Mismatching number of colors and positions in mesh"
        );
        assert!(
            normal_vectors.is_empty() || normal_vectors.len() == n_vertices,
            "Mismatching number of normal vectors and positions in mesh"
        );
        assert!(
            texture_coords.is_empty() || texture_coords.len() == n_vertices,
            "Mismatching number of texture coordinates and positions in mesh"
        );

        Self {
            positions,
            colors,
            normal_vectors,
            texture_coords,
            indices,
            position_change_tracker: CollectionChangeTracker::default(),
            color_change_tracker: CollectionChangeTracker::default(),
            normal_vector_change_tracker: CollectionChangeTracker::default(),
            texture_coord_change_tracker: CollectionChangeTracker::default(),
            index_change_tracker: CollectionChangeTracker::default(),
        }
    }

    /// Returns the number of vertices in the mesh.
    pub fn n_vertices(&self) -> usize {
        self.positions.len()
    }

    /// Returns the number of vertex indices in the mesh.
    pub fn n_indices(&self) -> usize {
        self.indices.len()
    }

    /// Returns the number of triangles in the mesh.
    pub fn n_triangles(&self) -> usize {
        self.n_indices() / 3
    }

    /// Returns a slice with the positions of the mesh vertices.
    pub fn positions(&self) -> &[VertexPosition<F>] {
        &self.positions
    }

    /// Returns a slice with the colors of the mesh vertices.
    pub fn colors(&self) -> &[VertexColor<F>] {
        &self.colors
    }

    /// Returns a slice with the normal vectors of the mesh vertices.
    pub fn normal_vectors(&self) -> &[VertexNormalVector<F>] {
        &self.normal_vectors
    }

    /// Returns a slice with the texture coordinates of the mesh vertices.
    pub fn texture_coords(&self) -> &[VertexTextureCoords<F>] {
        &self.texture_coords
    }

    /// Returns a slice with the vertex indices describing the faces of the
    /// mesh.
    pub fn indices(&self) -> &[u32] {
        &self.indices
    }

    /// Whether the mesh has any vertices.
    pub fn has_positions(&self) -> bool {
        !self.positions.is_empty()
    }

    /// Whether the vertices have associated colors.
    pub fn has_colors(&self) -> bool {
        !self.colors.is_empty()
    }

    /// Whether the vertices have associated normal vectors.
    pub fn has_normal_vectors(&self) -> bool {
        !self.normal_vectors.is_empty()
    }

    /// Whether the vertices have associated texture coordinates.
    pub fn has_texture_coords(&self) -> bool {
        !self.texture_coords.is_empty()
    }

    /// Returns the kind of change that has been made to the vertex positions
    /// since the last reset of change tracking.
    pub fn position_change(&self) -> CollectionChange {
        self.position_change_tracker.change()
    }

    /// Returns the kind of change that has been made to the vertex colors
    /// since the last reset of change tracking.
    pub fn color_change(&self) -> CollectionChange {
        self.color_change_tracker.change()
    }

    /// Returns the kind of change that has been made to the vertex normal
    /// vectors since the last reset of change tracking.
    pub fn normal_vector_change(&self) -> CollectionChange {
        self.normal_vector_change_tracker.change()
    }

    /// Returns the kind of change that has been made to the vertex texture
    /// coordinates since the last reset of change tracking.
    pub fn texture_coord_change(&self) -> CollectionChange {
        self.texture_coord_change_tracker.change()
    }

    /// Returns the kind of change that has been made to the mesh
    /// vertex indices since the last reset of change tracking.
    pub fn index_change(&self) -> CollectionChange {
        self.index_change_tracker.change()
    }

    /// Computes the axis-aligned bounding box enclosing all vertices in the
    /// mesh, or returns [`None`] if the mesh has no vertices.
    pub fn compute_aabb(&self) -> Option<AxisAlignedBox<F>> {
        if self.has_positions() {
            Some(AxisAlignedBox::aabb_for_points(self.positions()))
        } else {
            None
        }
    }

    /// Computes the smallest sphere enclosing all vertices in the mesh, or
    /// returns [`None`] if the mesh has no vertices.
    pub fn compute_bounding_sphere(&self) -> Option<Sphere<F>> {
        self.compute_aabb()
            .as_ref()
            .map(Sphere::bounding_sphere_from_aabb)
    }

    /// Computes new vertex normal vectors for the mesh. Each vertex normal
    /// vector is computed as the average direction of the normals of the
    /// triangles that the vertex is a part of.
    pub fn generate_smooth_normal_vectors(&mut self) {
        let mut summed_normal_vectors = vec![Vector3::zeros(); self.n_vertices()];

        for indices in self.indices.chunks_exact(3) {
            let idx0 = indices[0] as usize;
            let idx1 = indices[1] as usize;
            let idx2 = indices[2] as usize;

            let p0 = &self.positions[idx0].0;
            let p1 = &self.positions[idx1].0;
            let p2 = &self.positions[idx2].0;

            let face_normal_vector = UnitVector3::new_normalize((p1 - p0).cross(&(p2 - p0)));

            summed_normal_vectors[idx0] += face_normal_vector.as_ref();
            summed_normal_vectors[idx1] += face_normal_vector.as_ref();
            summed_normal_vectors[idx2] += face_normal_vector.as_ref();
        }

        self.normal_vectors = summed_normal_vectors
            .into_iter()
            .map(|vector| VertexNormalVector(UnitVector3::new_normalize(vector)))
            .collect();

        self.normal_vector_change_tracker.notify_count_change();
    }

    /// Merges the given mesh into this mesh.
    ///
    /// # Panics
    /// If the two meshes do not have the same set of vertex attributes.
    pub fn merge_with(&mut self, other: &Self) {
        let original_n_indices = self.n_indices();
        let original_n_vertices = self.n_vertices();

        if self.has_positions() {
            assert!(other.has_positions());
            self.positions.extend_from_slice(&other.positions);
            self.position_change_tracker.notify_count_change();

            self.indices.extend_from_slice(&other.indices);
            self.index_change_tracker.notify_count_change();
        }

        if self.has_colors() {
            assert!(other.has_colors());
            self.colors.extend_from_slice(&other.colors);
            self.color_change_tracker.notify_count_change();
        }

        if self.has_normal_vectors() {
            assert!(other.has_normal_vectors());
            self.normal_vectors.extend_from_slice(&other.normal_vectors);
            self.normal_vector_change_tracker.notify_count_change();
        }

        if self.has_texture_coords() {
            assert!(other.has_texture_coords());
            self.texture_coords.extend_from_slice(&other.texture_coords);
            self.texture_coord_change_tracker.notify_count_change();
        }

        let offset = u32::try_from(original_n_vertices).unwrap();
        for idx in &mut self.indices[original_n_indices..] {
            *idx += offset;
        }
    }

    /// Forgets any recorded changes to the vertex positions.
    pub fn reset_position_change_tracking(&self) {
        self.position_change_tracker.reset();
    }

    /// Forgets any recorded changes to the vertex colors.
    pub fn reset_color_change_tracking(&self) {
        self.color_change_tracker.reset();
    }

    /// Forgets any recorded changes to the vertex normal vectors.
    pub fn reset_normal_vector_change_tracking(&self) {
        self.normal_vector_change_tracker.reset();
    }

    /// Forgets any recorded changes to the vertex texture coordinates.
    pub fn reset_texture_coord_change_tracking(&self) {
        self.texture_coord_change_tracker.reset();
    }

    /// Forgets any recorded changes to the vertices.
    pub fn reset_index_change_tracking(&self) {
        self.index_change_tracker.reset();
    }

    /// Forgets any recorded changes to the vertex attributes and indices.
    pub fn reset_change_tracking(&self) {
        self.reset_position_change_tracking();
        self.reset_color_change_tracking();
        self.reset_normal_vector_change_tracking();
        self.reset_texture_coord_change_tracking();
        self.reset_index_change_tracking();
    }
}

impl<F: Float> Point<F> for VertexPosition<F> {
    fn point(&self) -> &Point3<F> {
        &self.0
    }
}
