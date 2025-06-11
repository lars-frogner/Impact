//! Line segment meshes.

use crate::mesh::{VertexColor, VertexPosition};
use impact_containers::{CollectionChange, CollectionChangeTracker};
use impact_math::Float;
use nalgebra::{Point3, Similarity3, UnitQuaternion};

/// A 3D mesh of line segments represented by pairs of vertices.
///
/// The vertices have associated positions and optionally colors. Each
/// consecutive pair of vertices represents the end points of a line segment
/// making up an edge in the mesh. The mesh does not have a concept of faces or
/// surfaces, only edges.
#[derive(Debug)]
pub struct LineSegmentMesh<F: Float> {
    positions: Vec<VertexPosition<F>>,
    colors: Vec<VertexColor<F>>,
    position_change_tracker: CollectionChangeTracker,
    color_change_tracker: CollectionChangeTracker,
}

impl<F: Float> LineSegmentMesh<F> {
    /// Creates a new mesh described by the given vertex positions and colors.
    ///
    /// # Panics
    /// If the length of `colors` is neither zero nor equal to the length of
    /// `positions`.
    pub fn new(positions: Vec<VertexPosition<F>>, colors: Vec<VertexColor<F>>) -> Self {
        let n_vertices = positions.len();

        assert!(
            colors.is_empty() || colors.len() == n_vertices,
            "Mismatching number of colors and positions in line segment mesh"
        );

        Self {
            positions,
            colors,
            position_change_tracker: CollectionChangeTracker::default(),
            color_change_tracker: CollectionChangeTracker::default(),
        }
    }

    /// Returns the number of vertices in the mesh.
    pub fn n_vertices(&self) -> usize {
        self.positions.len()
    }

    /// Returns the number of line segments in the mesh.
    pub fn n_line_segments(&self) -> usize {
        self.n_vertices() / 2
    }

    /// Returns a slice with the positions of the mesh vertices.
    pub fn positions(&self) -> &[VertexPosition<F>] {
        &self.positions
    }

    /// Returns a slice with the colors of the mesh vertices.
    pub fn colors(&self) -> &[VertexColor<F>] {
        &self.colors
    }

    /// Whether the mesh has any vertices.
    pub fn has_positions(&self) -> bool {
        !self.positions.is_empty()
    }

    /// Whether the mesh has any colors.
    pub fn has_colors(&self) -> bool {
        !self.colors.is_empty()
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

    /// Returns an iterator over the mesh line segments, each item containing
    /// the two line segment vertex positions.
    pub fn line_segment_vertex_positions(&self) -> impl Iterator<Item = [&Point3<F>; 2]> {
        self.positions()
            .chunks_exact(2)
            .map(|pair| [&pair[0].0, &pair[1].0])
    }

    /// Applies the given scaling factor to the vertex positions of the mesh.
    pub fn scale(&mut self, scaling: F) {
        for position in &mut self.positions {
            *position = position.scaled(scaling);
        }
    }

    /// Applies the given rotation to the mesh, rotating vertex positions.
    pub fn rotate(&mut self, rotation: &UnitQuaternion<F>) {
        for position in &mut self.positions {
            *position = position.rotated(rotation);
        }
    }

    /// Applies the given similarity transform to the mesh, transforming vertex
    /// positions.
    pub fn transform(&mut self, transform: &Similarity3<F>) {
        for position in &mut self.positions {
            *position = position.transformed(transform);
        }
    }

    /// Sets the color of every vertex to the given color.
    pub fn set_same_color(&mut self, color: VertexColor<F>) {
        self.colors = vec![color; self.positions.len()];
    }

    /// Merges the given mesh into this mesh.
    ///
    /// # Panics
    /// If the two meshes do not have the same set of vertex attributes.
    pub fn merge_with(&mut self, other: &Self) {
        if self.has_positions() {
            assert!(other.has_positions());
            self.positions.extend_from_slice(&other.positions);
            self.position_change_tracker.notify_count_change();
        }

        if self.has_colors() {
            assert!(other.has_colors());
            self.colors.extend_from_slice(&other.colors);
            self.color_change_tracker.notify_count_change();
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

    /// Forgets any recorded changes to the vertex attributes.
    pub fn reset_change_tracking(&self) {
        self.reset_position_change_tracking();
        self.reset_color_change_tracking();
    }
}
