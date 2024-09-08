//! Signed distance field for chunked voxel objects.

use crate::{
    gpu::rendering::fre,
    voxel::{
        chunks::{
            linear_chunk_idx_within_superchunk, ChunkedVoxelObject, ExposedVoxelChunk,
            LoopForChunkVoxels, LoopForSuperchunkChunks, NonUniformVoxelChunk,
            NonUniformVoxelSuperchunk, Voxel, VoxelChunk, VoxelSuperchunk, SUPERCHUNK_CHUNK_COUNT,
            SUPERCHUNK_SIZE, SUPERCHUNK_SIZE_SQUARED,
        },
        utils::{DataLoop3, Dimension, Loop3, MutDataLoop3, Side},
    },
};

/// A signed distance field for a voxel chunk in a [`ChunkedVoxelObject`].
#[derive(Clone, Debug)]
pub struct VoxelChunkSignedDistanceField {
    sdf: [fre; SDF_VOXEL_COUNT],
}

/// The number of voxels across a cubic voxel chunk plus a padding of one
/// voxel on each side (used when storing the signed distance field for
/// a chunk).
pub const SDF_PADDED_SIZE: usize = ChunkedVoxelObject::chunk_size() + 2;

/// The total number of voxels comprising a chunk with a one-voxel padding
/// on the outside (used when storing the signed distance field for a chunk).
pub const SDF_VOXEL_COUNT: usize = SDF_PADDED_SIZE.pow(3);

type LoopForChunkSDF = Loop3<SDF_PADDED_SIZE>;
type LoopForChunkSDFData<'a, 'b> = DataLoop3<'a, 'b, fre, SDF_PADDED_SIZE>;
type LoopForChunkSDFDataMut<'a, 'b> = MutDataLoop3<'a, 'b, fre, SDF_PADDED_SIZE>;

impl VoxelChunkSignedDistanceField {
    const fn zeroed() -> Self {
        Self {
            sdf: [0.0; SDF_VOXEL_COUNT],
        }
    }

    #[inline(always)]
    fn loop_over_data<'a, 'b>(&'b self, lp: &'a LoopForChunkSDF) -> LoopForChunkSDFData<'a, 'b> {
        LoopForChunkSDFData::new(lp, &self.sdf)
    }

    #[inline(always)]
    fn loop_over_data_mut<'a, 'b>(
        &'b mut self,
        lp: &'a LoopForChunkSDF,
    ) -> LoopForChunkSDFDataMut<'a, 'b> {
        LoopForChunkSDFDataMut::new(lp, &mut self.sdf)
    }
}

