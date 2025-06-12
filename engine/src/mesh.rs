//! Triangle and line segment meshes.

pub mod buffer;
pub mod components;
pub mod entity;
pub mod generation;
pub mod line_segment;
pub mod texture_projection;
pub mod triangle;

use crate::{io, mesh::line_segment::LineSegmentMesh};
use anyhow::{Context, Result, anyhow, bail};
use bitflags::bitflags;
use bytemuck::{Pod, Zeroable};
use impact_containers::{HashMap, HashSet};
use impact_geometry::Point;
use impact_math::{Float, hash64, stringhash64_newtype};
use log::debug;
use nalgebra::{Point3, Similarity3, UnitQuaternion, UnitVector3, Vector2, Vector4, vector};
use roc_integration::roc;
use serde::{Deserialize, Serialize};
use std::{
    collections::hash_map::Entry,
    fmt::Debug,
    ops::Neg,
    path::{Path, PathBuf},
};
use texture_projection::{PlanarTextureProjection, TextureProjectionSpecification};
use triangle::TriangleMesh;

stringhash64_newtype!(
    /// Identifier for specific meshes.
    /// Wraps a [`StringHash64`](impact_math::StringHash64).
    #[roc(parents = "Mesh")]
    [pub] MeshID
);

/// The geometric primitive used for a mesh.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MeshPrimitive {
    Triangle,
    LineSegment,
}

/// Repository where [`TriangleMesh`]es are stored under unique [`MeshID`]s.
#[derive(Debug, Default)]
pub struct MeshRepository {
    triangle_meshes: HashMap<MeshID, TriangleMesh<f32>>,
    line_segment_meshes: HashMap<MeshID, LineSegmentMesh<f32>>,
}

