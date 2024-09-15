//! Mesh generation for voxel chunk signed distance fields using surface nets.
//! Adapted from <https://github.com/bonsairobo/fast-surface-nets-rs>.

use super::VoxelChunkSignedDistanceField;
use crate::voxel::{
    mesh::{VoxelMeshVertexNormalVector, VoxelMeshVertexPosition},
    utils::{Dimension, Side},
};
use glam::{Vec3A, Vec3Swizzles};

/// The output buffers used by
/// [`VoxelChunkSignedDistanceField::compute_surface_nets_mesh`]. These buffers
/// can be reused to avoid reallocating memory.
#[derive(Debug, Default)]
pub struct SurfaceNetsBuffer {
    /// The triangle mesh vertex positions.
    pub positions: Vec<VoxelMeshVertexPosition>,
    /// The triangle mesh vertex normal vectors.
    ///
    /// The normals are **not** normalized, since that is done most efficiently
    /// on the GPU.
    pub normal_vectors: Vec<VoxelMeshVertexNormalVector>,
    /// The triangle mesh indices.
    pub indices: Vec<u16>,

    /// Local 3D array coordinates of every voxel that intersects the
    /// isosurface, together with the corresponding linear indices in the SDF
    /// array.
    pub surface_points_and_linear_indices: Vec<([u8; 3], u16)>,
    /// Used to map back from voxel linear index to vertex index.
    pub voxel_linear_idx_to_vertex_index: Vec<u16>,
}

impl SurfaceNetsBuffer {
    /// Clears all of the buffers, but keeps the memory allocated for reuse.
    fn reset(&mut self, array_size: usize) {
        self.positions.clear();
        self.normal_vectors.clear();
        self.indices.clear();
        self.surface_points_and_linear_indices.clear();

        // Just make sure this buffer is big enough, whether or not we've used it
        // before.
        self.voxel_linear_idx_to_vertex_index
            .resize(array_size, NULL_VERTEX);
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
        buffer.reset(Self::grid_cell_count());

        self.estimate_surface_nets_surface(voxel_extent, position_offset, buffer);
        self.make_all_surface_nets_quads(buffer);
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
                    if let Some((centroid, normal)) =
                        self.estimate_surface_net_centroid_and_normal_in_cube(linear_idx)
                    {
                        let position = voxel_extent
                            * (Vec3A::from([i as f32, j as f32, k as f32]) + centroid)
                            + position_offset;

                        buffer
                            .positions
                            .push(VoxelMeshVertexPosition(position.into()));
                        buffer
                            .normal_vectors
                            .push(VoxelMeshVertexNormalVector(normal.into()));

                        // Note: performing these pushes before pushing vertices seems
                        // to produce a significant slowdown
                        buffer.voxel_linear_idx_to_vertex_index[linear_idx as usize] =
                            buffer.positions.len() as u16 - 1; // Mind dependency on `vertices`
                        buffer
                            .surface_points_and_linear_indices
                            .push(([i as u8, j as u8, k as u8], linear_idx as u16));
                    } else {
                        buffer.voxel_linear_idx_to_vertex_index[linear_idx as usize] = NULL_VERTEX;
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
    fn estimate_surface_net_centroid_and_normal_in_cube(
        &self,
        min_corner_linear_idx: u32,
    ) -> Option<(Vec3A, Vec3A)> {
        // Get the signed distance values at each corner of this cube.
        let mut corner_dists = [0f32; 8];
        let mut num_negative = 0;
        for (i, dist) in corner_dists.iter_mut().enumerate() {
            let corner_linear_idx = min_corner_linear_idx + Self::linear_idx_u32(&CUBE_CORNERS[i]);
            *dist = self.values[corner_linear_idx as usize];
            if dist.is_sign_negative() {
                num_negative += 1;
            }
        }

        if num_negative == 0 || num_negative == 8 {
            // No crossings.
            return None;
        }

        let centroid = centroid_of_edge_intersections(&corner_dists);
        let normal = sdf_gradient(&corner_dists, centroid);

        Some((centroid, normal))
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
    #[allow(clippy::too_many_arguments)]
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
            _ => return, // No face.
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