impl ChunkedVoxelObject {
    /// Calls the given closure for each exposed chunk in the object, passing in
    /// the chunk and a reference to the associated
    /// [`VoxelChunkSignedDistanceField`].
    ///
    /// While the closure is guaranteed to be called for every chunk that is in
    /// any way exposed to the outside of the object, some of the chunks may not
    /// actually be exposed to the outside (for example, the chunk could be part
    /// of a closed hollow volume that crosses a superchunk boundary).
    pub fn for_each_exposed_chunk_with_sdf(
        &self,
        f: &mut impl FnMut(ExposedVoxelChunk, &VoxelChunkSignedDistanceField),
    ) {
        let mut sdf = VoxelChunkSignedDistanceField::zeroed();

        let mut superchunks = self.superchunks.iter();
        for superchunk_i in 0..self.n_superchunks_per_axis {
            for superchunk_j in 0..self.n_superchunks_per_axis {
                for superchunk_k in 0..self.n_superchunks_per_axis {
                    match superchunks.next().unwrap() {
                        VoxelSuperchunk::NonUniform(NonUniformVoxelSuperchunk {
                            start_chunk_idx,
                            flags,
                            ..
                        }) if flags.has_exposed_face() => {
                            let start_object_chunk_i = superchunk_i * SUPERCHUNK_SIZE;
                            let start_object_chunk_j = superchunk_j * SUPERCHUNK_SIZE;
                            let start_object_chunk_k = superchunk_k * SUPERCHUNK_SIZE;

                            let chunks = &self.chunks
                                [*start_chunk_idx..start_chunk_idx + SUPERCHUNK_CHUNK_COUNT];

                            LoopForSuperchunkChunks::over_interior().execute(
                                &mut |chunk_i, chunk_j, chunk_k| {
                                    let chunk_idx = linear_chunk_idx_within_superchunk(&[
                                        chunk_i, chunk_j, chunk_k,
                                    ]);
                                    match &chunks[chunk_idx] {
                                        VoxelChunk::NonUniform(chunk)
                                            if chunk.flags.has_exposed_face() =>
                                        {
                                            let object_chunk_i = start_object_chunk_i + chunk_i;
                                            let object_chunk_j = start_object_chunk_j + chunk_j;
                                            let object_chunk_k = start_object_chunk_k + chunk_k;

                                            self.fill_sdf_for_non_uniform_interior_chunk(
                                                &mut sdf, chunks, chunk_idx, chunk,
                                            );

                                            f(
                                                ExposedVoxelChunk::new([
                                                    object_chunk_i,
                                                    object_chunk_j,
                                                    object_chunk_k,
                                                ]),
                                                &sdf,
                                            );
                                        }
                                        _ => {}
                                    }
                                },
                            );

                            for boundary_loop in LoopForSuperchunkChunks::over_full_boundary() {
                                boundary_loop.execute(&mut |chunk_i, chunk_j, chunk_k| {
                                    let chunk_idx = linear_chunk_idx_within_superchunk(&[
                                        chunk_i, chunk_j, chunk_k,
                                    ]);
                                    match &chunks[chunk_idx] {
                                        VoxelChunk::NonUniform(chunk)
                                            if chunk.flags.has_exposed_face() =>
                                        {
                                            let object_chunk_i = start_object_chunk_i + chunk_i;
                                            let object_chunk_j = start_object_chunk_j + chunk_j;
                                            let object_chunk_k = start_object_chunk_k + chunk_k;

                                            self.fill_sdf_for_non_uniform_chunk(
                                                &mut sdf,
                                                [object_chunk_i, object_chunk_j, object_chunk_k],
                                                chunk,
                                            );

                                            f(
                                                ExposedVoxelChunk::new([
                                                    object_chunk_i,
                                                    object_chunk_j,
                                                    object_chunk_k,
                                                ]),
                                                &sdf,
                                            );
                                        }
                                        _ => {}
                                    }
                                });
                            }
                        }
                        _ => {
                            continue;
                        }
                    }
                }
            }
        }
    }

