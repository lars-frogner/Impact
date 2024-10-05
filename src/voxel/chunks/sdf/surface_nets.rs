//! Mesh generation for voxel chunk signed distance fields using surface nets.
//! Adapted from <https://github.com/bonsairobo/fast-surface-nets-rs>.

use crate::voxel::{
    chunks::sdf::{VoxelChunkSignedDistanceField, SDF_GRID_CELL_COUNT},
    mesh::{VoxelMeshIndexMaterials, VoxelMeshVertexNormalVector, VoxelMeshVertexPosition},
    utils::{Dimension, Side},
};
use glam::{Vec3A, Vec3Swizzles};
use std::array;

/// The output buffers used by
/// [`VoxelChunkSignedDistanceField::compute_surface_nets_mesh`]. These buffers
/// can be reused to avoid reallocating memory.
#[derive(Debug)]
pub struct SurfaceNetsBuffer {
    /// The triangle mesh vertex positions.
    pub positions: Vec<VoxelMeshVertexPosition>,
    /// The triangle mesh vertex normal vectors.
    ///
    /// The normals are **not** normalized, since that is done most efficiently
    /// on the GPU.
    pub normal_vectors: Vec<VoxelMeshVertexNormalVector>,
    /// The material indices and weights for each vertex in the mesh.
    pub vertex_materials: Vec<SurfaceNetsVertexMaterials>,
    /// The material indices and weights for each vertex index in the mesh.
    pub index_materials: Vec<VoxelMeshIndexMaterials>,
    /// The vertex index triples defining the triangles in the mesh.
    pub indices: Vec<u16>,

    /// Local 3D array coordinates of every voxel that intersects the
    /// isosurface, together with the corresponding linear indices in the SDF
    /// array.
    pub surface_points_and_linear_indices: Vec<([u8; 3], u16)>,
    /// Used to map back from voxel linear index to vertex index.
    pub voxel_linear_idx_to_vertex_index: [u16; SDF_GRID_CELL_COUNT],
}

/// The materials of the voxels defining a vertex in a surface nets mesh. Each
/// material is represented by an index and a weight corresponding to the
/// number of voxels among the eight voxels defining the vertex that have that
/// material. There can be at most seven different materials (since at least one
/// of the eight voxels must be empty for there to be a vertex), and at least
/// one (since at least one voxel must be non-empty). The materials are sorted
/// in descending order of weight.
#[derive(Clone, Copy, Debug)]
pub struct SurfaceNetsVertexMaterials {
    pub indices: [u8; 8],
    pub weights: [u8; 8],
}

impl SurfaceNetsBuffer {
    /// Whether the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }

    /// Creates a new empty buffer with the given capacities for vertices and
    /// indices.
    pub fn with_capacities(vertex_capacity: usize, index_capacity: usize) -> Self {
        Self {
            positions: Vec::with_capacity(vertex_capacity),
            normal_vectors: Vec::with_capacity(vertex_capacity),
            vertex_materials: Vec::with_capacity(vertex_capacity),
            index_materials: Vec::with_capacity(index_capacity),
            indices: Vec::with_capacity(index_capacity),
            surface_points_and_linear_indices: Vec::with_capacity(vertex_capacity),
            voxel_linear_idx_to_vertex_index: [NULL_VERTEX; SDF_GRID_CELL_COUNT],
        }
    }

    /// Clears all of the buffers, but keeps the memory allocated for reuse.
    fn reset(&mut self) {
        self.positions.clear();
        self.normal_vectors.clear();
        self.vertex_materials.clear();
        self.index_materials.clear();
        self.indices.clear();
        self.surface_points_and_linear_indices.clear();
        self.voxel_linear_idx_to_vertex_index.fill(NULL_VERTEX);
    }
}

/// This linear index in the SDF array did not produce a vertex.
pub const NULL_VERTEX: u16 = u16::MAX;

