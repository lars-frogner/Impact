//! Triangle meshes.

use crate::mesh::{
    VertexColor, VertexNormalVector, VertexPosition, VertexTangentSpaceQuaternion,
    VertexTextureCoords, texture_projection::TextureProjection,
};
use approx::{abs_diff_eq, abs_diff_ne};
use impact_containers::{CollectionChange, CollectionChangeTracker};
use impact_geometry::{AxisAlignedBox, Sphere};
use impact_math::Float;
use nalgebra::{Matrix3x2, Point3, Similarity3, UnitQuaternion, UnitVector3, Vector3};
use std::fmt::Debug;

/// A 3D mesh of triangles represented by vertices and indices.
///
/// The vertices are typically unique, and they have associated positions and
/// potentially other attributes. Each index refers to a vertex, and the
/// sequence of indices describes the triangles making up the mesh faces.
#[derive(Debug)]
pub struct TriangleMesh<F: Float> {
    positions: Vec<VertexPosition<F>>,
    normal_vectors: Vec<VertexNormalVector<F>>,
    texture_coords: Vec<VertexTextureCoords<F>>,
    tangent_space_quaternions: Vec<VertexTangentSpaceQuaternion<F>>,
    colors: Vec<VertexColor<F>>,
    indices: Vec<u32>,
    position_change_tracker: CollectionChangeTracker,
    normal_vector_change_tracker: CollectionChangeTracker,
    texture_coord_change_tracker: CollectionChangeTracker,
    tangent_space_quaternion_change_tracker: CollectionChangeTracker,
    color_change_tracker: CollectionChangeTracker,
    index_change_tracker: CollectionChangeTracker,
}

