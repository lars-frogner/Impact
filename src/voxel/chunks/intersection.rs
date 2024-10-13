//! Intersection of shapes with chunked voxel objects.

use super::chunk_voxels_mut;
use crate::{
    geometry::Sphere,
    voxel::{
        chunks::{
            linear_voxel_idx_within_chunk_from_object_voxel_indices, ChunkedVoxelObject,
            VoxelChunk, CHUNK_SIZE,
        },
        Voxel,
    },
};
use nalgebra::{self as na, point, Point3};
use std::{array, ops::Range};

impl ChunkedVoxelObject {
    /// Finds all non-empty voxels whose center fall within the given sphere and
    /// calls the given closure with the voxel's indices, squared distance
    /// between the voxel center and the center of the sphere and a mutable
    /// reference to the voxel itself.
    ///
    /// Since it is assumed that the given closure will modify the voxels, the
    /// adjacency information will be updated for all voxels within the sphere,
    /// and any chunk whose mesh data would be invalidated by changes to these
    /// voxels will be registered. The invalidated chunks can be obtained by
    /// calling [`Self::invalidated_mesh_chunk_indices`].
    pub fn modify_voxels_within_sphere(
        &mut self,
        sphere: &Sphere<f64>,
        modify_voxel: &mut impl FnMut([usize; 3], f64, &mut Voxel),
    ) {
        let touched_voxel_ranges = self.voxel_ranges_touching_sphere(sphere);

        if touched_voxel_ranges.iter().any(Range::is_empty) {
            return;
        }

        let touched_chunk_ranges = touched_voxel_ranges
            .clone()
            .map(chunk_range_encompassing_voxel_range);

        for chunk_i in touched_chunk_ranges[0].clone() {
            for chunk_j in touched_chunk_ranges[1].clone() {
                for chunk_k in touched_chunk_ranges[2].clone() {
                    let chunk_indices = [chunk_i, chunk_j, chunk_k];
                    let chunk_idx = self.linear_chunk_idx(&chunk_indices);

                    let chunk = &mut self.chunks[chunk_idx];

                    let chunk = match chunk {
                        VoxelChunk::Empty => {
                            continue;
                        }
                        VoxelChunk::Uniform(_) => {
                            chunk.convert_to_non_uniform_if_uniform(
                                &mut self.voxels,
                                &mut self.split_detector,
                            );
                            if let VoxelChunk::NonUniform(chunk) = chunk {
                                chunk
                            } else {
                                unreachable!()
                            }
                        }
                        VoxelChunk::NonUniform(chunk) => chunk,
                    };

                    let object_voxel_ranges_in_chunk =
                        chunk_indices.map(|index| index * CHUNK_SIZE..(index + 1) * CHUNK_SIZE);

                    let touched_voxel_ranges_in_chunk: [_; 3] = array::from_fn(|dim| {
                        let range_in_chunk = &object_voxel_ranges_in_chunk[dim];
                        let touched_range = &touched_voxel_ranges[dim];
                        usize::max(range_in_chunk.start, touched_range.start)
                            ..usize::min(range_in_chunk.end, touched_range.end)
                    });

                    let voxels = chunk_voxels_mut(&mut self.voxels, chunk.data_offset);

                    let mut chunk_touched = false;

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

                                let distance_squared =
                                    na::distance_squared(sphere.center(), &voxel_center_position);

                                if distance_squared < sphere.radius_squared() {
                                    let voxel_idx =
                                        linear_voxel_idx_within_chunk_from_object_voxel_indices(
                                            i, j, k,
                                        );
                                    let voxel = &mut voxels[voxel_idx];
                                    modify_voxel([i, j, k], distance_squared, voxel);
                                    chunk_touched = true;
                                }
                            }
                        }
                    }

                    if chunk_touched {
                        chunk.update_face_distributions_and_internal_adjacencies(&mut self.voxels);

                        self.split_detector
                            .update_local_connected_regions_for_chunk(
                                &self.voxels,
                                chunk,
                                chunk_idx as u32,
                            );

                        // The mesh of the touched chunk is invalidated
                        self.invalidated_mesh_chunk_indices.insert(chunk_indices);

                        for dim in 0..3 {
                            // The meshes of adjacent chunks are invalidated if any voxel within 2
                            // voxels of the relevant boundary was touched (that is, a boundary
                            // voxel or a voxel adjacent to a boundary voxel)

                            let voxel_range = &object_voxel_ranges_in_chunk[dim];
                            let touched_voxel_range = &touched_voxel_ranges_in_chunk[dim];

                            if chunk_indices[dim] > self.occupied_chunk_ranges[dim].start
                                && touched_voxel_range.start - voxel_range.start < 2
                            {
                                let mut adjacent_chunk_indices = chunk_indices;
                                adjacent_chunk_indices[dim] -= 1;
                                self.invalidated_mesh_chunk_indices
                                    .insert(adjacent_chunk_indices);
                            }

                            if chunk_indices[dim] < self.occupied_chunk_ranges[dim].end - 1
                                && voxel_range.end - touched_voxel_range.end < 2
                            {
                                let mut adjacent_chunk_indices = chunk_indices;
                                adjacent_chunk_indices[dim] += 1;
                                self.invalidated_mesh_chunk_indices
                                    .insert(adjacent_chunk_indices);
                            }
                        }
                    }
                }
            }
        }

        self.update_upper_boundary_adjacencies_for_chunks_in_ranges(touched_chunk_ranges);

        self.resolve_connected_regions_between_all_chunks();
        dbg!(self.count_regions());
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
                    if sphere.contains_point(&voxel_center_position) {
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