impl VoxelChunkSignedDistanceField {
    /// Computes a mesh for the surface defined by the signed distance field
    /// using the Naive Surface Nets smooth voxel meshing algorithm.
    ///
    /// Each value in the field determines how close that point is to the
    /// isosurface. Negative values are considered "interior" of the surface
    /// volume, and positive values are considered "exterior." These lattice
    /// points will be considered corners of unit cubes. For each unit cube,
    /// at most one isosurface vertex will be estimated, as below, where `p`
    /// is a positive corner value, `n` is a negative corner value, `s` is
    /// an isosurface vertex, and `|` or `-` are mesh polygons connecting
    /// the vertices.
    ///
    /// ```text
    /// p   p   p   p
    ///   s---s
    /// p | n | p   p
    ///   s   s---s
    /// p | n   n | p
    ///   s---s---s
    /// p   p   p   p
    /// ```
    ///
    /// Since the chunk's SDF is padded with a 1-voxel border copied from
    /// neighboring voxels, meshes for adjacent chunks will connect seamlessly.
    ///
    /// The scale and offset of the mesh relative to local SDF array coordinates
    /// is determined by the given voxel extent and position offset. Remember to
    /// account for the padding (subtract one voxel extent from the position of
    /// the chunk's lower corner) when computing the offset for composing
    /// multiple chunks.
    pub fn compute_surface_nets_mesh(
        &self,
        voxel_extent: f32,
        position_offset: &Vec3A,
        buffer: &mut SurfaceNetsBuffer,
    ) {
        buffer.reset();

        self.estimate_surface_nets_surface(voxel_extent, position_offset, buffer);

        self.make_all_surface_nets_quads(buffer);

        calculate_all_index_materials(
            &buffer.indices,
            &buffer.vertex_materials,
            &mut buffer.index_materials,
        );
    }

    // Find all vertex positions and normals. Also generate a map from grid position
    // to vertex index to be used to look up vertices when generating quads.
    fn estimate_surface_nets_surface(
        &self,
        voxel_extent: f32,
        position_offset: &Vec3A,
        buffer: &mut SurfaceNetsBuffer,
    ) {
        for i in 0..Self::grid_size_u32() - 1 {
            for j in 0..Self::grid_size_u32() - 1 {
                for k in 0..Self::grid_size_u32() - 1 {
                    let linear_idx = Self::linear_idx_u32(&[i, j, k]);
                    if let Some((centroid, normal, vertex_materials)) =
                        self.estimate_surface_net_vertex_attributes_in_cube(linear_idx)
                    {
                        let position = voxel_extent
                            * (Vec3A::from([i as f32, j as f32, k as f32]) + centroid)
                            + position_offset;

                        buffer.voxel_linear_idx_to_vertex_index[linear_idx as usize] =
                            buffer.positions.len() as u16; // Mind dependency on `positions`
                        buffer
                            .surface_points_and_linear_indices
                            .push(([i as u8, j as u8, k as u8], linear_idx as u16));

                        buffer
                            .positions
                            .push(VoxelMeshVertexPosition(position.into()));
                        buffer
                            .normal_vectors
                            .push(VoxelMeshVertexNormalVector(normal.into()));
                        buffer.vertex_materials.push(vertex_materials);
                    }
                }
            }
        }
    }

    // Considers the grid-aligned cube whose minimal corner has the given linear
    // index in the SDF array and find a point inside this cube (relative to the
    // minimal corner) that is approximately on the isosurface.
    //
    // This is done by estimating, for each cube edge, where the isosurface crosses
    // the edge (if it does at all). Then the estimated surface point is the average
    // of these edge crossings.
    //
    // Also computes the normal vector of the surface at the surface point.
    //
    // Returns [`None`] if there is no surface point within the cube.
    fn estimate_surface_net_vertex_attributes_in_cube(
        &self,
        min_corner_linear_idx: u32,
    ) -> Option<(Vec3A, Vec3A, SurfaceNetsVertexMaterials)> {
        let mut corner_dists = [0.0; 8];
        let mut corner_has_voxel = [false; 8];
        let mut corner_material_indices = [0; 8];

        let mut num_negative = 0;

        // Get the signed distance and material index at each corner of this cube
        for idx in 0..8 {
            let corner_linear_idx =
                min_corner_linear_idx + Self::linear_idx_u32(&CUBE_CORNERS[idx]);

            corner_dists[idx] = self.values[corner_linear_idx as usize];
            corner_material_indices[idx] = self.voxel_types[corner_linear_idx as usize].idx_u8();

            if corner_dists[idx].is_sign_negative() {
                corner_has_voxel[idx] = true;
                num_negative += 1;
            }
        }

        if num_negative == 0 || num_negative == 8 {
            // No crossings.
            return None;
        }

        let centroid = centroid_of_edge_intersections(&corner_dists);
        let normal = sdf_gradient(&corner_dists, centroid);
        let vertex_materials =
            SurfaceNetsVertexMaterials::compute(corner_has_voxel, corner_material_indices);

        Some((centroid, normal, vertex_materials))
    }

