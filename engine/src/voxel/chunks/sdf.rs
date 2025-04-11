//! Signed distance field for chunked voxel objects.

pub mod surface_nets;

use crate::voxel::{
    VoxelSignedDistance,
    chunks::{
        ChunkedVoxelObject, ExposedVoxelChunk, LoopForChunkVoxels, NonUniformVoxelChunk,
        UniformVoxelChunk, VoxelChunk, VoxelChunkFlags,
    },
    utils::{DataLoop3, Dimension, Loop3, MutDataLoop3, Side},
    voxel_types::VoxelType,
};

/// A signed distance field for a voxel chunk in a [`ChunkedVoxelObject`].
#[derive(Clone, Debug)]
pub struct VoxelChunkSignedDistanceField {
    /// The reason we store the values as `f32` instead of the more compact
    /// [`VoxelSignedDistance`] is that mesh generation using surface nets needs
    /// to load each value many times, and the having to decode to an `f32`
    /// every time more than eats up the performance gain from the smaller
    /// element size.
    values: [f32; SDF_GRID_CELL_COUNT],
    voxel_types: [VoxelType; SDF_GRID_CELL_COUNT],
    adjacent_is_non_uniform: [[bool; 2]; 3],
}

/// The number of grid cells holding a signed distance in the SDF grid for a
/// single voxel chunk (equals the number of voxels in the chunk plus one
/// cell of padding on each side).
const SDF_GRID_SIZE: usize = ChunkedVoxelObject::chunk_size() + 2;

/// The number of grid cells holding a signed distance in the SDF grid for a
/// single voxel chunk (equals the number of voxels in the chunk plus one
/// cell of padding on each side).
pub const SDF_GRID_CELL_COUNT: usize = SDF_GRID_SIZE.pow(3);

type LoopForChunkSDF = Loop3<SDF_GRID_SIZE>;
#[cfg(any(test, feature = "fuzzing"))]
type LoopForChunkSDFValues<'a, 'b> = DataLoop3<'a, 'b, f32, SDF_GRID_SIZE>;
#[cfg(any(test, feature = "fuzzing"))]
type LoopForChunkSDFVoxelTypes<'a, 'b> = DataLoop3<'a, 'b, VoxelType, SDF_GRID_SIZE>;
type LoopForChunkSDFValuesMut<'a, 'b> = MutDataLoop3<'a, 'b, f32, SDF_GRID_SIZE>;
type LoopForChunkSDFVoxelTypesMut<'a, 'b> = MutDataLoop3<'a, 'b, VoxelType, SDF_GRID_SIZE>;

impl VoxelChunkSignedDistanceField {
    /// The number of grid cells holding a signed distance in the SDF grid for a
    /// single voxel chunk (equals the number of voxels in the chunk plus one
    /// cell of padding on each side).
    pub const fn grid_size() -> usize {
        SDF_GRID_SIZE
    }

    /// The total number of grid cells holding a signed distance in the SDF grid
    /// for a single voxel chunk, including the padding cells on the boundary.
    pub const fn grid_cell_count() -> usize {
        SDF_GRID_CELL_COUNT
    }

    const fn grid_size_u32() -> u32 {
        Self::grid_size() as u32
    }

    const fn squared_grid_size() -> usize {
        Self::grid_size() * Self::grid_size()
    }

    const fn linear_idx(indices: &[usize; 3]) -> usize {
        indices[0] * Self::squared_grid_size() + indices[1] * Self::grid_size() + indices[2]
    }

    const fn linear_idx_u32(indices: &[u32; 3]) -> u32 {
        indices[0] * Self::squared_grid_size() as u32
            + indices[1] * Self::grid_size_u32()
            + indices[2]
    }

    #[allow(clippy::large_stack_arrays)]
    pub const fn default() -> Self {
        Self {
            values: [0.0; SDF_GRID_CELL_COUNT],
            voxel_types: [VoxelType::dummy(); SDF_GRID_CELL_COUNT],
            adjacent_is_non_uniform: [[false; 2]; 3],
        }
    }