    fn fill_sdf_for_non_uniform_interior_chunk(
        &self,
        sdf: &mut VoxelChunkSignedDistanceField,
        superchunk_chunks: &[VoxelChunk],
        chunk_idx: usize,
        chunk: &NonUniformVoxelChunk,
    ) {
        // Since we know we are in the interior of the
        // superchunk, all adjacent chunks are in the `chunks`
        // slice

        self.fill_sdf_interior_for_non_uniform_chunk(sdf, chunk);

        #[rustfmt::skip]
        let adjacent_face_offsets = [
            (Dimension::X, Side::Lower, chunk_idx - SUPERCHUNK_SIZE_SQUARED),
            (Dimension::X, Side::Upper, chunk_idx + SUPERCHUNK_SIZE_SQUARED),
            (Dimension::Y, Side::Lower, chunk_idx - SUPERCHUNK_SIZE),
            (Dimension::Y, Side::Upper, chunk_idx + SUPERCHUNK_SIZE),
            (Dimension::Z, Side::Lower, chunk_idx - 1),
            (Dimension::Z, Side::Upper, chunk_idx + 1),
        ];

        for (dim, side, adjacent_chunk_idx) in adjacent_face_offsets {
            self.fill_sdf_face_padding_for_adjacent_chunk(
                sdf,
                dim,
                side,
                &superchunk_chunks[adjacent_chunk_idx],
            );
        }

        #[rustfmt::skip]
        let adjacent_edge_offsets = [
            (Dimension::X, Side::Lower, Side::Lower, chunk_idx - SUPERCHUNK_SIZE_SQUARED - SUPERCHUNK_SIZE),
            (Dimension::X, Side::Lower, Side::Upper, chunk_idx - SUPERCHUNK_SIZE_SQUARED + SUPERCHUNK_SIZE),
            (Dimension::X, Side::Upper, Side::Lower, chunk_idx + SUPERCHUNK_SIZE_SQUARED - SUPERCHUNK_SIZE),
            (Dimension::X, Side::Upper, Side::Upper, chunk_idx + SUPERCHUNK_SIZE_SQUARED + SUPERCHUNK_SIZE),
            (Dimension::Y, Side::Lower, Side::Lower, chunk_idx - SUPERCHUNK_SIZE - 1),
            (Dimension::Y, Side::Lower, Side::Upper, chunk_idx - SUPERCHUNK_SIZE + 1),
            (Dimension::Y, Side::Upper, Side::Lower, chunk_idx + SUPERCHUNK_SIZE - 1),
            (Dimension::Y, Side::Upper, Side::Upper, chunk_idx + SUPERCHUNK_SIZE + 1),
            (Dimension::Z, Side::Lower, Side::Lower, chunk_idx - 1 - SUPERCHUNK_SIZE_SQUARED),
            (Dimension::Z, Side::Lower, Side::Upper, chunk_idx - 1 + SUPERCHUNK_SIZE_SQUARED),
            (Dimension::Z, Side::Upper, Side::Lower, chunk_idx + 1 - SUPERCHUNK_SIZE_SQUARED),
            (Dimension::Z, Side::Upper, Side::Upper, chunk_idx + 1 + SUPERCHUNK_SIZE_SQUARED),
        ];

        for (face_dim, face_side, secondary_side, adjacent_chunk_idx) in adjacent_edge_offsets {
            self.fill_sdf_edge_padding_for_adjacent_chunk(
                sdf,
                face_dim,
                face_side,
                secondary_side,
                &superchunk_chunks[adjacent_chunk_idx],
            );
        }

        #[rustfmt::skip]
        let adjacent_corner_offsets = [
            (Side::Lower, Side::Lower, Side::Lower, chunk_idx - SUPERCHUNK_SIZE_SQUARED - SUPERCHUNK_SIZE - 1),
            (Side::Lower, Side::Lower, Side::Upper, chunk_idx - SUPERCHUNK_SIZE_SQUARED - SUPERCHUNK_SIZE + 1),
            (Side::Lower, Side::Upper, Side::Lower, chunk_idx - SUPERCHUNK_SIZE_SQUARED + SUPERCHUNK_SIZE - 1),
            (Side::Lower, Side::Upper, Side::Upper, chunk_idx - SUPERCHUNK_SIZE_SQUARED + SUPERCHUNK_SIZE + 1),
            (Side::Upper, Side::Lower, Side::Lower, chunk_idx + SUPERCHUNK_SIZE_SQUARED - SUPERCHUNK_SIZE - 1),
            (Side::Upper, Side::Lower, Side::Upper, chunk_idx + SUPERCHUNK_SIZE_SQUARED - SUPERCHUNK_SIZE + 1),
            (Side::Upper, Side::Upper, Side::Lower, chunk_idx + SUPERCHUNK_SIZE_SQUARED + SUPERCHUNK_SIZE - 1),
            (Side::Upper, Side::Upper, Side::Upper, chunk_idx + SUPERCHUNK_SIZE_SQUARED + SUPERCHUNK_SIZE + 1),
        ];

        for (x_side, y_side, z_side, adjacent_chunk_idx) in adjacent_corner_offsets {
            self.fill_sdf_corner_padding_for_adjacent_chunk(
                sdf,
                x_side,
                y_side,
                z_side,
                &superchunk_chunks[adjacent_chunk_idx],
            );
        }
    }