    // For every edge that crosses the isosurface, makes a quad between the
    // "centers" of the four cubes touching that surface. The "centers" are
    // actually the vertex positions found earlier. Also makes sure the triangles
    // are facing the right way. See the comments on `maybe_make_quad` to help
    // with understanding the indexing.
    fn make_all_surface_nets_quads(&self, buffer: &mut SurfaceNetsBuffer) {
        let mut upper_indices = [Self::grid_size_u32() as u8 - 1; 3];

        // To avoid z-fighting due to triangles duplicated between adjacent chunks,
        // we avoid generating triangles from the upper voxels for every dimension where
        // we have an adjacent chunk that will be rendered
        for dim in Dimension::all() {
            if self.adjacent_is_non_uniform(dim, Side::Upper) {
                upper_indices[dim.idx()] -= 1;
            }
        }

        for &([i, j, k], p_linear_idx) in &buffer.surface_points_and_linear_indices {
            let p_linear_idx = p_linear_idx as usize;

            // Do edges parallel with the X axis
            if j != 0 && k != 0 && i < upper_indices[0] {
                self.maybe_make_surface_nets_quad(
                    &buffer.voxel_linear_idx_to_vertex_index,
                    &buffer.positions,
                    p_linear_idx,
                    p_linear_idx + Self::squared_grid_size(),
                    Self::grid_size(),
                    1,
                    &mut buffer.indices,
                );
            }
            // Do edges parallel with the Y axis
            if i != 0 && k != 0 && j < upper_indices[1] {
                self.maybe_make_surface_nets_quad(
                    &buffer.voxel_linear_idx_to_vertex_index,
                    &buffer.positions,
                    p_linear_idx,
                    p_linear_idx + Self::grid_size(),
                    1,
                    Self::squared_grid_size(),
                    &mut buffer.indices,
                );
            }
            // Do edges parallel with the Z axis
            if i != 0 && j != 0 && k < upper_indices[2] {
                self.maybe_make_surface_nets_quad(
                    &buffer.voxel_linear_idx_to_vertex_index,
                    &buffer.positions,
                    p_linear_idx,
                    p_linear_idx + 1,
                    Self::squared_grid_size(),
                    Self::grid_size(),
                    &mut buffer.indices,
                );
            }
        }
    }