/// Record of the state of a [`MeshRepository`].
#[derive(Clone, Debug)]
pub struct MeshRepositoryState {
    triangle_mesh_ids: HashSet<MeshID>,
    line_segment_mesh_ids: HashSet<MeshID>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TriangleMeshSpecification {
    pub name: String,
    pub file_path: PathBuf,
    pub texture_projection: Option<TextureProjectionSpecification>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TriangleMeshFileFormat {
    Obj,
    Ply,
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
impl MeshID {
    #[roc(body = "Hashing.hash_str_64(name)")]
    /// Creates a mesh ID hashed from the given name.
    pub fn from_name(name: &str) -> Self {
        Self(hash64!(name))
    }
}

impl MeshRepository {
    /// Creates a new empty mesh repository.
    pub fn new() -> Self {
        Self {
            triangle_meshes: HashMap::default(),
            line_segment_meshes: HashMap::default(),
        }
    }

    /// Generates the meshes that should be available by default and inserts
    /// them into the repository.
    pub fn create_default_meshes(&mut self) {
        self.triangle_meshes.insert(
            screen_filling_quad_mesh_id(),
            TriangleMesh::create_screen_filling_quad(),
        );

        self.triangle_meshes.insert(
            spherical_light_volume_mesh_id(),
            TriangleMesh::create_spherical_light_volume(16),
        );

        self.triangle_meshes.insert(
            skybox_mesh_id(),
            TriangleMesh::create_box(1.0, 1.0, 1.0, FrontFaceSide::Inside),
        );

        self.triangle_meshes.insert(
            bounding_sphere_mesh_id(),
            TriangleMesh::create_colored_unit_sphere(32, VertexColor::YELLOW.with_alpha(0.15)),
        );

        self.line_segment_meshes.insert(
            reference_frame_axes_mesh_id(),
            LineSegmentMesh::create_reference_frame_axes(),
        );
    }

    /// Loads all meshes in the given specifications and stores them in the
    /// repository
    ///
    /// # Errors
    /// See [`Self::load_specified_mesh`].
    pub fn load_specified_meshes(
        &mut self,
        triangle_mesh_specifications: &[TriangleMeshSpecification],
    ) -> Result<()> {
        for specification in triangle_mesh_specifications {
            self.load_specified_triangle_mesh(specification)?;
        }
        Ok(())
    }

    /// Loads the triangle mesh in the given specification and stores it in the
    /// repository.
    ///
    /// # Errors
    /// Returns an error if:
    /// - Another mesh with the same name is already loaded.
    /// - The file format is not supported.
    /// - The file can not be found or loaded as a mesh.
    pub fn load_specified_triangle_mesh(
        &mut self,
        specification: &TriangleMeshSpecification,
    ) -> Result<()> {
        debug!(
            "Loading triangle mesh `{}` from {}",
            specification.name,
            specification.file_path.display()
        );

        let file_path = &specification.file_path;
        let file_format = specification.resolve_file_format()?;

        let mesh_id = MeshID(hash64!(&specification.name));
        if self.has_triangle_mesh(mesh_id) {
            bail!(
                "Tried to load triangle mesh under already existing name: {}",
                specification.name
            );
        }

        let mut mesh = match file_format {
            TriangleMeshFileFormat::Obj => io::obj::read_mesh_from_obj_file(file_path),
            TriangleMeshFileFormat::Ply => io::ply::read_mesh_from_ply_file(file_path),
        }
        .with_context(|| format!("Failed to load triangle mesh from {}", file_path.display()))?;

        match &specification.texture_projection {
            None => {}
            Some(TextureProjectionSpecification::Planar {
                origin,
                u_vector,
                v_vector,
            }) => {
                let projection = PlanarTextureProjection::new(*origin, *u_vector, *v_vector)
                    .with_context(|| {
                        format!(
                            "Invalid planar texture projection for triangle mesh `{}`",
                            specification.name
                        )
                    })?;
                mesh.generate_texture_coords(&projection);
            }
        }

        self.add_triangle_mesh(mesh_id, mesh)?;

        Ok(())
    }

    /// Records the current state of the repository and returns it as a
    /// [`MeshRepositoryState`].
    pub fn record_state(&self) -> MeshRepositoryState {
        MeshRepositoryState {
            triangle_mesh_ids: self.triangle_meshes.keys().cloned().collect(),
            line_segment_mesh_ids: self.line_segment_meshes.keys().cloned().collect(),
        }
    }

    /// Returns a reference to the [`TriangleMesh`] with the given ID, or
    /// [`None`] if no triangle mesh with that ID is present.
    pub fn get_triangle_mesh(&self, mesh_id: MeshID) -> Option<&TriangleMesh<f32>> {
        self.triangle_meshes.get(&mesh_id)
    }

    /// Returns a reference to the [`LineSegmentMesh`] with the given ID, or
    /// [`None`] if no line segment mesh with that ID is present.
    pub fn get_line_segment_mesh(&self, mesh_id: MeshID) -> Option<&LineSegmentMesh<f32>> {
        self.line_segment_meshes.get(&mesh_id)
    }

    /// Returns a mutable reference to the [`TriangleMesh`] with the given ID,
    /// or [`None`] if no triangle mesh with that ID is present.
    pub fn get_triangle_mesh_mut(&mut self, mesh_id: MeshID) -> Option<&mut TriangleMesh<f32>> {
        self.triangle_meshes.get_mut(&mesh_id)
    }

    /// Returns a mutable reference to the [`LineSegmentMesh`] with the given
    /// ID, or [`None`] if no line segment mesh with that ID is present.
    pub fn get_line_segment_mesh_mut(
        &mut self,
        mesh_id: MeshID,
    ) -> Option<&mut LineSegmentMesh<f32>> {
        self.line_segment_meshes.get_mut(&mesh_id)
    }

    /// Whether a triangle mesh with the given ID exists in the repository.
    pub fn has_triangle_mesh(&self, mesh_id: MeshID) -> bool {
        self.triangle_meshes.contains_key(&mesh_id)
    }

    /// Whether a line segment mesh with the given ID exists in the repository.
    pub fn has_line_segment_mesh(&self, mesh_id: MeshID) -> bool {
        self.line_segment_meshes.contains_key(&mesh_id)
    }

    /// Returns a reference to the [`HashMap`] storing all triangle meshes.
    pub fn triangle_meshes(&self) -> &HashMap<MeshID, TriangleMesh<f32>> {
        &self.triangle_meshes
    }

    /// Returns a reference to the [`HashMap`] storing all line segment meshes.
    pub fn line_segment_meshes(&self) -> &HashMap<MeshID, LineSegmentMesh<f32>> {
        &self.line_segment_meshes
    }

    /// Includes the given triangle mesh in the repository under the given ID.
    ///
    /// # Errors
    /// Returns an error if a triangle mesh with the given ID already
    /// exists. The repository will remain unchanged.
    pub fn add_triangle_mesh(&mut self, mesh_id: MeshID, mesh: TriangleMesh<f32>) -> Result<()> {
        match self.triangle_meshes.entry(mesh_id) {
            Entry::Vacant(entry) => {
                entry.insert(mesh);
                Ok(())
            }
            Entry::Occupied(_) => Err(anyhow!(
                "Triangle mesh {} already present in repository",
                mesh_id
            )),
        }
    }

    /// Includes the given line segment mesh in the repository under the given
    /// ID.
    ///
    /// # Errors
    /// Returns an error if a line segment mesh with the given ID already
    /// exists. The repository will remain unchanged.
    pub fn add_line_segment_mesh(
        &mut self,
        mesh_id: MeshID,
        mesh: LineSegmentMesh<f32>,
    ) -> Result<()> {
        match self.line_segment_meshes.entry(mesh_id) {
            Entry::Vacant(entry) => {
                entry.insert(mesh);
                Ok(())
            }
            Entry::Occupied(_) => Err(anyhow!(
                "Line segment mesh {} already present in repository",
                mesh_id
            )),
        }
    }

    /// Includes the given triangle mesh in the repository under the given ID,
    /// unless a triangle mesh with the same ID is already present.
    pub fn add_triangle_mesh_unless_present(&mut self, mesh_id: MeshID, mesh: TriangleMesh<f32>) {
        let _ = self.add_triangle_mesh(mesh_id, mesh);
    }

    /// Includes the given line segment mesh in the repository under the given
    /// ID, unless a line segment mesh with the same ID is already present.
    pub fn add_line_segment_mesh_unless_present(
        &mut self,
        mesh_id: MeshID,
        mesh: LineSegmentMesh<f32>,
    ) {
        let _ = self.add_line_segment_mesh(mesh_id, mesh);
    }

    /// Removes the meshes that are not part of the given repository state.
    pub fn reset_to_state(&mut self, state: &MeshRepositoryState) {
        self.triangle_meshes
            .retain(|mesh_id, _| state.triangle_mesh_ids.contains(mesh_id));
        self.line_segment_meshes
            .retain(|mesh_id, _| state.line_segment_mesh_ids.contains(mesh_id));
    }
}

impl TriangleMeshSpecification {
    /// Resolves all paths in the specification by prepending the given root
    /// path to all paths.
    pub fn resolve_paths(&mut self, root_path: &Path) {
        self.file_path = root_path.join(&self.file_path);
    }

    fn resolve_file_format(&self) -> Result<TriangleMeshFileFormat> {
        let Some(extension) = self.file_path.extension() else {
            bail!(
                "Missing extension for triangle mesh file {}",
                self.file_path.display()
            );
        };
        match &*extension.to_string_lossy().to_lowercase() {
            "obj" => Ok(TriangleMeshFileFormat::Obj),
            "ply" => Ok(TriangleMeshFileFormat::Ply),
            other => Err(anyhow!(
                "Unsupported triangle mesh file format {other} for triangle mesh file {}",
                self.file_path.display()
            )),
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

    /// Returns the position transformed by the given similarity transform.
    pub fn transformed(&self, transform: &Similarity3<F>) -> Self {
        Self(transform * self.0)
    }
}

impl<F: Float> VertexNormalVector<F> {
    /// Returns the normal vector transformed by the given similarity transform.
    pub fn transformed(&self, transform: &Similarity3<F>) -> Self {
        Self(transform.isometry.rotation * self.0)
    }
}

impl<F: Float> VertexTangentSpaceQuaternion<F> {
    /// Returns the tangent space quaternion transformed by the given similarity
    /// transform.
    pub fn transformed(&self, transform: &Similarity3<F>) -> Self {
        let mut rotated_tangent_space_quaternion = transform.isometry.rotation * self.0;

        // Preserve encoding of tangent space handedness in real component of
        // tangent space quaternion
        if (rotated_tangent_space_quaternion.w < F::ZERO) != (self.0.w < F::ZERO) {
            rotated_tangent_space_quaternion =
                UnitQuaternion::new_unchecked(rotated_tangent_space_quaternion.neg());
        }

        Self(rotated_tangent_space_quaternion)
    }
}

impl<F: Float> VertexColor<F> {
    pub const BLACK: Self = Self(vector![F::ZERO, F::ZERO, F::ZERO, F::ONE]);
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

impl std::fmt::Display for VertexAttributeSet {
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

impl<F: Float> Point<F> for VertexPosition<F> {
    fn point(&self) -> &Point3<F> {
        &self.0
    }
}

macro_rules! define_mesh_ids {
    (
        $(
            $(#[$meta:meta])*
            fn $fn_name:ident() => $desc:expr;
        )+
    ) => {
        paste::paste! {
            $(
                $(#[$meta])*
                pub fn $fn_name() -> MeshID {
                    *[<$fn_name:upper>]
                }

                static [<$fn_name:upper>]: std::sync::LazyLock<MeshID> =
                    std::sync::LazyLock::new(|| MeshID(impact_math::hash64!($desc)));
            )+
        }
    };
}

define_mesh_ids! {
    /// The ID of a [`TriangleMesh`] in the [`MeshRepository`] generated by
    /// [`TriangleMesh::create_screen_filling_quad`];
    fn screen_filling_quad_mesh_id() => "Screen filling quad mesh";

    /// The ID of a [`TriangleMesh`] in the [`MeshRepository`] generated by
    /// [`TriangleMesh::create_spherical_light_volume`].
    fn spherical_light_volume_mesh_id() => "Spherical light volume mesh";

    /// The ID of a [`TriangleMesh`] in the [`MeshRepository`] generated by
    /// [`TriangleMesh::create_box`] with unit extents and front faces on the
    /// inside.
    fn skybox_mesh_id() => "Skybox mesh";

    /// The ID of a [`TriangleMesh`] in the [`MeshRepository`] generated by
    /// [`TriangleMesh::create_colored_unit_sphere`] with a semi-transparent
    /// yellow color.
    fn bounding_sphere_mesh_id() => "Bounding sphere mesh";

    /// The ID of a [`LineSegmentMesh`] in the [`MeshRepository`] generated by
    /// [`LineSegmentMesh::create_reference_frame_axes`].
    fn reference_frame_axes_mesh_id() => "Reference frame axes mesh";
}
