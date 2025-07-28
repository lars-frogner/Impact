//! Triangle and line segment meshes.

#[macro_use]
mod macros;

pub mod builtin;
pub mod generation;
pub mod gpu_resource;
pub mod import;
pub mod io;
mod line_segment;
pub mod setup;
pub mod texture_projection;
mod triangle;

pub use line_segment::*;
pub use triangle::*;

use bitflags::bitflags;
use bytemuck::{Pod, Zeroable};
use impact_geometry::Point;
use impact_math::{Float, StringHash64, hash64};
use nalgebra::{
    Point3, Similarity3, UnitQuaternion, UnitVector3, Vector2, Vector3, Vector4, vector,
};
use roc_integration::roc;
use std::{
    fmt::{self, Debug},
    ops::Neg,
};

/// The persistent ID of a [`TriangleMesh`] or [`LineSegmentMesh`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MeshID {
    Triangle(TriangleMeshID),
    LineSegment(LineSegmentMeshID),
}

/// Handle to a [`TriangleMesh`] or [`LineSegmentMesh`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MeshHandle {
    Triangle(TriangleMeshHandle),
    LineSegment(LineSegmentMeshHandle),
}

/// The geometric primitive used for a mesh.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MeshPrimitive {
    Triangle,
    LineSegment,
}

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

/// The unit normal vector of a mesh at a vertex position.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct VertexNormalVector<F: Float>(pub UnitVector3<F>);

/// The (u, v) texture coordinates of a mesh at a vertex position.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct VertexTextureCoords<F: Float>(pub Vector2<F>);

/// The rotation quaternion from local tangent space to model space at a vertex
/// position. The handedness of the tangent basis is encoded in the sign of the
/// real component (when it is negative, the basis is really left-handed and the
/// y-component of the tangent space vector to transform to model space should
/// be negated before applying the rotation to it).
#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct VertexTangentSpaceQuaternion<F: Float>(pub UnitQuaternion<F>);

/// The RGBA color of a mesh vertex.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct VertexColor<F: Float>(pub Vector4<F>);

bitflags! {
    /// Bitflag encoding a set of [`VertexAttribute`]s.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct VertexAttributeSet: u8 {
        const POSITION                 = 1 << 0;
        const NORMAL_VECTOR            = 1 << 1;
        const TEXTURE_COORDS           = 1 << 2;
        const TANGENT_SPACE_QUATERNION = 1 << 3;
        const COLOR                    = 1 << 4;
    }
}

/// Whether the front faces of a triangle mesh are oriented toward the outside
/// or the inside.
#[roc(parents = "Mesh")]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum FrontFaceSide {
    Outside,
    Inside,
}

/// The total number of available vertex attribute types.
pub const N_VERTEX_ATTRIBUTES: usize = 5;

/// The bitflag of each individual vertex attribute, ordered according to
/// [`VertexAttribute::GLOBAL_INDEX`].
pub const VERTEX_ATTRIBUTE_FLAGS: [VertexAttributeSet; N_VERTEX_ATTRIBUTES] = [
    VertexAttributeSet::POSITION,
    VertexAttributeSet::NORMAL_VECTOR,
    VertexAttributeSet::TEXTURE_COORDS,
    VertexAttributeSet::TANGENT_SPACE_QUATERNION,
    VertexAttributeSet::COLOR,
];

/// The name of each individual vertex attribute, ordered according to
/// [`VertexAttribute::GLOBAL_INDEX`].
pub const VERTEX_ATTRIBUTE_NAMES: [&str; N_VERTEX_ATTRIBUTES] = [
    "position",
    "normal vector",
    "texture coords",
    "tangent space quaternion",
    "color",
];

#[roc(dependencies = [impact_math::Hash64])]
impl TriangleMeshID {
    #[roc(body = "Hashing.hash_str_64(name)")]
    /// Creates a triangle mesh ID hashed from the given name.
    pub fn from_name(name: &str) -> Self {
        Self(hash64!(name))
    }
}

impl From<TriangleMeshID> for StringHash64 {
    fn from(id: TriangleMeshID) -> Self {
        id.0
    }
}

#[roc(dependencies = [impact_math::Hash64])]
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

impl fmt::Display for MeshHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Triangle(id) => write!(f, "{id}"),
            Self::LineSegment(id) => write!(f, "{id}"),
        }
    }
}