    // Constructs a quad in the dual graph of the SDF lattice.
    //
    // The surface point s was found somewhere inside of the cube with minimal
    // corner p1.
    //
    //       x ---- x
    //      /      /|
    //     x ---- x |
    //     |   s  | x
    //     |      |/
    //    p1 --- p2
    //
    // And now we want to find the quad between p1 and p2 where s is a corner of the
    // quad.
    //
    //          s
    //         /|
    //        / |
    //       |  |
    //   p1  |  |  p2
    //       | /
    //       |/
    //
    // If A is (of the three grid axes) the axis between p1 and p2,
    //
    //       A
    //   p1 ---> p2
    //
    // then we must find the other 3 quad corners by moving along the other two axes
    // (those orthogonal to A) in the negative directions; these are axis B and axis
    // C.
    fn maybe_make_surface_nets_quad(
        &self,
        linear_idx_to_vertex_index: &[u16],
        positions: &[VoxelMeshVertexPosition],
        p1: usize,
        p2: usize,
        axis_b_linear_idx: usize,
        axis_c_linear_idx: usize,
        indices: &mut Vec<u16>,
    ) {
        let d1 = self.values[p1];
        let d2 = self.values[p2];
        let negative_face = match (d1.is_sign_negative(), d2.is_sign_negative()) {
            (true, false) => false,
            (false, true) => true,
            _ => return, // No face
        };

        // The triangle points, viewed face-front, look like this:
        // v1 v3
        // v2 v4
        let v1 = linear_idx_to_vertex_index[p1];
        let v2 = linear_idx_to_vertex_index[p1 - axis_b_linear_idx];
        let v3 = linear_idx_to_vertex_index[p1 - axis_c_linear_idx];
        let v4 = linear_idx_to_vertex_index[p1 - axis_b_linear_idx - axis_c_linear_idx];
        let (pos1, pos2, pos3, pos4) = (
            Vec3A::from(positions[v1 as usize].0),
            Vec3A::from(positions[v2 as usize].0),
            Vec3A::from(positions[v3 as usize].0),
            Vec3A::from(positions[v4 as usize].0),
        );
        // Split the quad along the shorter axis, rather than the longer one.
        let quad = if pos1.distance_squared(pos4) < pos2.distance_squared(pos3) {
            if negative_face {
                [v1, v4, v2, v1, v3, v4]
            } else {
                [v1, v2, v4, v1, v4, v3]
            }
        } else if negative_face {
            [v2, v3, v4, v2, v1, v3]
        } else {
            [v2, v4, v3, v2, v3, v1]
        };
        indices.extend_from_slice(&quad);
    }
}

fn centroid_of_edge_intersections(dists: &[f32; 8]) -> Vec3A {
    let mut count = 0;
    let mut sum = Vec3A::ZERO;
    for &[corner1, corner2] in &CUBE_EDGES {
        let d1 = dists[corner1 as usize];
        let d2 = dists[corner2 as usize];
        if opposite_signs(d1, d2) {
            count += 1;
            sum += estimate_surface_edge_intersection(corner1, corner2, d1, d2);
        }
    }

    sum / count as f32
}

fn opposite_signs(a: f32, b: f32) -> bool {
    (a.to_bits() ^ b.to_bits()) & 0x8000_0000 != 0
}

// Given two cube corners, finds the point between them where the SDF is zero.
// (This might not exist).
fn estimate_surface_edge_intersection(
    corner1: u32,
    corner2: u32,
    value1: f32,
    value2: f32,
) -> Vec3A {
    let interp1 = value1 / (value1 - value2);
    let interp2 = 1.0 - interp1;

    interp2 * CUBE_CORNER_VECTORS[corner1 as usize]
        + interp1 * CUBE_CORNER_VECTORS[corner2 as usize]
}

/// Calculates an unnormalized surface normal vector as the gradient of the
/// distance field.
///
/// For each dimension, there are 4 cube edges along that axis. This will do
/// bilinear interpolation between the differences along those edges based on
/// the position of the surface (s).
fn sdf_gradient(dists: &[f32; 8], s: Vec3A) -> Vec3A {
    let p00 = Vec3A::from([dists[0b100], dists[0b010], dists[0b001]]);
    let n00 = Vec3A::from([dists[0b000], dists[0b000], dists[0b000]]);

    let p01 = Vec3A::from([dists[0b101], dists[0b110], dists[0b011]]);
    let n01 = Vec3A::from([dists[0b001], dists[0b100], dists[0b010]]);

    let p10 = Vec3A::from([dists[0b110], dists[0b011], dists[0b101]]);
    let n10 = Vec3A::from([dists[0b010], dists[0b001], dists[0b100]]);

    let p11 = Vec3A::from([dists[0b111], dists[0b111], dists[0b111]]);
    let n11 = Vec3A::from([dists[0b011], dists[0b101], dists[0b110]]);

    // Each dimension encodes an edge delta, giving 12 in total.
    let d00 = p00 - n00; // Edges (0bx00, 0b0y0, 0b00z)
    let d01 = p01 - n01; // Edges (0bx01, 0b1y0, 0b01z)
    let d10 = p10 - n10; // Edges (0bx10, 0b0y1, 0b10z)
    let d11 = p11 - n11; // Edges (0bx11, 0b1y1, 0b11z)

    let neg = Vec3A::ONE - s;

    // Do bilinear interpolation between 4 edges in each dimension.
    neg.yzx() * neg.zxy() * d00
        + neg.yzx() * s.zxy() * d01
        + s.yzx() * neg.zxy() * d10
        + s.yzx() * s.zxy() * d11
}

