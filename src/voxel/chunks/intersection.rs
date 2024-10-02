//! Intersection of shapes with chunked voxel objects.

use crate::{
    geometry::Sphere,
    voxel::{
        chunks::{
            linear_chunk_idx_within_superchunk_from_object_chunk_indices,
            linear_voxel_idx_within_chunk_from_object_voxel_indices, ChunkIndex,
            ChunkedVoxelObject, NonUniformVoxelChunk, VoxelChunk, CHUNK_SIZE, CHUNK_VOXEL_COUNT,
            SUPERCHUNK_CHUNK_COUNT, SUPERCHUNK_SIZE,
        },
        Voxel,
    },
};
use nalgebra::{point, Point3};
use std::{array, ops::Range};

impl ChunkedVoxelObject {
    pub fn modify_voxels_within_sphere(
        &mut self,
        sphere: &Sphere<f64>,
        modify_voxel: &mut impl FnMut([usize; 3], Point3<f64>, &mut Voxel),
    ) {
        let touched_voxel_ranges = self.voxel_ranges_touching_sphere(sphere);

        if touched_voxel_ranges.iter().any(Range::is_empty) {
            return;
        }

        let touched_chunk_ranges = touched_voxel_ranges
            .clone()
            .map(chunk_range_encompassing_voxel_range);

        let touched_superchunk_ranges = touched_chunk_ranges
            .clone()
            .map(superchunk_range_encompassing_chunk_range);

        for superchunk_i in touched_superchunk_ranges[0].clone() {
            for superchunk_j in touched_superchunk_ranges[1].clone() {
                for superchunk_k in touched_superchunk_ranges[2].clone() {
                    let superchunk_indices = [superchunk_i, superchunk_j, superchunk_k];
                    let superchunk_idx = self.linear_superchunk_idx(&superchunk_indices);

                    let superchunk = &mut self.superchunks[superchunk_idx];

                    let start_chunk_idx = match superchunk.start_chunk_idx() {
                        ChunkIndex::AbsentEmpty => {
                            continue;
                        }
                        ChunkIndex::AbsentUniform(_) => {
                            superchunk.convert_to_non_uniform_if_uniform(&mut self.chunks);
                            superchunk.start_chunk_idx().unwrap_idx()
                        }
                        ChunkIndex::Present(idx) => idx,
                    };

                    let object_chunk_ranges_in_superchunk = superchunk_indices
                        .map(|index| index * SUPERCHUNK_SIZE..(index + 1) * SUPERCHUNK_SIZE);

                    let touched_chunk_ranges_in_superchunk: [Range<_>; 3] = array::from_fn(|dim| {
                        let range_in_superchunk = &object_chunk_ranges_in_superchunk[dim];
                        let touched_range = &touched_chunk_ranges[dim];
                        usize::max(range_in_superchunk.start, touched_range.start)
                            ..usize::min(range_in_superchunk.end, touched_range.end)
                    });

                    let chunks =
                        &mut self.chunks[start_chunk_idx..start_chunk_idx + SUPERCHUNK_CHUNK_COUNT];

                    for chunk_i in touched_chunk_ranges_in_superchunk[0].clone() {
                        for chunk_j in touched_chunk_ranges_in_superchunk[1].clone() {
                            for chunk_k in touched_chunk_ranges_in_superchunk[2].clone() {
                                let object_chunk_indices = [chunk_i, chunk_j, chunk_k];

                                let chunk_idx =
                                    linear_chunk_idx_within_superchunk_from_object_chunk_indices(
                                        chunk_i, chunk_j, chunk_k,
                                    );

                                let chunk = &mut chunks[chunk_idx];

                                let start_voxel_idx = match chunk {
                                    VoxelChunk::Empty => {
                                        continue;
                                    }
                                    VoxelChunk::Uniform(_) => {
                                        chunk.convert_to_non_uniform_if_uniform(&mut self.voxels);
                                        chunk.start_voxel_idx_if_non_uniform().unwrap()
                                    }
                                    VoxelChunk::NonUniform(NonUniformVoxelChunk {
                                        start_voxel_idx,
                                        ..
                                    }) => *start_voxel_idx,
                                };

                                let object_voxel_ranges_in_chunk = object_chunk_indices
                                    .map(|index| index * CHUNK_SIZE..(index + 1) * CHUNK_SIZE);

                                let touched_voxel_ranges_in_chunk: [Range<_>; 3] =
                                    array::from_fn(|dim| {
                                        let range_in_chunk = &object_voxel_ranges_in_chunk[dim];
                                        let touched_range = &touched_voxel_ranges[dim];
                                        usize::max(range_in_chunk.start, touched_range.start)
                                            ..usize::min(range_in_chunk.end, touched_range.end)
                                    });

                                let voxels = &mut self.voxels
                                    [start_voxel_idx..start_voxel_idx + CHUNK_VOXEL_COUNT];

                                for i in touched_voxel_ranges_in_chunk[0].clone() {
                                    for j in touched_voxel_ranges_in_chunk[1].clone() {
                                        for k in touched_voxel_ranges_in_chunk[2].clone() {
                                            let voxel_center_position =
                                                voxel_center_position_from_object_voxel_indices(
                                                    self.voxel_extent,
                                                    i,
                                                    j,
                                                    k,
                                                );

                                            if sphere.contains_point(&voxel_center_position) {
                                                let voxel_idx = linear_voxel_idx_within_chunk_from_object_voxel_indices(i, j, k);
                                                let voxel = &mut voxels[voxel_idx];
                                                if !voxel.is_empty() {
                                                    modify_voxel(
                                                        [i, j, k],
                                                        voxel_center_position,
                                                        voxel,
                                                    );
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    #[cfg(any(test, feature = "fuzzing"))]
    fn for_each_voxel_indices_in_sphere_brute_force(
        &self,
        sphere: &Sphere<f64>,
        f: &mut impl FnMut([usize; 3]),
    ) {
        for i in self.occupied_voxel_ranges[0].clone() {
            for j in self.occupied_voxel_ranges[1].clone() {
                for k in self.occupied_voxel_ranges[2].clone() {
                    let voxel_center_position =
                        voxel_center_position_from_object_voxel_indices(self.voxel_extent, i, j, k);
                    if sphere.contains_point(&voxel_center_position)
                        && self.get_voxel(i, j, k).is_some()
                    {
                        f([i, j, k]);
                    }
                }
            }
        }
    }

    fn voxel_ranges_touching_sphere(&self, sphere: &Sphere<f64>) -> [Range<usize>; 3] {
        let sphere_aabb = sphere.compute_aabb();

        let inverse_voxel_extent = self.voxel_extent().recip();
        let lower_sphere_voxel_space_coord = sphere_aabb.lower_corner() * inverse_voxel_extent;
        let upper_sphere_voxel_space_coord = sphere_aabb.upper_corner() * inverse_voxel_extent;

        let mut touched_voxel_ranges = self.occupied_voxel_ranges.clone();

        for dim in 0..3 {
            let range = &mut touched_voxel_ranges[dim];
            range.start = range
                .start
                .max(lower_sphere_voxel_space_coord[dim].floor().max(0.0) as usize);
            range.end = range
                .end
                .min(upper_sphere_voxel_space_coord[dim].ceil() as usize);
        }

        touched_voxel_ranges
    }
}

fn superchunk_range_encompassing_chunk_range(chunk_range: Range<usize>) -> Range<usize> {
    let start = chunk_range.start / SUPERCHUNK_SIZE;
    let end = chunk_range.end.div_ceil(SUPERCHUNK_SIZE);
    start..end
}

fn chunk_range_encompassing_voxel_range(voxel_range: Range<usize>) -> Range<usize> {
    let start = voxel_range.start / CHUNK_SIZE;
    let end = voxel_range.end.div_ceil(CHUNK_SIZE);
    start..end
}

fn voxel_center_position_from_object_voxel_indices(
    voxel_extent: f64,
    i: usize,
    j: usize,
    k: usize,
) -> Point3<f64> {
    point![
        (i as f64 + 0.5) * voxel_extent,
        (j as f64 + 0.5) * voxel_extent,
        (k as f64 + 0.5) * voxel_extent
    ]
}

#[cfg(feature = "fuzzing")]
pub mod fuzzing {
    use super::*;
    use crate::voxel::generation::fuzzing::ArbitrarySDFVoxelGenerator;
    use arbitrary::{Arbitrary, Result, Unstructured};
    use std::{collections::HashSet, mem};

    #[derive(Clone, Debug)]
    pub struct ArbitrarySphere(Sphere<f64>);

    impl Arbitrary<'_> for ArbitrarySphere {
        fn arbitrary(u: &mut Unstructured<'_>) -> Result<Self> {
            let radius = 1e3 * arbitrary_norm_f64(u)?;
            let x = 1e3 * arbitrary_norm_f64(u)?;
            let y = 1e3 * arbitrary_norm_f64(u)?;
            let z = 1e3 * arbitrary_norm_f64(u)?;
            Ok(Self(Sphere::new(point![x, y, z], radius)))
        }

        fn size_hint(_depth: usize) -> (usize, Option<usize>) {
            let size = 4 * mem::size_of::<i32>();
            (size, Some(size))
        }
    }

    pub fn fuzz_test_obtaining_voxels_within_sphere(
        (generator, sphere): (ArbitrarySDFVoxelGenerator, ArbitrarySphere),
    ) {
        if let Some(mut object) = ChunkedVoxelObject::generate(&generator) {
            let mut indices_of_inside_voxels = HashSet::new();

            object.modify_voxels_within_sphere(&sphere.0, &mut |indices, _, _| {
                let was_absent = indices_of_inside_voxels.insert(indices);
                assert!(was_absent, "Voxel in sphere found twice: {:?}", indices);
            });

            object.for_each_voxel_indices_in_sphere_brute_force(&sphere.0, &mut |indices| {
                let was_present = indices_of_inside_voxels.remove(&indices);
                assert!(was_present, "Voxel in sphere was not found: {:?}", indices);
            });

            assert!(
                indices_of_inside_voxels.is_empty(),
                "Found voxels not inside sphere: {:?}",
                &indices_of_inside_voxels
            );
        }
    }

    fn arbitrary_norm_f64(u: &mut Unstructured<'_>) -> Result<f64> {
        Ok(f64::from(u.int_in_range(0..=1000000)?) / 1000000.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::voxel::{
        generation::{SDFVoxelGenerator, SameVoxelTypeGenerator, SphereSDFGenerator},
        voxel_types::VoxelType,
    };
    use nalgebra::{vector, UnitVector3};
    use std::collections::HashSet;

    #[test]
    fn modifying_voxels_within_sphere_finds_correct_voxels() {
        let object_radius = 20.0;
        let sphere_radius = 0.2 * object_radius;

        let generator = SDFVoxelGenerator::new(
            1.0,
            SphereSDFGenerator::new(object_radius),
            SameVoxelTypeGenerator::new(VoxelType::default()),
        );
        let mut object = ChunkedVoxelObject::generate(&generator).unwrap();

        let sphere = Sphere::new(
            object.compute_aabb::<f64>().center()
                - UnitVector3::new_normalize(vector![1.0, 1.0, 1.0]).scale(object_radius),
            sphere_radius,
        );

        let mut indices_of_inside_voxels = HashSet::new();

        object.modify_voxels_within_sphere(&sphere, &mut |indices, _, _| {
            let was_absent = indices_of_inside_voxels.insert(indices);
            assert!(was_absent, "Voxel in sphere found twice: {:?}", indices);
        });

        object.for_each_voxel_indices_in_sphere_brute_force(&sphere, &mut |indices| {
            let was_present = indices_of_inside_voxels.remove(&indices);
            assert!(was_present, "Voxel in sphere was not found: {:?}", indices);
        });

        assert!(
            indices_of_inside_voxels.is_empty(),
            "Found voxels not inside sphere: {:?}",
            &indices_of_inside_voxels
        );
    }
}