    pub fn get_value(&self, i: usize, j: usize, k: usize) -> Option<f32> {
        self.values.get(Self::linear_idx(&[i, j, k])).copied()
    }

    #[cfg(any(test, feature = "fuzzing"))]
    fn loop_over_sdf_values<'a, 'b>(
        &'b self,
        lp: &'a LoopForChunkSDF,
    ) -> LoopForChunkSDFValues<'a, 'b> {
        LoopForChunkSDFValues::new(lp, &self.values)
    }

    #[cfg(any(test, feature = "fuzzing"))]
    fn loop_over_voxel_types<'a, 'b>(
        &'b self,
        lp: &'a LoopForChunkSDF,
    ) -> LoopForChunkSDFVoxelTypes<'a, 'b> {
        LoopForChunkSDFVoxelTypes::new(lp, &self.voxel_types)
    }

    fn loops_over_sdf_values_and_voxel_types_mut<'a, 'b>(
        &'b mut self,
        lp: &'a LoopForChunkSDF,
    ) -> (
        LoopForChunkSDFValuesMut<'a, 'b>,
        LoopForChunkSDFVoxelTypesMut<'a, 'b>,
    ) {
        (
            LoopForChunkSDFValuesMut::new(lp, &mut self.values),
            LoopForChunkSDFVoxelTypesMut::new(lp, &mut self.voxel_types),
        )
    }

    fn set_adjacent_is_non_uniform(&mut self, dim: Dimension, side: Side, is_non_uniform: bool) {
        self.adjacent_is_non_uniform[dim.idx()][side.idx()] = is_non_uniform;
    }

    fn adjacent_is_non_uniform(&self, dim: Dimension, side: Side) -> bool {
        self.adjacent_is_non_uniform[dim.idx()][side.idx()]
    }
}

impl ChunkedVoxelObject {
    /// Calls the given closure for each exposed chunk in the object, passing in
    /// the chunk and a reference to the given
    /// [`VoxelChunkSignedDistanceField`] that has been filled with signed
    /// distances for the chunk.
    ///
    /// While the closure is guaranteed to be called for every chunk that is in
    /// any way exposed to the outside of the object, some of the chunks may not
    /// actually be exposed to the outside (for example, the chunk could be part
    /// of a closed hollow volume).
    pub fn for_each_exposed_chunk_with_sdf(
        &self,
        sdf: &mut VoxelChunkSignedDistanceField,
        f: &mut impl FnMut(ExposedVoxelChunk, &VoxelChunkSignedDistanceField),
    ) {
        for chunk_i in self.occupied_chunk_ranges[0].clone() {
            for chunk_j in self.occupied_chunk_ranges[1].clone() {
                for chunk_k in self.occupied_chunk_ranges[2].clone() {
                    let chunk_indices = [chunk_i, chunk_j, chunk_k];
                    if let Some(chunk_flags) =
                        self.fill_sdf_for_chunk_if_exposed(sdf, chunk_indices)
                    {
                        f(ExposedVoxelChunk::new(chunk_indices, chunk_flags), sdf);
                    }
                }
            }
        }
    }