macro_rules! sorting_network {
    ($cmpswap:expr, $(($i:expr, $j:expr)),* $(,)?) => {{
        $(
            $cmpswap($i, $j);
        )*
    }};
}

#[rustfmt::skip]
macro_rules! sorting_network_7 {
    ($cmpswap:expr) => {
        sorting_network!($cmpswap,
            // Step 1
            (0, 6), (1, 5), (2, 4),
            // Step 2
            (0, 3), (1, 2), (4, 5),
            // Step 3
            (0, 1), (2, 3), (4, 6), (5, 6),
            // Step 4
            (1, 4), (3, 5),
            // Step 5
            (1, 2), (3, 4), (5, 6),
            // Step 6
            (2, 3), (4, 5),
        );
    };
}

// TODO: When the vertex has a single material and is surrounded by vertices
// without that material, the resulting blending looks unnatural. This is
// expected, but it would be nice to make it look better. It is mainly an issue
// when the single material has a strong contrast with the surrounding
// materials.
impl SurfaceNetsVertexMaterials {
    fn compute(corner_has_voxel: [bool; 8], corner_material_indices: [u8; 8]) -> Self {
        let mut materials = Self {
            indices: [0; 8],
            weights: [0; 8],
        };

        // All voxels can't be empty
        debug_assert!(!corner_has_voxel.iter().all(|&has_voxel| !has_voxel));
        // All voxels can't be non-empty
        debug_assert!(!corner_has_voxel.iter().all(|&has_voxel| has_voxel));

        const INVALID_IDX: u8 = 255;
        let mut material_idx_map = [INVALID_IDX; 256];

        let mut material_count = 0;

        for (&has_voxel, &corner_material_index) in
            corner_has_voxel.iter().zip(&corner_material_indices)
        {
            if has_voxel {
                let idx = material_idx_map[corner_material_index as usize];
                if idx == INVALID_IDX {
                    materials.indices[material_count] = corner_material_index;
                    materials.weights[material_count] = 1;
                    material_idx_map[corner_material_index as usize] = material_count as u8;
                    material_count += 1;
                } else {
                    materials.weights[idx as usize] += 1;
                }
            }
        }

        materials.indices[7] = material_count as u8;

        materials.sort_descending();

        materials
    }

    #[cfg(test)]
    fn with_valid_indices_and_weights(indices: &[u8], weights: &[u8]) -> Self {
        assert_eq!(indices.len(), weights.len());
        assert!(!indices.is_empty());
        assert!(indices.len() < 8);
        assert!(weights.iter().sum::<u8>() > 0);
        assert!(weights.is_sorted_by(|a, b| a >= b));

        let material_count = indices.len();
        let mut indices = array::from_fn(|idx| indices.get(idx).copied().unwrap_or(0));
        indices[7] = material_count as u8;

        let weights = array::from_fn(|idx| weights.get(idx).copied().unwrap_or(0));

        Self { indices, weights }
    }

    #[inline]
    fn sort_descending(&mut self) {
        let mut compare_and_swap = |i: usize, j: usize| {
            if self.weights[i] < self.weights[j] {
                self.indices.swap(i, j);
                self.weights.swap(i, j);
            }
        };
        // We know that at least one voxel must be empty, so we ignore the last
        // value
        sorting_network_7!(compare_and_swap);
    }

    pub fn material_count(&self) -> usize {
        self.indices[7] as usize
    }

    pub fn valid_indices(&self) -> &[u8] {
        &self.indices[..self.material_count()]
    }

    pub fn valid_weights(&self) -> &[u8] {
        &self.weights[..self.material_count()]
    }
}

fn calculate_all_index_materials(
    indices: &[u16],
    vertex_materials: &[SurfaceNetsVertexMaterials],
    index_materials: &mut Vec<VoxelMeshIndexMaterials>,
) {
    index_materials.reserve(indices.len());
    for indices in indices.chunks_exact(3) {
        calculate_index_materials_for_triangle(
            [
                &vertex_materials[indices[0] as usize],
                &vertex_materials[indices[1] as usize],
                &vertex_materials[indices[2] as usize],
            ],
            index_materials,
        );
    }
}

