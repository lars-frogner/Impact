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

use impact_math::transform::Similarity3;
pub use line_segment::*;
pub use triangle::*;

use bitflags::bitflags;
use bytemuck::{Pod, Zeroable};
use nalgebra::{Point3, UnitQuaternion, UnitVector3, Vector2, Vector3, Vector4, vector};
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
pub struct VertexPosition(pub Point3<f32>);

/// The unit normal vector of a mesh at a vertex position.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct VertexNormalVector(pub UnitVector3<f32>);

/// The (u, v) texture coordinates of a mesh at a vertex position.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct VertexTextureCoords(pub Vector2<f32>);

/// The rotation quaternion from local tangent space to model space at a vertex
/// position. The handedness of the tangent basis is encoded in the sign of the
/// real component (when it is negative, the basis is really left-handed and the
/// y-component of the tangent space vector to transform to model space should
/// be negated before applying the rotation to it).
#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct VertexTangentSpaceQuaternion(pub UnitQuaternion<f32>);

/// The RGBA color of a mesh vertex.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct VertexColor(pub Vector4<f32>);

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

impl fmt::Display for MeshID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Triangle(id) => write!(f, "{id}"),
            Self::LineSegment(id) => write!(f, "{id}"),
        }
    }
}

impl VertexPosition {
    /// Returns the binding location of the GPU vertex buffer for position.
    pub const fn binding_location() -> u32 {
        0
    }

    /// Returns the position scaled by the given scaling factor.
    pub fn scaled(&self, scaling: f32) -> Self {
        Self(self.0.coords.scale(scaling).into())
    }

    /// Returns the position rotated by the given unit quaternion.
    pub fn rotated(&self, rotation: &UnitQuaternion<f32>) -> Self {
        Self(rotation * self.0)
    }

    /// Returns the position translated by the given displacement vector.
    pub fn translated(&self, translation: &Vector3<f32>) -> Self {
        Self(self.0 + translation)
    }

    /// Returns the position transformed by the given similarity transform.
    pub fn transformed(&self, transform: &Similarity3) -> Self {
        Self(transform.transform_point(&self.0))
    }
}

impl VertexNormalVector {
    /// Returns the normal vector rotated by the given unit quaternion.
    pub fn rotated(&self, rotation: &UnitQuaternion<f32>) -> Self {
        Self(rotation * self.0)
    }

    /// Returns the normal vector transformed by the given similarity transform.
    pub fn transformed(&self, transform: &Similarity3) -> Self {
        self.rotated(transform.rotation())
    }
}

impl VertexTangentSpaceQuaternion {
    /// Returns the tangent space quaternion rotated by the given unit
    /// quaternion.
    pub fn rotated(&self, rotation: &UnitQuaternion<f32>) -> Self {
        let mut rotated_tangent_space_quaternion = rotation * self.0;

        // Preserve encoding of tangent space handedness in real component of
        // tangent space quaternion
        if (rotated_tangent_space_quaternion.w < 0.0) != (self.0.w < 0.0) {
            rotated_tangent_space_quaternion =
                UnitQuaternion::new_unchecked(rotated_tangent_space_quaternion.neg());
        }

        Self(rotated_tangent_space_quaternion)
    }

    /// Returns the tangent space quaternion transformed by the given similarity
    /// transform.
    pub fn transformed(&self, transform: &Similarity3) -> Self {
        self.rotated(transform.rotation())
    }
}

impl VertexColor {
    pub const BLACK: Self = Self(vector![0.0, 0.0, 0.0, 1.0]);
    pub const WHITE: Self = Self(vector![1.0, 1.0, 1.0, 1.0]);
    pub const RED: Self = Self(vector![1.0, 0.0, 0.0, 1.0]);
    pub const GREEN: Self = Self(vector![0.0, 1.0, 0.0, 1.0]);
    pub const BLUE: Self = Self(vector![0.0, 0.0, 1.0, 1.0]);
    pub const CYAN: Self = Self(vector![0.0, 1.0, 1.0, 1.0]);
    pub const MAGENTA: Self = Self(vector![1.0, 0.0, 1.0, 1.0]);
    pub const YELLOW: Self = Self(vector![1.0, 1.0, 0.0, 1.0]);

    pub fn with_alpha(self, alpha: f32) -> Self {
        let mut color = self.0;
        color.w = alpha;
        Self(color)
    }
}

impl VertexAttribute for VertexPosition {
    const GLOBAL_INDEX: usize = 0;
}

impl VertexAttribute for VertexNormalVector {
    const GLOBAL_INDEX: usize = 1;
}

impl VertexAttribute for VertexTextureCoords {
    const GLOBAL_INDEX: usize = 2;
}

impl VertexAttribute for VertexTangentSpaceQuaternion {
    const GLOBAL_INDEX: usize = 3;
}

impl VertexAttribute for VertexColor {
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