    /// If the voxel chunk at the given indices in the object's chunk grid is
    /// exposed, fills the given [`VoxelChunkSignedDistanceField`] for it and
    /// returns its [`VoxelChunkFlags`]. Otherwise, returns [`None`].
    pub fn fill_sdf_for_chunk_if_exposed(
        &self,
        sdf: &mut VoxelChunkSignedDistanceField,
        chunk_indices: [usize; 3],
    ) -> Option<VoxelChunkFlags> {
        assert!(
            chunk_indices
                .iter()
                .zip(self.chunk_counts())
                .all(|(&index, &count)| index < count)
        );

        let chunk_idx = self.linear_chunk_idx(&chunk_indices);

        match &self.chunks[chunk_idx] {
            VoxelChunk::NonUniform(chunk) if chunk.flags.has_exposed_face() => {
                if chunk_indices[0] > 0
                    && chunk_indices[0] < self.chunk_counts[0] - 1
                    && chunk_indices[1] > 0
                    && chunk_indices[1] < self.chunk_counts[1] - 1
                    && chunk_indices[2] > 0
                    && chunk_indices[2] < self.chunk_counts[2] - 1
                {
                    self.fill_sdf_for_non_uniform_interior_chunk(sdf, chunk_idx, chunk);
                } else {
                    self.fill_sdf_for_non_uniform_chunk(sdf, chunk_indices, chunk);
                }

                Some(chunk.flags)
            }
            _ => None,
        }
    }

    fn fill_sdf_for_non_uniform_interior_chunk(
        &self,
        sdf: &mut VoxelChunkSignedDistanceField,
        chunk_idx: usize,
        chunk: &NonUniformVoxelChunk,
    ) {
        // Since we know we are in the interior of the object, all adjacent
        // chunks are in the `chunks` slice

        self.fill_sdf_interior_for_non_uniform_chunk(sdf, chunk);

        #[rustfmt::skip]
        let adjacent_face_offsets = [
            (Dimension::X, Side::Lower, chunk_idx - self.chunk_idx_strides[0]),
            (Dimension::X, Side::Upper, chunk_idx + self.chunk_idx_strides[0]),
            (Dimension::Y, Side::Lower, chunk_idx - self.chunk_idx_strides[1]),
            (Dimension::Y, Side::Upper, chunk_idx + self.chunk_idx_strides[1]),
            (Dimension::Z, Side::Lower, chunk_idx - 1),
            (Dimension::Z, Side::Upper, chunk_idx + 1),
        ];

        for (dim, side, adjacent_chunk_idx) in adjacent_face_offsets {
            self.fill_sdf_face_padding_for_adjacent_chunk(
                sdf,
                dim,
                side,
                &self.chunks[adjacent_chunk_idx],
            );
        }

        #[rustfmt::skip]
        let adjacent_edge_offsets = [
            (Dimension::X, Side::Lower, Side::Lower, chunk_idx - self.chunk_idx_strides[0] - self.chunk_idx_strides[1]),
            (Dimension::X, Side::Lower, Side::Upper, chunk_idx - self.chunk_idx_strides[0] + self.chunk_idx_strides[1]),
            (Dimension::X, Side::Upper, Side::Lower, chunk_idx + self.chunk_idx_strides[0] - self.chunk_idx_strides[1]),
            (Dimension::X, Side::Upper, Side::Upper, chunk_idx + self.chunk_idx_strides[0] + self.chunk_idx_strides[1]),
            (Dimension::Y, Side::Lower, Side::Lower, chunk_idx - self.chunk_idx_strides[1] - 1),
            (Dimension::Y, Side::Lower, Side::Upper, chunk_idx - self.chunk_idx_strides[1] + 1),
            (Dimension::Y, Side::Upper, Side::Lower, chunk_idx + self.chunk_idx_strides[1] - 1),
            (Dimension::Y, Side::Upper, Side::Upper, chunk_idx + self.chunk_idx_strides[1] + 1),
            (Dimension::Z, Side::Lower, Side::Lower, chunk_idx - 1 - self.chunk_idx_strides[0]),
            (Dimension::Z, Side::Lower, Side::Upper, chunk_idx - 1 + self.chunk_idx_strides[0]),
            (Dimension::Z, Side::Upper, Side::Lower, chunk_idx + 1 - self.chunk_idx_strides[0]),
            (Dimension::Z, Side::Upper, Side::Upper, chunk_idx + 1 + self.chunk_idx_strides[0]),
        ];

        for (face_dim, face_side, secondary_side, adjacent_chunk_idx) in adjacent_edge_offsets {
            self.fill_sdf_edge_padding_for_adjacent_chunk(
                sdf,
                face_dim,
                face_side,
                secondary_side,
                &self.chunks[adjacent_chunk_idx],
            );
        }

        #[rustfmt::skip]
        let adjacent_corner_offsets = [
            (Side::Lower, Side::Lower, Side::Lower, chunk_idx - self.chunk_idx_strides[0] - self.chunk_idx_strides[1] - 1),
            (Side::Lower, Side::Lower, Side::Upper, chunk_idx - self.chunk_idx_strides[0] - self.chunk_idx_strides[1] + 1),
            (Side::Lower, Side::Upper, Side::Lower, chunk_idx - self.chunk_idx_strides[0] + self.chunk_idx_strides[1] - 1),
            (Side::Lower, Side::Upper, Side::Upper, chunk_idx - self.chunk_idx_strides[0] + self.chunk_idx_strides[1] + 1),
            (Side::Upper, Side::Lower, Side::Lower, chunk_idx + self.chunk_idx_strides[0] - self.chunk_idx_strides[1] - 1),
            (Side::Upper, Side::Lower, Side::Upper, chunk_idx + self.chunk_idx_strides[0] - self.chunk_idx_strides[1] + 1),
            (Side::Upper, Side::Upper, Side::Lower, chunk_idx + self.chunk_idx_strides[0] + self.chunk_idx_strides[1] - 1),
            (Side::Upper, Side::Upper, Side::Upper, chunk_idx + self.chunk_idx_strides[0] + self.chunk_idx_strides[1] + 1),
        ];

        for (x_side, y_side, z_side, adjacent_chunk_idx) in adjacent_corner_offsets {
            self.fill_sdf_corner_padding_for_adjacent_chunk(
                sdf,
                x_side,
                y_side,
                z_side,
                &self.chunks[adjacent_chunk_idx],
            );
        }
    }