impl<F: Float> TriangleMesh<F> {
    /// Creates a new mesh described by the given vertex attributes and indices.
    ///
    /// # Panics
    /// If the length of `normal_vectors`, `texture_coords`,
    /// `tangent_space_quaternions` and `colors` are neither zero nor equal to
    /// the length of `positions`.
    pub fn new(
        positions: Vec<VertexPosition<F>>,
        normal_vectors: Vec<VertexNormalVector<F>>,
        texture_coords: Vec<VertexTextureCoords<F>>,
        tangent_space_quaternions: Vec<VertexTangentSpaceQuaternion<F>>,
        colors: Vec<VertexColor<F>>,
        indices: Vec<u32>,
    ) -> Self {
        let n_vertices = positions.len();

        assert!(
            normal_vectors.is_empty() || normal_vectors.len() == n_vertices,
            "Mismatching number of normal vectors and positions in triangle mesh"
        );
        assert!(
            texture_coords.is_empty() || texture_coords.len() == n_vertices,
            "Mismatching number of texture coordinates and positions in triangle mesh"
        );
        assert!(
            tangent_space_quaternions.is_empty() || tangent_space_quaternions.len() == n_vertices,
            "Mismatching number of tangent space quaternions and positions in triangle mesh"
        );
        assert!(
            colors.is_empty() || colors.len() == n_vertices,
            "Mismatching number of colors and positions in triangle mesh"
        );

        Self {
            positions,
            normal_vectors,
            texture_coords,
            tangent_space_quaternions,
            colors,
            indices,
            position_change_tracker: CollectionChangeTracker::default(),
            normal_vector_change_tracker: CollectionChangeTracker::default(),
            texture_coord_change_tracker: CollectionChangeTracker::default(),
            tangent_space_quaternion_change_tracker: CollectionChangeTracker::default(),
            color_change_tracker: CollectionChangeTracker::default(),
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

    /// Returns a slice with the normal vectors of the mesh vertices.
    pub fn normal_vectors(&self) -> &[VertexNormalVector<F>] {
        &self.normal_vectors
    }

    /// Returns a slice with the texture coordinates of the mesh vertices.
    pub fn texture_coords(&self) -> &[VertexTextureCoords<F>] {
        &self.texture_coords
    }

    /// Returns a slice with the tangent space quaternions of the mesh vertices.
    pub fn tangent_space_quaternions(&self) -> &[VertexTangentSpaceQuaternion<F>] {
        &self.tangent_space_quaternions
    }

    /// Returns a slice with the colors of the mesh vertices.
    pub fn colors(&self) -> &[VertexColor<F>] {
        &self.colors
    }

    /// Returns a slice with the vertex indices describing the faces of the
    /// mesh.
    pub fn indices(&self) -> &[u32] {
        &self.indices
    }

    /// Whether the mesh has any indices.
    pub fn has_indices(&self) -> bool {
        !self.indices.is_empty()
    }

    /// Whether the mesh has any vertices.
    pub fn has_positions(&self) -> bool {
        !self.positions.is_empty()
    }

    /// Whether the vertices have associated normal vectors.
    pub fn has_normal_vectors(&self) -> bool {
        !self.normal_vectors.is_empty()
    }

    /// Whether the vertices have associated texture coordinates.
    pub fn has_texture_coords(&self) -> bool {
        !self.texture_coords.is_empty()
    }

    /// Whether the vertices have associated tangent space quaternions.
    pub fn has_tangent_space_quaternions(&self) -> bool {
        !self.tangent_space_quaternions.is_empty()
    }

    /// Whether the vertices have associated colors.
    pub fn has_colors(&self) -> bool {
        !self.colors.is_empty()
    }

    /// Returns the kind of change that has been made to the vertex positions
    /// since the last reset of change tracking.
    pub fn position_change(&self) -> CollectionChange {
        self.position_change_tracker.change()
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

    /// Returns the kind of change that has been made to the vertex tangent
    /// space quaternions since the last reset of change tracking.
    pub fn tangent_space_quaternion_change(&self) -> CollectionChange {
        self.tangent_space_quaternion_change_tracker.change()
    }

    /// Returns the kind of change that has been made to the vertex colors
    /// since the last reset of change tracking.
    pub fn color_change(&self) -> CollectionChange {
        self.color_change_tracker.change()
    }

    /// Returns the kind of change that has been made to the mesh
    /// vertex indices since the last reset of change tracking.
    pub fn index_change(&self) -> CollectionChange {
        self.index_change_tracker.change()
    }

    /// Returns an iterator over the mesh triangles, each item containing the
    /// three triangle indices.
    pub fn triangle_indices(&self) -> impl Iterator<Item = [usize; 3]> {
        self.indices().chunks_exact(3).map(|indices| {
            [
                indices[0] as usize,
                indices[1] as usize,
                indices[2] as usize,
            ]
        })
    }

    /// Returns an iterator over the mesh triangles, each item containing the
    /// three triangle vertex positions.
    pub fn triangle_vertex_positions(&self) -> impl Iterator<Item = [&Point3<F>; 3]> {
        self.triangle_indices().map(|[i, j, k]| {
            [
                &self.positions[i].0,
                &self.positions[j].0,
                &self.positions[k].0,
            ]
        })
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

    /// Finds a sphere enclosing all vertices in the mesh, or returns [`None`]
    /// if the mesh has no vertices.
    pub fn compute_bounding_sphere(&self) -> Option<Sphere<F>> {
        if self.has_positions() {
            Some(Sphere::bounding_sphere_for_points(self.positions()))
        } else {
            None
        }
    }

    /// Computes new vertex normal vectors for the mesh. Each vertex normal
    /// vector is computed as the average direction of the normals of the
    /// triangles that the vertex is a part of.
    ///
    /// # Panics
    /// If the mesh misses positions.
    pub fn generate_smooth_normal_vectors(&mut self) {
        assert!(self.has_positions());

        let mut summed_normal_vectors = vec![Vector3::zeros(); self.n_vertices()];

        for [idx0, idx1, idx2] in self.triangle_indices() {
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

    /// Uses the given projection to compute new texture coordinates for the
    /// mesh.
    ///
    /// # Panics
    /// If the mesh misses positions.
    pub fn generate_texture_coords(&mut self, projection: &impl TextureProjection<F>) {
        assert!(self.has_positions());

        self.texture_coords.clear();
        self.texture_coords.reserve(self.n_vertices());

        for position in &self.positions {
            self.texture_coords.push(VertexTextureCoords(
                projection.project_position(&position.0),
            ));
        }
    }

    /// Computes new tangent space quaternions for the mesh using the texture
    /// coordinates and normal vectors. For each vertex, the averages of the
    /// tangent and bitangent vectors for the triangles that the vertex is a
    /// part of is computed, and the basis formed by the tangent, bitangent and
    /// normal vector is orthogonalized. The quaternion is computed from the
    /// rotation matrix consisting of the three basis vectors as columns.
    ///
    /// If the mesh does not have normal vectors, they will be generated using
    /// [`Self::generate_smooth_normal_vectors`].
    ///
    /// # Panics
    /// If the mesh misses positions or texture coordinates.
    pub fn generate_smooth_tangent_space_quaternions(&mut self) {
        assert!(self.has_positions());
        assert!(self.has_texture_coords());

        if !self.has_normal_vectors() {
            self.generate_smooth_normal_vectors();
        }

        let mut summed_tangent_and_bitangent_vectors = vec![Matrix3x2::zeros(); self.n_vertices()];

        for [idx0, idx1, idx2] in self.triangle_indices() {
            let p0 = &self.positions[idx0].0;
            let p1 = &self.positions[idx1].0;
            let p2 = &self.positions[idx2].0;

            let uv0 = &self.texture_coords[idx0].0;
            let uv1 = &self.texture_coords[idx1].0;
            let uv2 = &self.texture_coords[idx2].0;

            // Solve set of equations for unnormalized tangent and bitangent
            // vectors

            let q1;
            let q2;

            let mut st1 = uv1 - uv0;
            let mut st2 = uv2 - uv0;

            // Switch which two triangle edges to use if the system of equations
            // becomes degenerate with the current edges (required if the third
            // edge is aligned with the u- or v-direction)
            if abs_diff_eq!(st1.x, st2.x) || abs_diff_eq!(st1.y, st2.y) {
                st1 = uv2 - uv1;
                st2 = uv0 - uv1;

                if abs_diff_eq!(st1.x, st2.x) || abs_diff_eq!(st1.y, st2.y) {
                    st1 = uv0 - uv2;
                    st2 = uv1 - uv2;

                    q1 = p0 - p2;
                    q2 = p1 - p2;
                } else {
                    q1 = p2 - p1;
                    q2 = p0 - p1;
                }
            } else {
                q1 = p1 - p0;
                q2 = p2 - p0;
            }

            let inv_denom = F::ONE / (st1.x * st2.y - st2.x * st1.y);

            // Skip the triangle altogether if no solution is possible (happens
            // if the triangle is perpendicular to the UV-plane)
            if inv_denom.is_finite() {
                let tangent_and_bitangent =
                    Matrix3x2::from_columns(&[q1 * st2.y - q2 * st1.y, q2 * st1.x - q1 * st2.x])
                        * inv_denom;

                // The unnormalized tangent and bitangent will have the same
                // normalization factor for each triangle, so there is no need to
                // normalize them before aggregating them as long as we perform
                // normalization after aggregation
                summed_tangent_and_bitangent_vectors[idx0] += tangent_and_bitangent;
                summed_tangent_and_bitangent_vectors[idx1] += tangent_and_bitangent;
                summed_tangent_and_bitangent_vectors[idx2] += tangent_and_bitangent;
            }
        }

        self.tangent_space_quaternions.clear();
        self.tangent_space_quaternions.reserve(self.n_vertices());

        for (summed_tangent_and_bitangent, normal) in summed_tangent_and_bitangent_vectors
            .into_iter()
            .zip(self.normal_vectors.iter())
        {
            let summed_tangent = summed_tangent_and_bitangent.column(0);
            let summed_bitangent = summed_tangent_and_bitangent.column(1);

            // Use Gram-Schmidt to make the summed tangent and bitangent
            // orthogonal to the normal vector and each other, then normalize

            let orthogonal_tangent =
                summed_tangent - normal.0.as_ref() * normal.0.dot(&summed_tangent);

            let inv_orthogonal_tangent_squared_length =
                F::ONE / orthogonal_tangent.magnitude_squared();

            let tangent;
            let mut bitangent;
            let mut is_lefthanded = false;

            if inv_orthogonal_tangent_squared_length.is_finite() {
                let orthogonal_bitangent = summed_bitangent
                    - normal.0.as_ref() * normal.0.dot(&summed_bitangent)
                    - orthogonal_tangent
                        * (orthogonal_tangent.dot(&summed_bitangent)
                            * inv_orthogonal_tangent_squared_length);

                tangent = UnitVector3::new_unchecked(
                    orthogonal_tangent * F::sqrt(inv_orthogonal_tangent_squared_length),
                );

                bitangent = UnitVector3::new_normalize(orthogonal_bitangent);

                // Check if basis is left-handed
                if tangent.cross(&bitangent).dot(&normal.0) < F::ZERO {
                    // Make sure tangent, bitangent and normal form a
                    // right-handed bases, as this is required for converting
                    // the basis to a rotation quaternion. But we note the fact
                    // that the system is really left-handed, so that we can
                    // encode this into the quaternion.
                    bitangent = -bitangent;
                    is_lefthanded = true;
                }
            } else {
                if abs_diff_ne!(normal.0.x.abs(), F::ONE) {
                    tangent =
                        UnitVector3::new_normalize(Vector3::x() - normal.0.as_ref() * normal.0.x);
                } else {
                    tangent =
                        UnitVector3::new_normalize(Vector3::y() - normal.0.as_ref() * normal.0.y);
                }

                bitangent = UnitVector3::new_normalize(normal.0.cross(&tangent));
            }

            // Convert right-handed orthonormal basis vectors to rotation
            // quaternion
            let mut tangent_space_quaternion = UnitQuaternion::from_basis_unchecked(&[
                tangent.into_inner(),
                bitangent.into_inner(),
                normal.0.into_inner(),
            ])
            .into_inner();

            // Make sure real component is always positive initially (negating a
            // quaternion gives same rotation)
            if tangent_space_quaternion.w < F::ZERO {
                tangent_space_quaternion = -tangent_space_quaternion;
            }

            // If we have a left-handed basis, negate the quaternion so that the
            // real component is negative (but we still have the same rotation)
            if is_lefthanded {
                tangent_space_quaternion = -tangent_space_quaternion;
            }

            self.tangent_space_quaternions
                .push(VertexTangentSpaceQuaternion(UnitQuaternion::new_unchecked(
                    tangent_space_quaternion,
                )));
        }

        self.tangent_space_quaternion_change_tracker
            .notify_count_change();
    }

    /// Removes all normal vectors from the mesh.
    pub fn remove_normal_vectors(&mut self) {
        self.normal_vectors.clear();
    }

    /// Flips the winding order of all triangles in the mesh.
    pub fn flip_triangle_winding_order(&mut self) {
        for triangle in self.indices.chunks_exact_mut(3) {
            triangle.swap(1, 2);
        }
    }

    /// Applies the given scaling factor to the vertex positions of the mesh.
    pub fn scale(&mut self, scaling: F) {
        for position in &mut self.positions {
            *position = position.scaled(scaling);
        }
    }

    /// Applies the given similarity transform to the mesh, transforming vertex
    /// positions, normal vectors and tangent space quaternions.
    pub fn transform(&mut self, transform: &Similarity3<F>) {
        for position in &mut self.positions {
            *position = position.transformed(transform);
        }

        for normal_vector in &mut self.normal_vectors {
            *normal_vector = normal_vector.transformed(transform);
        }

        for tangent_space_quaternion in &mut self.tangent_space_quaternions {
            *tangent_space_quaternion = tangent_space_quaternion.transformed(transform);
        }
    }

    /// Assigns the given colors to the mesh vertices.
    ///
    /// # Panics
    /// If the number of colors differs from the number of vertices.
    pub fn set_colors(&mut self, colors: Vec<VertexColor<F>>) {
        self.colors = colors;
        self.color_change_tracker.notify_count_change();
    }

    /// Sets the color of every vertex to the given color.
    pub fn set_same_color(&mut self, color: VertexColor<F>) {
        self.set_colors(vec![color; self.positions.len()]);
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

        if self.has_tangent_space_quaternions() {
            assert!(other.has_tangent_space_quaternions());
            self.tangent_space_quaternions
                .extend_from_slice(&other.tangent_space_quaternions);
            self.tangent_space_quaternion_change_tracker
                .notify_count_change();
        }

        if self.has_colors() {
            assert!(other.has_colors());
            self.colors.extend_from_slice(&other.colors);
            self.color_change_tracker.notify_count_change();
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

    /// Forgets any recorded changes to the vertex normal vectors.
    pub fn reset_normal_vector_change_tracking(&self) {
        self.normal_vector_change_tracker.reset();
    }

    /// Forgets any recorded changes to the vertex texture coordinates.
    pub fn reset_texture_coord_change_tracking(&self) {
        self.texture_coord_change_tracker.reset();
    }

    /// Forgets any recorded changes to the vertex tangent space quaternions.
    pub fn reset_tangent_space_quaternion_change_tracking(&self) {
        self.tangent_space_quaternion_change_tracker.reset();
    }

    /// Forgets any recorded changes to the vertex colors.
    pub fn reset_color_change_tracking(&self) {
        self.color_change_tracker.reset();
    }

    /// Forgets any recorded changes to the indices.
    pub fn reset_index_change_tracking(&self) {
        self.index_change_tracker.reset();
    }

    /// Forgets any recorded changes to the vertex attributes and indices.
    pub fn reset_change_tracking(&self) {
        self.reset_position_change_tracking();
        self.reset_normal_vector_change_tracking();
        self.reset_texture_coord_change_tracking();
        self.reset_tangent_space_quaternion_change_tracking();
        self.reset_color_change_tracking();
        self.reset_index_change_tracking();
    }
}
