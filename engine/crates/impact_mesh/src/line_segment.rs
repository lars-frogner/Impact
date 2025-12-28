//! Line segment meshes.

use std::fmt;

use crate::{VertexColor, VertexPosition};
use bitflags::bitflags;
use bytemuck::{Pod, Zeroable};
use impact_math::{
    hash::StringHash64, hash64, point::Point3P, quaternion::UnitQuaternion, transform::Similarity3,
    vector::Vector3,
};
use impact_resource::{
    MutableResource, Resource, ResourceDirtyMask, ResourceID, registry::MutableResourceRegistry,
};
use roc_integration::roc;

define_component_type! {
    /// The ID of a [`LineSegmentMesh`].
    #[roc(parents = "Comp")]
    #[repr(C)]
    #[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Zeroable, Pod)]
    pub struct LineSegmentMeshID(pub StringHash64);
}

/// A registry of loaded [`LineSegmentMesh`]es.
pub type LineSegmentMeshRegistry = MutableResourceRegistry<LineSegmentMesh>;

/// A 3D mesh of line segments represented by pairs of vertices.
///
/// The vertices have associated positions and optionally colors. Each
/// consecutive pair of vertices represents the end points of a line segment
/// making up an edge in the mesh. The mesh does not have a concept of faces or
/// surfaces, only edges.
#[derive(Clone, Debug)]
pub struct LineSegmentMesh {
    positions: Vec<VertexPosition>,
    colors: Vec<VertexColor>,
}

bitflags! {
    /// The set of line segment mesh properties that have been modified.
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    pub struct LineSegmentMeshDirtyMask: u8 {
        const POSITIONS = 1 << 0;
        const COLORS    = 1 << 1;
    }
}

#[roc(dependencies = [impact_math::hash::Hash64])]
impl LineSegmentMeshID {
    #[roc(body = "Hashing.hash_str_64(name)")]
    /// Creates a line segment mesh ID hashed from the given name.
    pub fn from_name(name: &str) -> Self {
        Self(hash64!(name))
    }
}

impl From<LineSegmentMeshID> for StringHash64 {
    fn from(id: LineSegmentMeshID) -> Self {
        id.0
    }
}

impl fmt::Display for LineSegmentMeshID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl ResourceID for LineSegmentMeshID {}

#[cfg(feature = "serde")]
impl serde::Serialize for LineSegmentMeshID {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for LineSegmentMeshID {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(LineSegmentMeshID::from_name(&s))
    }
}

impl LineSegmentMesh {
    /// Creates a new mesh described by the given vertex positions and colors.
    ///
    /// # Panics
    /// If the length of `colors` is neither zero nor equal to the length of
    /// `positions`.
    pub fn new(positions: Vec<VertexPosition>, colors: Vec<VertexColor>) -> Self {
        let n_vertices = positions.len();

        assert!(
            colors.is_empty() || colors.len() == n_vertices,
            "Mismatching number of colors and positions in line segment mesh"
        );

        Self { positions, colors }
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
    pub fn positions(&self) -> &[VertexPosition] {
        &self.positions
    }

    /// Returns a slice with the colors of the mesh vertices.
    pub fn colors(&self) -> &[VertexColor] {
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

    /// Returns an iterator over the mesh line segments, each item containing
    /// the two line segment vertex positions.
    pub fn line_segment_vertex_positions(&self) -> impl Iterator<Item = [&Point3P; 2]> {
        self.positions()
            .chunks_exact(2)
            .map(|pair| [&pair[0].0, &pair[1].0])
    }

    /// Applies the given scaling factor to the vertex positions of the mesh.
    pub fn scale(&mut self, scaling: f32, dirty_mask: &mut LineSegmentMeshDirtyMask) {
        for position in &mut self.positions {
            *position = position.scaled(scaling);
        }
        *dirty_mask |= LineSegmentMeshDirtyMask::POSITIONS;
    }

    /// Applies the given rotation to the mesh, rotating the vertex positions.
    pub fn rotate(&mut self, rotation: &UnitQuaternion, dirty_mask: &mut LineSegmentMeshDirtyMask) {
        for position in &mut self.positions {
            *position = position.rotated(rotation);
        }
        *dirty_mask |= LineSegmentMeshDirtyMask::POSITIONS;
    }

    /// Applies the given displacement vector to the mesh, translating the
    /// vertex positions.
    pub fn translate(&mut self, translation: &Vector3, dirty_mask: &mut LineSegmentMeshDirtyMask) {
        for position in &mut self.positions {
            *position = position.translated(translation);
        }
        *dirty_mask |= LineSegmentMeshDirtyMask::POSITIONS;
    }

    /// Applies the given similarity transform to the mesh, transforming the
    /// vertex positions.
    pub fn transform(
        &mut self,
        transform: &Similarity3,
        dirty_mask: &mut LineSegmentMeshDirtyMask,
    ) {
        for position in &mut self.positions {
            *position = position.transformed(transform);
        }
        *dirty_mask |= LineSegmentMeshDirtyMask::POSITIONS;
    }

    /// Sets the color of every vertex to the given color.
    pub fn set_same_color(
        &mut self,
        color: VertexColor,
        dirty_mask: &mut LineSegmentMeshDirtyMask,
    ) {
        self.colors = vec![color; self.positions.len()];
        *dirty_mask |= LineSegmentMeshDirtyMask::COLORS;
    }

    /// Merges the given mesh into this mesh.
    ///
    /// # Panics
    /// If the two meshes do not have the same set of vertex attributes.
    pub fn merge_with(&mut self, other: &Self, dirty_mask: &mut LineSegmentMeshDirtyMask) {
        if self.has_positions() {
            assert!(other.has_positions());
            self.positions.extend_from_slice(&other.positions);
            *dirty_mask |= LineSegmentMeshDirtyMask::POSITIONS;
        }

        if self.has_colors() {
            assert!(other.has_colors());
            self.colors.extend_from_slice(&other.colors);
            *dirty_mask |= LineSegmentMeshDirtyMask::COLORS;
        }
    }
}

impl Resource for LineSegmentMesh {
    type ID = LineSegmentMeshID;
}

impl MutableResource for LineSegmentMesh {
    type DirtyMask = LineSegmentMeshDirtyMask;
}

impl ResourceDirtyMask for LineSegmentMeshDirtyMask {
    fn empty() -> Self {
        Self::empty()
    }

    fn full() -> Self {
        Self::all()
    }
}