    fn fill_sdf_for_non_uniform_chunk(
        &self,
        sdf: &mut VoxelChunkSignedDistanceField,
        [chunk_i, chunk_j, chunk_k]: [usize; 3],
        chunk: &NonUniformVoxelChunk,
    ) {
        #[rustfmt::skip]
        const ADJACENT_FACE_OFFSETS: [(Dimension, Side, [isize; 3]); 6] = [
            (Dimension::X, Side::Lower, [-1, 0, 0]),
            (Dimension::X, Side::Upper, [1, 0, 0]),
            (Dimension::Y, Side::Lower, [0, -1, 0]),
            (Dimension::Y, Side::Upper, [0, 1, 0]),
            (Dimension::Z, Side::Lower, [0, 0, -1]),
            (Dimension::Z, Side::Upper, [0, 0, 1]),
        ];

        #[rustfmt::skip]
        const ADJACENT_EDGE_OFFSETS: [(Dimension, Side, Side, [isize; 3]); 12] = [
            (Dimension::X, Side::Lower, Side::Lower, [-1, -1, 0]),
            (Dimension::X, Side::Lower, Side::Upper, [-1, 1, 0]),
            (Dimension::X, Side::Upper, Side::Lower, [1, -1, 0]),
            (Dimension::X, Side::Upper, Side::Upper, [1, 1, 0]),
            (Dimension::Y, Side::Lower, Side::Lower, [0, -1, -1]),
            (Dimension::Y, Side::Lower, Side::Upper, [0, -1, 1]),
            (Dimension::Y, Side::Upper, Side::Lower, [0, 1, -1]),
            (Dimension::Y, Side::Upper, Side::Upper, [0, 1, 1]),
            (Dimension::Z, Side::Lower, Side::Lower, [-1, 0, -1]),
            (Dimension::Z, Side::Lower, Side::Upper, [1, 0, -1]),
            (Dimension::Z, Side::Upper, Side::Lower, [-1, 0, 1]),
            (Dimension::Z, Side::Upper, Side::Upper, [1, 0, 1]),
        ];

        #[rustfmt::skip]
        const ADJACENT_CORNER_OFFSETS: [(Side, Side, Side, [isize; 3]); 8] = [
            (Side::Lower, Side::Lower, Side::Lower, [-1, -1, -1]),
            (Side::Lower, Side::Lower, Side::Upper, [-1, -1, 1]),
            (Side::Lower, Side::Upper, Side::Lower, [-1, 1, -1]),
            (Side::Lower, Side::Upper, Side::Upper, [-1, 1, 1]),
            (Side::Upper, Side::Lower, Side::Lower, [1, -1, -1]),
            (Side::Upper, Side::Lower, Side::Upper, [1, -1, 1]),
            (Side::Upper, Side::Upper, Side::Lower, [1, 1, -1]),
            (Side::Upper, Side::Upper, Side::Upper, [1, 1, 1]),
        ];

        self.fill_sdf_interior_for_non_uniform_chunk(sdf, chunk);

        let chunk_i = isize::try_from(chunk_i).unwrap();
        let chunk_j = isize::try_from(chunk_j).unwrap();
        let chunk_k = isize::try_from(chunk_k).unwrap();

        for (dim, side, [di, dj, dk]) in ADJACENT_FACE_OFFSETS {
            let adjacent_chunk = self.get_chunk(chunk_i + di, chunk_j + dj, chunk_k + dk);
            self.fill_sdf_face_padding_for_adjacent_chunk(sdf, dim, side, &adjacent_chunk);
        }

        for (face_dim, face_side, secondary_side, [di, dj, dk]) in ADJACENT_EDGE_OFFSETS {
            let adjacent_chunk = self.get_chunk(chunk_i + di, chunk_j + dj, chunk_k + dk);
            self.fill_sdf_edge_padding_for_adjacent_chunk(
                sdf,
                face_dim,
                face_side,
                secondary_side,
                &adjacent_chunk,
            );
        }

        for (x_side, y_side, z_side, [di, dj, dk]) in ADJACENT_CORNER_OFFSETS {
            let adjacent_chunk = self.get_chunk(chunk_i + di, chunk_j + dj, chunk_k + dk);
            self.fill_sdf_corner_padding_for_adjacent_chunk(
                sdf,
                x_side,
                y_side,
                z_side,
                &adjacent_chunk,
            );
        }
    }