    fn fill_sdf_for_non_uniform_chunk(
        &self,
        sdf: &mut VoxelChunkSignedDistanceField,
        [object_chunk_i, object_chunk_j, object_chunk_k]: [usize; 3],
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

        let object_chunk_i = isize::try_from(object_chunk_i).unwrap();
        let object_chunk_j = isize::try_from(object_chunk_j).unwrap();
        let object_chunk_k = isize::try_from(object_chunk_k).unwrap();

        for (dim, side, [di, dj, dk]) in ADJACENT_FACE_OFFSETS {
            let adjacent_chunk = self.get_chunk(
                object_chunk_i + di,
                object_chunk_j + dj,
                object_chunk_k + dk,
            );
            self.fill_sdf_face_padding_for_adjacent_chunk(sdf, dim, side, &adjacent_chunk);
        }

        for (face_dim, face_side, secondary_side, [di, dj, dk]) in ADJACENT_EDGE_OFFSETS {
            let adjacent_chunk = self.get_chunk(
                object_chunk_i + di,
                object_chunk_j + dj,
                object_chunk_k + dk,
            );
            self.fill_sdf_edge_padding_for_adjacent_chunk(
                sdf,
                face_dim,
                face_side,
                secondary_side,
                &adjacent_chunk,
            );
        }

        for (x_side, y_side, z_side, [di, dj, dk]) in ADJACENT_CORNER_OFFSETS {
            let adjacent_chunk = self.get_chunk(
                object_chunk_i + di,
                object_chunk_j + dj,
                object_chunk_k + dk,
            );
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
        sdf.loop_over_data_mut(&LoopForChunkSDF::over_interior())
            .map_slice_values_into_data(voxels, &Voxel::signed_distance_value);
    }

    fn fill_sdf_face_padding_for_adjacent_chunk(
        &self,
        sdf: &mut VoxelChunkSignedDistanceField,
        dim: Dimension,
        side: Side,
        adjacent_chunk: &VoxelChunk,
    ) {
        const SDF_LOOPS: [LoopForChunkSDF; 6] = [
            LoopForChunkSDF::over_face_interior(Dimension::X, Side::Lower),
            LoopForChunkSDF::over_face_interior(Dimension::X, Side::Upper),
            LoopForChunkSDF::over_face_interior(Dimension::Y, Side::Lower),
            LoopForChunkSDF::over_face_interior(Dimension::Y, Side::Upper),
            LoopForChunkSDF::over_face_interior(Dimension::Z, Side::Lower),
            LoopForChunkSDF::over_face_interior(Dimension::Z, Side::Upper),
        ];
        const VOXEL_LOOPS: [LoopForChunkVoxels; 6] = [
            LoopForChunkVoxels::over_face(Dimension::X, Side::Lower),
            LoopForChunkVoxels::over_face(Dimension::X, Side::Upper),
            LoopForChunkVoxels::over_face(Dimension::Y, Side::Lower),
            LoopForChunkVoxels::over_face(Dimension::Y, Side::Upper),
            LoopForChunkVoxels::over_face(Dimension::Z, Side::Lower),
            LoopForChunkVoxels::over_face(Dimension::Z, Side::Upper),
        ];

        self.fill_sdf_for_adjacent_chunk_using_loops(
            adjacent_chunk,
            sdf.loop_over_data_mut(&SDF_LOOPS[2 * dim.idx() + side.idx()]),
            &VOXEL_LOOPS[2 * dim.idx() + side.opposite().idx()],
        );
    }

    fn fill_sdf_edge_padding_for_adjacent_chunk(
        &self,
        sdf: &mut VoxelChunkSignedDistanceField,
        face_dim: Dimension,
        face_side: Side,
        secondary_side: Side,
        adjacent_chunk: &VoxelChunk,
    ) {
        self.fill_sdf_for_adjacent_chunk_using_loops(
            adjacent_chunk,
            sdf.loop_over_data_mut(&LoopForChunkSDF::over_edge_interior(
                face_dim,
                face_side,
                secondary_side,
            )),
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
        self.fill_sdf_for_adjacent_chunk_using_loops(
            adjacent_chunk,
            sdf.loop_over_data_mut(&LoopForChunkSDF::over_corner(x_side, y_side, z_side)),
            &LoopForChunkVoxels::over_corner(
                x_side.opposite(),
                y_side.opposite(),
                z_side.opposite(),
            ),
        );
    }

    #[inline(always)]
    fn fill_sdf_for_adjacent_chunk_using_loops(
        &self,
        adjacent_chunk: &VoxelChunk,
        sdf_data_loop: LoopForChunkSDFDataMut<'_, '_>,
        non_uniform_chunk_loop: &LoopForChunkVoxels,
    ) {
        match adjacent_chunk {
            VoxelChunk::Empty => {
                sdf_data_loop.fill_data_with_value(Voxel::signed_distance_value_if_empty());
            }
            VoxelChunk::Uniform(voxel) => {
                sdf_data_loop.fill_data_with_value(voxel.signed_distance_value());
            }
            VoxelChunk::NonUniform(chunk) => {
                let voxels = self.non_uniform_chunk_voxels(chunk);
                sdf_data_loop.map_other_data_into_data(
                    DataLoop3::new(non_uniform_chunk_loop, voxels),
                    &Voxel::signed_distance_value,
                );
            }
        }
    }

    #[cfg(any(test, feature = "fuzzing"))]
    pub fn validate_sdf(&self) {
        self.for_each_exposed_chunk_with_sdf(&mut |chunk, sdf| {
            let lower_chunk_voxel_indices = chunk.lower_voxel_indices();

            // The SDF for the chunk is padded by one voxel
            let lower_chunk_sdf_voxel_indices = lower_chunk_voxel_indices
                .map(|voxel_index| isize::try_from(voxel_index).unwrap() - 1);

            sdf.loop_over_data(&LoopForChunkSDF::over_all()).execute(
                &mut |sdf_indices, &sdf_value| {
                    let voxel_indices = [
                        lower_chunk_sdf_voxel_indices[0] + isize::try_from(sdf_indices[0]).unwrap(),
                        lower_chunk_sdf_voxel_indices[1] + isize::try_from(sdf_indices[1]).unwrap(),
                        lower_chunk_sdf_voxel_indices[2] + isize::try_from(sdf_indices[2]).unwrap(),
                    ];

                    let voxel =
                        self.get_voxel(voxel_indices[0], voxel_indices[1], voxel_indices[2]);

                    match sdf_value.partial_cmp(&0.0) {
                        Some(std::cmp::Ordering::Less)
                            if voxel.map_or(true, |voxel| voxel.is_empty()) =>
                        {
                            eprintln!(
                                "SDF value ({}) is negative for empty voxel at indices {:?} (chunk starts at {:?})",
                                sdf_value, voxel_indices, lower_chunk_voxel_indices
                            );
                        }
                        Some(std::cmp::Ordering::Greater)
                            if voxel.map_or(false, |voxel| !voxel.is_empty()) =>
                        {
                            eprintln!(
                                "SDF value ({}) is positive for non-empty voxel at indices {:?} (chunk starts at {:?})",
                                sdf_value, voxel_indices, lower_chunk_voxel_indices
                            );
                        }
                        Some(std::cmp::Ordering::Equal) => {
                            eprintln!(
                                "SDF value is zero for voxel at indices {:?} (chunk starts at {:?})",
                                voxel_indices, lower_chunk_voxel_indices
                            );
                        }
                        None => {
                            eprintln!(
                                "SDF value is NaN for voxel at indices {:?} (chunk starts at {:?})",
                                voxel_indices, lower_chunk_voxel_indices
                            );
                        }
                        _ => {}
                    }
                },
            );
        });
    }
}

impl Voxel {
    const fn signed_distance_value_if_present() -> fre {
        -0.5
    }
    const fn signed_distance_value_if_empty() -> fre {
        0.5
    }

    fn signed_distance_value(&self) -> fre {
        if self.is_empty() {
            Self::signed_distance_value_if_empty()
        } else {
            Self::signed_distance_value_if_present()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::voxel::chunks::tests::BoxVoxelGenerator;

    #[test]
    fn should_calculate_correct_sdf_for_object_with_single_voxel() {
        let generator = BoxVoxelGenerator::single_default();
        let object = ChunkedVoxelObject::generate(&generator).unwrap();
        object.validate_sdf();
    }

    #[test]
    fn should_calculate_correct_sdf_for_object_with_full_chunk() {
        let generator = BoxVoxelGenerator::with_default([ChunkedVoxelObject::chunk_size(); 3]);
        let object = ChunkedVoxelObject::generate(&generator).unwrap();
        object.validate_sdf();
    }

    #[test]
    fn should_calculate_correct_sdf_for_object_with_two_adjacent_full_chunks() {
        let generator = BoxVoxelGenerator::with_default([
            2 * ChunkedVoxelObject::chunk_size(),
            ChunkedVoxelObject::chunk_size(),
            ChunkedVoxelObject::chunk_size(),
        ]);
        let object = ChunkedVoxelObject::generate(&generator).unwrap();
        object.validate_sdf();
    }

    #[test]
    fn should_calculate_correct_sdf_for_object_with_fully_enclosed_chunk() {
        let generator = BoxVoxelGenerator::with_default([3 * ChunkedVoxelObject::chunk_size(); 3]);
        let object = ChunkedVoxelObject::generate(&generator).unwrap();
        object.validate_sdf();
    }
}