impl<F: Float> VertexPosition<F> {
    /// Returns the binding location of the GPU vertex buffer for position.
    pub const fn binding_location() -> u32 {
        0
    }

    /// Returns the position scaled by the given scaling factor.
    pub fn scaled(&self, scaling: F) -> Self {
        Self(self.0.coords.scale(scaling).into())
    }

    /// Returns the position rotated by the given unit quaternion.
    pub fn rotated(&self, rotation: &UnitQuaternion<F>) -> Self {
        Self(rotation * self.0)
    }

    /// Returns the position translated by the given displacement vector.
    pub fn translated(&self, translation: &Vector3<F>) -> Self {
        Self(self.0 + translation)
    }

    /// Returns the position transformed by the given similarity transform.
    pub fn transformed(&self, transform: &Similarity3<F>) -> Self {
        Self(transform * self.0)
    }
}

impl<F: Float> VertexNormalVector<F> {
    /// Returns the normal vector rotated by the given unit quaternion.
    pub fn rotated(&self, rotation: &UnitQuaternion<F>) -> Self {
        Self(rotation * self.0)
    }

    /// Returns the normal vector transformed by the given similarity transform.
    pub fn transformed(&self, transform: &Similarity3<F>) -> Self {
        self.rotated(&transform.isometry.rotation)
    }
}

impl<F: Float> VertexTangentSpaceQuaternion<F> {
    /// Returns the tangent space quaternion rotated by the given unit
    /// quaternion.
    pub fn rotated(&self, rotation: &UnitQuaternion<F>) -> Self {
        let mut rotated_tangent_space_quaternion = rotation * self.0;

        // Preserve encoding of tangent space handedness in real component of
        // tangent space quaternion
        if (rotated_tangent_space_quaternion.w < F::ZERO) != (self.0.w < F::ZERO) {
            rotated_tangent_space_quaternion =
                UnitQuaternion::new_unchecked(rotated_tangent_space_quaternion.neg());
        }

        Self(rotated_tangent_space_quaternion)
    }

    /// Returns the tangent space quaternion transformed by the given similarity
    /// transform.
    pub fn transformed(&self, transform: &Similarity3<F>) -> Self {
        self.rotated(&transform.isometry.rotation)
    }
}

impl<F: Float> VertexColor<F> {
    pub const BLACK: Self = Self(vector![F::ZERO, F::ZERO, F::ZERO, F::ONE]);
    pub const WHITE: Self = Self(vector![F::ONE, F::ONE, F::ONE, F::ONE]);
    pub const RED: Self = Self(vector![F::ONE, F::ZERO, F::ZERO, F::ONE]);
    pub const GREEN: Self = Self(vector![F::ZERO, F::ONE, F::ZERO, F::ONE]);
    pub const BLUE: Self = Self(vector![F::ZERO, F::ZERO, F::ONE, F::ONE]);
    pub const CYAN: Self = Self(vector![F::ZERO, F::ONE, F::ONE, F::ONE]);
    pub const MAGENTA: Self = Self(vector![F::ONE, F::ZERO, F::ONE, F::ONE]);
    pub const YELLOW: Self = Self(vector![F::ONE, F::ONE, F::ZERO, F::ONE]);

    pub fn with_alpha(self, alpha: F) -> Self {
        let mut color = self.0;
        color.w = alpha;
        Self(color)
    }
}

impl<F: Float> VertexAttribute for VertexPosition<F> {
    const GLOBAL_INDEX: usize = 0;
}

impl<F: Float> VertexAttribute for VertexNormalVector<F> {
    const GLOBAL_INDEX: usize = 1;
}

impl<F: Float> VertexAttribute for VertexTextureCoords<F> {
    const GLOBAL_INDEX: usize = 2;
}

impl<F: Float> VertexAttribute for VertexTangentSpaceQuaternion<F> {
    const GLOBAL_INDEX: usize = 3;
}

impl<F: Float> VertexAttribute for VertexColor<F> {
    const GLOBAL_INDEX: usize = 4;
}

impl fmt::Display for VertexAttributeSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{ ")?;
        for (&attribute, name) in VERTEX_ATTRIBUTE_FLAGS
            .iter()
            .zip(VERTEX_ATTRIBUTE_NAMES.iter())
        {
            if self.contains(attribute) {
                write!(f, "`{name}` ")?;
            }
        }
        write!(f, "}}")
    }
}

impl<F: Float> Point<F> for VertexPosition<F> {
    fn point(&self) -> &Point3<F> {
        &self.0
    }
}