    fn fill_sdf_interior_for_non_uniform_chunk(
        &self,
        sdf: &mut VoxelChunkSignedDistanceField,
        chunk: &NonUniformVoxelChunk,
    ) {
        let voxels = self.non_uniform_chunk_voxels(chunk);

        let sdf_loop = LoopForChunkSDF::over_interior();
        let (sdf_values_loop, sdf_voxel_types_loop) =
            sdf.loops_over_sdf_values_and_voxel_types_mut(&sdf_loop);

        sdf_values_loop
            .map_slice_values_into_data(voxels, &|voxel| voxel.signed_distance().to_f32());

        sdf_voxel_types_loop.map_slice_values_into_data(voxels, &|voxel| voxel.voxel_type());
    }

    fn fill_sdf_face_padding_for_adjacent_chunk(
        &self,
        sdf: &mut VoxelChunkSignedDistanceField,
        dim: Dimension,
        side: Side,
        adjacent_chunk: &VoxelChunk,
    ) {
        let sdf_loop = LoopForChunkSDF::over_face_interior(dim, side);
        let (sdf_values_loop, sdf_voxel_types_loop) =
            sdf.loops_over_sdf_values_and_voxel_types_mut(&sdf_loop);

        let adjacent_is_non_uniform = self.fill_sdf_for_adjacent_chunk_using_loops(
            adjacent_chunk,
            sdf_values_loop,
            sdf_voxel_types_loop,
            &LoopForChunkVoxels::over_face(dim, side.opposite()),
        );
        sdf.set_adjacent_is_non_uniform(dim, side, adjacent_is_non_uniform);
    }