#[inline]
fn calculate_index_materials_for_triangle(
    vertex_materials: [&SurfaceNetsVertexMaterials; 3],
    index_materials: &mut Vec<VoxelMeshIndexMaterials>,
) {
    // Fast path when all three vertices have the same material
    if vertex_materials[0].material_count() == 1
        && vertex_materials[1].material_count() == 1
        && vertex_materials[2].material_count() == 1
    {
        let index = vertex_materials[0].indices[0];
        if vertex_materials[1].indices[0] == index && vertex_materials[2].indices[0] == index {
            let index_material = VoxelMeshIndexMaterials {
                indices: [index, 0, 0, 0],
                weights: [1, 0, 0, 0],
            };
            index_materials.push(index_material);
            index_materials.push(index_material);
            index_materials.push(index_material);
            return;
        }
    }

    let mut top_indices = [0; 4];
    let mut n_top_indices = 0;

    let mut is_top_index = [false; 256];

    let mut offsets = [0; 3];

    for top_index in &mut top_indices {
        let weights: [u8; 3] = array::from_fn(|i| vertex_materials[i].weights[offsets[i]]);

        let max_weight_idx = if weights[0] >= weights[1] {
            if weights[0] >= weights[2] {
                0
            } else {
                2
            }
        } else if weights[1] >= weights[2] {
            1
        } else {
            2
        };

        if weights[max_weight_idx] == 0 {
            break;
        }

        *top_index = vertex_materials[max_weight_idx].indices[offsets[max_weight_idx]];
        n_top_indices += 1;

        is_top_index[*top_index as usize] = true;

        // Advance the offsets until we find the next indices that are not among the
        // existing top indices. We stop if we reach the end of the valid material
        // indices. When we are at the end, the weight will always be zero.
        for i in 0..3 {
            while offsets[i] < vertex_materials[i].material_count()
                && is_top_index[vertex_materials[i].indices[offsets[i]] as usize]
            {
                offsets[i] += 1;
            }
        }
    }

    for materials in vertex_materials {
        let mut weights = [0; 4];

        for i in 0..n_top_indices {
            for j in 0..materials.material_count() {
                if materials.indices[j] == top_indices[i] {
                    weights[i] = materials.weights[j];
                    break;
                }
            }
        }

        index_materials.push(VoxelMeshIndexMaterials {
            indices: top_indices,
            weights,
        });
    }
}

const CUBE_CORNERS: [[u32; 3]; 8] = [
    [0, 0, 0],
    [0, 0, 1],
    [0, 1, 0],
    [0, 1, 1],
    [1, 0, 0],
    [1, 0, 1],
    [1, 1, 0],
    [1, 1, 1],
];

const CUBE_CORNER_VECTORS: [Vec3A; 8] = [
    Vec3A::new(0.0, 0.0, 0.0),
    Vec3A::new(0.0, 0.0, 1.0),
    Vec3A::new(0.0, 1.0, 0.0),
    Vec3A::new(0.0, 1.0, 1.0),
    Vec3A::new(1.0, 0.0, 0.0),
    Vec3A::new(1.0, 0.0, 1.0),
    Vec3A::new(1.0, 1.0, 0.0),
    Vec3A::new(1.0, 1.0, 1.0),
];