    fn fill_sdf_edge_padding_for_adjacent_chunk(
        &self,
        sdf: &mut VoxelChunkSignedDistanceField,
        face_dim: Dimension,
        face_side: Side,
        secondary_side: Side,
        adjacent_chunk: &VoxelChunk,
    ) {
        let sdf_loop = LoopForChunkSDF::over_edge_interior(face_dim, face_side, secondary_side);
        let (sdf_values_loop, sdf_voxel_types_loop) =
            sdf.loops_over_sdf_values_and_voxel_types_mut(&sdf_loop);

        self.fill_sdf_for_adjacent_chunk_using_loops(
            adjacent_chunk,
            sdf_values_loop,
            sdf_voxel_types_loop,
            &LoopForChunkVoxels::over_edge(
                face_dim,
                face_side.opposite(),
                secondary_side.opposite(),
            ),
        );
    }

    fn fill_sdf_corner_padding_for_adjacent_chunk(
        &self,
        sdf: &mut VoxelChunkSignedDistanceField,
        x_side: Side,
        y_side: Side,
        z_side: Side,
        adjacent_chunk: &VoxelChunk,
    ) {
        let sdf_loop = LoopForChunkSDF::over_corner(x_side, y_side, z_side);
        let (sdf_values_loop, sdf_voxel_types_loop) =
            sdf.loops_over_sdf_values_and_voxel_types_mut(&sdf_loop);

        self.fill_sdf_for_adjacent_chunk_using_loops(
            adjacent_chunk,
            sdf_values_loop,
            sdf_voxel_types_loop,
            &LoopForChunkVoxels::over_corner(
                x_side.opposite(),
                y_side.opposite(),
                z_side.opposite(),
            ),
        );
    }

    fn fill_sdf_for_adjacent_chunk_using_loops(
        &self,
        adjacent_chunk: &VoxelChunk,
        sdf_values_loop: LoopForChunkSDFValuesMut<'_, '_>,
        voxel_types_loop: LoopForChunkSDFVoxelTypesMut<'_, '_>,
        non_uniform_chunk_loop: &LoopForChunkVoxels,
    ) -> bool {
        match adjacent_chunk {
            VoxelChunk::Empty => {
                sdf_values_loop
                    .fill_data_with_value(VoxelSignedDistance::maximally_outside().to_f32());
                false
            }
            VoxelChunk::Uniform(UniformVoxelChunk { voxel, .. }) => {
                sdf_values_loop
                    .fill_data_with_value(VoxelSignedDistance::maximally_inside().to_f32());
                voxel_types_loop.fill_data_with_value(voxel.voxel_type());
                false
            }
            VoxelChunk::NonUniform(chunk) => {
                let voxels = self.non_uniform_chunk_voxels(chunk);
                let chunk_voxel_loop = DataLoop3::new(non_uniform_chunk_loop, voxels);
                sdf_values_loop.map_other_data_into_data(&chunk_voxel_loop, &|voxel| {
                    voxel.signed_distance().to_f32()
                });
                voxel_types_loop
                    .map_other_data_into_data(&chunk_voxel_loop, &|voxel| voxel.voxel_type());
                true
            }
        }
    }

    #[cfg(any(test, feature = "fuzzing"))]
    pub fn validate_sdf(&self) {
        let mut sdf = VoxelChunkSignedDistanceField::default();
        self.for_each_exposed_chunk_with_sdf(&mut sdf,&mut |chunk, sdf| {
            let lower_chunk_voxel_indices = chunk.lower_voxel_indices();

            // The SDF for the chunk is padded by one voxel
            let lower_chunk_sdf_voxel_indices = lower_chunk_voxel_indices
                .map(|voxel_index| isize::try_from(voxel_index).unwrap() - 1);

            sdf.loop_over_sdf_values(&LoopForChunkSDF::over_all()).execute(
                &mut |sdf_indices, &signed_dist| {
                    let voxel_indices = [
                        lower_chunk_sdf_voxel_indices[0] + isize::try_from(sdf_indices[0]).unwrap(),
                        lower_chunk_sdf_voxel_indices[1] + isize::try_from(sdf_indices[1]).unwrap(),
                        lower_chunk_sdf_voxel_indices[2] + isize::try_from(sdf_indices[2]).unwrap(),
                    ];

                    let voxel =
                        self.get_voxel(voxel_indices[0], voxel_indices[1], voxel_indices[2]);

                    if signed_dist.is_sign_negative() && voxel.is_none_or(|voxel| voxel.is_empty()) {
                        panic!(
                            "SDF value ({}) is negative for empty voxel at indices {:?} (chunk starts at {:?})",
                            signed_dist, voxel_indices, lower_chunk_voxel_indices
                        );
                    } else if signed_dist.is_sign_positive() && voxel.is_some_and(|voxel| !voxel.is_empty()) {
                        panic!(
                            "SDF value ({}) is non-negative for non-empty voxel at indices {:?} (chunk starts at {:?})",
                            signed_dist, voxel_indices, lower_chunk_voxel_indices
                        );
                    }
                },
            );

            sdf.loop_over_voxel_types(&LoopForChunkSDF::over_all()).execute(
                &mut |sdf_indices, &voxel_type| {
                    let voxel_indices = [
                        lower_chunk_sdf_voxel_indices[0] + isize::try_from(sdf_indices[0]).unwrap(),
                        lower_chunk_sdf_voxel_indices[1] + isize::try_from(sdf_indices[1]).unwrap(),
                        lower_chunk_sdf_voxel_indices[2] + isize::try_from(sdf_indices[2]).unwrap(),
                    ];

                    let voxel =
                        self.get_voxel(voxel_indices[0], voxel_indices[1], voxel_indices[2]);

                    if matches!(voxel, Some(v) if v.voxel_type() != voxel_type) {
                        panic!(
                            "Recorded voxel type ({:?}) differs from actual voxel type ({:?}) at indices {:?} (chunk starts at {:?})",
                            voxel_type, voxel.unwrap().voxel_type(), voxel_indices, lower_chunk_voxel_indices
                        );
                    }
                },
            );
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::voxel::chunks::tests::OffsetBoxVoxelGenerator;

    #[test]
    fn should_calculate_valid_sdf_for_object_with_single_voxel() {
        let generator = OffsetBoxVoxelGenerator::single_default();
        let object = ChunkedVoxelObject::generate(&generator).unwrap();
        object.validate_sdf();
    }

    #[test]
    fn should_calculate_valid_sdf_for_object_with_full_chunk() {
        let generator =
            OffsetBoxVoxelGenerator::with_default([ChunkedVoxelObject::chunk_size(); 3]);
        let object = ChunkedVoxelObject::generate(&generator).unwrap();
        object.validate_sdf();
    }

    #[test]
    fn should_calculate_valid_sdf_for_object_with_two_adjacent_full_chunks() {
        let generator = OffsetBoxVoxelGenerator::with_default([
            2 * ChunkedVoxelObject::chunk_size(),
            ChunkedVoxelObject::chunk_size(),
            ChunkedVoxelObject::chunk_size(),
        ]);
        let object = ChunkedVoxelObject::generate(&generator).unwrap();
        object.validate_sdf();
    }

    #[test]
    fn should_calculate_valid_sdf_for_object_with_fully_enclosed_chunk() {
        let generator =
            OffsetBoxVoxelGenerator::with_default([3 * ChunkedVoxelObject::chunk_size(); 3]);
        let object = ChunkedVoxelObject::generate(&generator).unwrap();
        object.validate_sdf();
    }
}