const CUBE_EDGES: [[u32; 2]; 12] = [
    [0b000, 0b001],
    [0b000, 0b010],
    [0b000, 0b100],
    [0b001, 0b011],
    [0b001, 0b101],
    [0b010, 0b011],
    [0b010, 0b110],
    [0b011, 0b111],
    [0b100, 0b101],
    [0b100, 0b110],
    [0b101, 0b111],
    [0b110, 0b111],
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vertex_materials_have_single_nonzero_weight_when_all_voxels_have_same_type() {
        let corner_has_voxel = [true, true, true, false, true, true, true, true];
        let materials = SurfaceNetsVertexMaterials::compute(corner_has_voxel, [0; 8]);
        assert_eq!(materials.material_count(), 1);
        assert_eq!(materials.valid_indices(), &[0]);
        assert_eq!(materials.valid_weights(), &[7]);

        let materials = SurfaceNetsVertexMaterials::compute(corner_has_voxel, [1; 8]);
        assert_eq!(materials.material_count(), 1);
        assert_eq!(materials.valid_indices(), &[1]);
        assert_eq!(materials.valid_weights(), &[7]);

        let materials = SurfaceNetsVertexMaterials::compute(corner_has_voxel, [254; 8]);
        assert_eq!(materials.material_count(), 1);
        assert_eq!(materials.valid_indices(), &[254]);
        assert_eq!(materials.valid_weights(), &[7]);
    }

    #[test]
    fn vertex_materials_have_two_nonzero_weights_for_two_different_voxel_types() {
        let corner_has_voxel = [true, true, true, false, true, true, true, true];
        let materials =
            SurfaceNetsVertexMaterials::compute(corner_has_voxel, [0, 0, 0, 0, 0, 0, 0, 1]);
        assert_eq!(materials.material_count(), 2);
        assert_eq!(materials.valid_indices(), &[0, 1]);
        assert_eq!(materials.valid_weights(), &[6, 1]);

        let materials =
            SurfaceNetsVertexMaterials::compute(corner_has_voxel, [0, 1, 0, 0, 1, 0, 0, 1]);
        assert_eq!(materials.material_count(), 2);
        assert_eq!(materials.valid_indices(), &[0, 1]);
        assert_eq!(materials.valid_weights(), &[4, 3]);

        let materials =
            SurfaceNetsVertexMaterials::compute(corner_has_voxel, [1, 1, 1, 0, 1, 1, 1, 0]);
        assert_eq!(materials.material_count(), 2);
        assert_eq!(materials.valid_indices(), &[1, 0]);
        assert_eq!(materials.valid_weights(), &[6, 1]);
    }

    #[test]
    fn vertex_materials_have_seven_nonzero_weights_for_seven_different_voxel_types() {
        let corner_has_voxel = [true, true, true, false, true, true, true, true];
        let materials =
            SurfaceNetsVertexMaterials::compute(corner_has_voxel, [0, 1, 2, 0, 4, 5, 6, 7]);
        assert_eq!(materials.material_count(), 7);
        assert_eq!(materials.valid_indices(), &[0, 1, 2, 4, 5, 6, 7]);
        assert_eq!(materials.valid_weights(), &[1, 1, 1, 1, 1, 1, 1]);

        let materials =
            SurfaceNetsVertexMaterials::compute(corner_has_voxel, [7, 6, 5, 0, 3, 2, 1, 0]);
        assert_eq!(materials.material_count(), 7);
        assert_eq!(materials.valid_indices(), &[7, 6, 5, 3, 2, 1, 0]);
        assert_eq!(materials.valid_weights(), &[1, 1, 1, 1, 1, 1, 1]);
    }

    #[test]
    fn vertex_materials_have_correct_weights_for_different_voxel_types_where_multiple_are_empty() {
        let materials = SurfaceNetsVertexMaterials::compute(
            [true, true, false, false, true, false, true, true],
            [4, 2, 0, 7, 0, 3, 0, 7],
        );
        assert_eq!(materials.material_count(), 4);
        assert_eq!(materials.valid_indices(), &[0, 4, 2, 7]);
        assert_eq!(materials.valid_weights(), &[2, 1, 1, 1]);
    }

    #[test]
    fn vertex_materials_are_sorted_correctly() {
        let materials = SurfaceNetsVertexMaterials::compute(
            [true, true, false, true, true, true, true, true],
            [3, 2, 0, 1, 1, 1, 1, 2],
        );
        assert_eq!(materials.material_count(), 3);
        assert_eq!(materials.valid_indices(), &[1, 2, 3]);
        assert_eq!(materials.valid_weights(), &[4, 2, 1]);
    }

    #[test]
    fn triangle_index_materials_are_correct_for_same_vertex_material() {
        let mut index_materials = Vec::new();
        calculate_index_materials_for_triangle(
            [
                &SurfaceNetsVertexMaterials::with_valid_indices_and_weights(&[0], &[7]),
                &SurfaceNetsVertexMaterials::with_valid_indices_and_weights(&[0], &[4]),
                &SurfaceNetsVertexMaterials::with_valid_indices_and_weights(&[0], &[1]),
            ],
            &mut index_materials,
        );
        assert_eq!(
            index_materials,
            vec![
                VoxelMeshIndexMaterials {
                    indices: [0, 0, 0, 0],
                    weights: [1, 0, 0, 0],
                },
                VoxelMeshIndexMaterials {
                    indices: [0, 0, 0, 0],
                    weights: [1, 0, 0, 0],
                },
                VoxelMeshIndexMaterials {
                    indices: [0, 0, 0, 0],
                    weights: [1, 0, 0, 0],
                },
            ]
        );
    }

    #[test]
    fn triangle_index_materials_are_correct_for_simple_vertex_material_combo() {
        let mut index_materials = Vec::new();
        calculate_index_materials_for_triangle(
            [
                &SurfaceNetsVertexMaterials::with_valid_indices_and_weights(&[1], &[1]),
                &SurfaceNetsVertexMaterials::with_valid_indices_and_weights(&[2], &[1]),
                &SurfaceNetsVertexMaterials::with_valid_indices_and_weights(&[3], &[1]),
            ],
            &mut index_materials,
        );
        assert_eq!(
            index_materials,
            vec![
                VoxelMeshIndexMaterials {
                    indices: [1, 2, 3, 0],
                    weights: [1, 0, 0, 0],
                },
                VoxelMeshIndexMaterials {
                    indices: [1, 2, 3, 0],
                    weights: [0, 1, 0, 0],
                },
                VoxelMeshIndexMaterials {
                    indices: [1, 2, 3, 0],
                    weights: [0, 0, 1, 0],
                },
            ]
        );
    }

    #[test]
    fn triangle_index_materials_are_correct_for_complex_vertex_material_combo_1() {
        let mut index_materials = Vec::new();
        calculate_index_materials_for_triangle(
            [
                &SurfaceNetsVertexMaterials::with_valid_indices_and_weights(&[0, 1], &[4, 3]),
                &SurfaceNetsVertexMaterials::with_valid_indices_and_weights(&[4, 1, 0], &[5, 1, 1]),
                &SurfaceNetsVertexMaterials::with_valid_indices_and_weights(&[2, 0], &[2, 1]),
            ],
            &mut index_materials,
        );
        assert_eq!(
            index_materials,
            vec![
                VoxelMeshIndexMaterials {
                    indices: [4, 0, 1, 2],
                    weights: [0, 4, 3, 0],
                },
                VoxelMeshIndexMaterials {
                    indices: [4, 0, 1, 2],
                    weights: [5, 1, 1, 0],
                },
                VoxelMeshIndexMaterials {
                    indices: [4, 0, 1, 2],
                    weights: [0, 1, 0, 2],
                },
            ]
        );
    }

    #[test]
    fn triangle_index_materials_are_correct_for_complex_vertex_material_combo_2() {
        let mut index_materials = Vec::new();
        calculate_index_materials_for_triangle(
            [
                &SurfaceNetsVertexMaterials::with_valid_indices_and_weights(&[4, 0], &[3, 2]),
                &SurfaceNetsVertexMaterials::with_valid_indices_and_weights(&[4, 1, 0], &[5, 1, 1]),
                &SurfaceNetsVertexMaterials::with_valid_indices_and_weights(&[0, 4], &[1, 1]),
            ],
            &mut index_materials,
        );
        assert_eq!(
            index_materials,
            vec![
                VoxelMeshIndexMaterials {
                    indices: [4, 0, 1, 0],
                    weights: [3, 2, 0, 0],
                },
                VoxelMeshIndexMaterials {
                    indices: [4, 0, 1, 0],
                    weights: [5, 1, 1, 0],
                },
                VoxelMeshIndexMaterials {
                    indices: [4, 0, 1, 0],
                    weights: [1, 1, 0, 0],
                },
            ]
        );
    }
}
