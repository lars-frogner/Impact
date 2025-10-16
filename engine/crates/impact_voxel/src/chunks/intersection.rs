//! Intersection of shapes with chunked voxel objects.

use super::{chunk_voxels, chunk_voxels_mut};
use crate::{
    Voxel, VoxelPlacement, VoxelSurfacePlacement,
    chunks::{
        CHUNK_SIZE, ChunkedVoxelObject, VoxelChunk, disconnection::SplitDetector,
        linear_voxel_idx_within_chunk_from_object_voxel_indices,
    },
};
use impact_containers::HashSet;
use impact_geometry::{AxisAlignedBox, Capsule, OrientedBox, Plane, Sphere};
use nalgebra::{self as na, Isometry3, Point3, point};
use std::{array, ops::Range};

pub type VoxelRanges = [Range<usize>; 3];
pub type ChunkRanges = [Range<usize>; 3];

impl ChunkedVoxelObject {
    /// Finds non-empty voxels with at least one exposed face that are not fully
    /// outside the negative halfspace of the given plane and calls the given
    /// closure with their indices, the voxels themselves and their placement on
    /// the surface.
    ///
    /// The plane should be specified in the model space of the voxel object,
    /// where the lower corner of the grid is at the origin and the cartesian
    /// axes are aligned with the grid.
    ///
    /// For efficiency, the closure may also be called with voxels
    /// that are fully outside the negative halfspace of the plane.
    pub fn for_each_surface_voxel_maybe_intersecting_negative_halfspace_of_plane(
        &self,
        plane: &Plane<f64>,
        f: &mut impl FnMut([usize; 3], &Voxel, VoxelSurfacePlacement),
    ) {
        let normalized_plane = plane.scaled(self.voxel_extent.recip());
        let included_voxel_ranges = self.voxel_ranges_in_object_within_plane(&normalized_plane);
        self.for_each_surface_voxel_in_voxel_ranges(included_voxel_ranges, f);
    }

    /// Finds non-empty voxels with at least one exposed face that are not
    /// fully outside the given sphere and calls the given closure with
    /// their indices, the voxels themselves and their placement on the
    /// surface.
    ///
    /// The sphere should be specified in the model space of the voxel object,
    /// where the lower corner of the grid is at the origin and the cartesian
    /// axes are aligned with the grid.
    ///
    /// For efficiency, the closure may also be called with voxels
    /// that are fully outside the sphere.
    pub fn for_each_surface_voxel_maybe_intersecting_sphere(
        &self,
        sphere: &Sphere<f64>,
        f: &mut impl FnMut([usize; 3], &Voxel, VoxelSurfacePlacement),
    ) {
        let normalized_sphere = sphere.scaled(self.voxel_extent.recip());
        let touched_voxel_ranges =
            self.voxel_ranges_in_object_touching_aab(&normalized_sphere.compute_aabb());
        self.for_each_surface_voxel_in_voxel_ranges(touched_voxel_ranges, f);
    }

    /// Finds non-empty voxels with at least one exposed face and calls the
    /// given closure with their indices, the voxels themselves and their
    /// placement on the surface.
    pub fn for_each_surface_voxel(
        &self,
        f: &mut impl FnMut([usize; 3], &Voxel, VoxelSurfacePlacement),
    ) {
        self.for_each_surface_voxel_in_voxel_ranges(self.occupied_voxel_ranges.clone(), f);
    }

    /// Finds non-empty voxels with at least one exposed face in the given voxel
    /// ranges and calls the given closure with their indices, the voxels
    /// themselves and their placement on the surface.
    pub fn for_each_surface_voxel_in_voxel_ranges(
        &self,
        included_voxel_ranges: VoxelRanges,
        f: &mut impl FnMut([usize; 3], &Voxel, VoxelSurfacePlacement),
    ) {
        if included_voxel_ranges.iter().any(Range::is_empty) {
            return;
        }

        let included_chunk_ranges = included_voxel_ranges
            .clone()
            .map(chunk_range_encompassing_voxel_range);

        for chunk_i in included_chunk_ranges[0].clone() {
            for chunk_j in included_chunk_ranges[1].clone() {
                for chunk_k in included_chunk_ranges[2].clone() {
                    let chunk_indices = [chunk_i, chunk_j, chunk_k];
                    let chunk_idx = self.linear_chunk_idx(&chunk_indices);

                    // Only non-uniform chunks can have surface voxels
                    let VoxelChunk::NonUniform(chunk) = &self.chunks[chunk_idx] else {
                        continue;
                    };

                    let object_voxel_ranges_in_chunk =
                        chunk_indices.map(|index| index * CHUNK_SIZE..(index + 1) * CHUNK_SIZE);

                    let included_voxel_ranges_in_chunk: [_; 3] = array::from_fn(|dim| {
                        let range_in_chunk = &object_voxel_ranges_in_chunk[dim];
                        let included_range = &included_voxel_ranges[dim];
                        usize::max(range_in_chunk.start, included_range.start)
                            ..usize::min(range_in_chunk.end, included_range.end)
                    });

                    let voxels = chunk_voxels(&self.voxels, chunk.data_offset);

                    for i in included_voxel_ranges_in_chunk[0].clone() {
                        for j in included_voxel_ranges_in_chunk[1].clone() {
                            for k in included_voxel_ranges_in_chunk[2].clone() {
                                let voxel_idx =
                                    linear_voxel_idx_within_chunk_from_object_voxel_indices(
                                        i, j, k,
                                    );
                                let voxel = &voxels[voxel_idx];
                                if let Some(VoxelPlacement::Surface(placement)) = voxel.placement()
                                {
                                    f([i, j, k], voxel, placement);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// Finds all non-empty voxels whose center fall within the given sphere and
    /// calls the given closure with the voxel's indices, squared distance
    /// between the voxel center and the center of the sphere and a mutable
    /// reference to the voxel itself.
    ///
    /// The sphere should be specified in the model space of the voxel object,
    /// where the lower corner of the grid is at the origin and the cartesian
    /// axes are aligned with the grid.
    ///
    /// Since it is assumed that the given closure will modify the voxels, the
    /// adjacency information will be updated for all voxels within the sphere,
    /// and any chunk whose mesh data would be invalidated by changes to these
    /// voxels will be registered. The invalidated chunks can be obtained by
    /// calling [`Self::invalidated_mesh_chunk_indices`].
    ///
    /// Even though modifying the object will invalidate the connected region
    /// information, this method does not call
    /// [`Self::resolve_connected_regions_between_all_chunks`] to avoid
    /// duplicating work when this method is called multiple times. Make sure to
    /// call it once all modifications have been made.
    pub fn modify_voxels_within_sphere(
        &mut self,
        sphere: &Sphere<f64>,
        modify_voxel: &mut impl FnMut([usize; 3], f64, &mut Voxel),
    ) {
        let normalized_sphere = sphere.scaled(self.voxel_extent.recip());

        let touched_voxel_ranges =
            self.voxel_ranges_in_object_touching_aab(&normalized_sphere.compute_aabb());

        if touched_voxel_ranges.iter().any(Range::is_empty) {
            return;
        }

        let touched_chunk_ranges = touched_voxel_ranges
            .clone()
            .map(chunk_range_encompassing_voxel_range);

        let voxel_extent_squared = self.voxel_extent.powi(2);
        let normalized_sphere_radius_squared = normalized_sphere.radius_squared();

        let mut removed_chunks = false;

        for chunk_i in touched_chunk_ranges[0].clone() {
            for chunk_j in touched_chunk_ranges[1].clone() {
                for chunk_k in touched_chunk_ranges[2].clone() {
                    let chunk_indices = [chunk_i, chunk_j, chunk_k];
                    let chunk_idx = self.linear_chunk_idx(&chunk_indices);

                    let chunk = &mut self.chunks[chunk_idx];

                    let data_offset = match chunk {
                        VoxelChunk::Empty => {
                            continue;
                        }
                        VoxelChunk::Uniform(_) => {
                            chunk.convert_to_non_uniform_if_uniform(
                                &mut self.voxels,
                                &mut self.split_detector,
                            );
                            if let VoxelChunk::NonUniform(chunk) = chunk {
                                chunk.data_offset
                            } else {
                                unreachable!()
                            }
                        }
                        VoxelChunk::NonUniform(chunk) => chunk.data_offset,
                    };

                    let object_voxel_ranges_in_chunk =
                        chunk_indices.map(|index| index * CHUNK_SIZE..(index + 1) * CHUNK_SIZE);

                    let touched_voxel_ranges_in_chunk: [_; 3] = array::from_fn(|dim| {
                        let range_in_chunk = &object_voxel_ranges_in_chunk[dim];
                        let touched_range = &touched_voxel_ranges[dim];
                        usize::max(range_in_chunk.start, touched_range.start)
                            ..usize::min(range_in_chunk.end, touched_range.end)
                    });

                    let voxels = chunk_voxels_mut(&mut self.voxels, data_offset);

                    let mut chunk_touched = false;

                    for i in touched_voxel_ranges_in_chunk[0].clone() {
                        for j in touched_voxel_ranges_in_chunk[1].clone() {
                            for k in touched_voxel_ranges_in_chunk[2].clone() {
                                let normalized_voxel_center_position =
                                    normalized_voxel_center_position_from_object_voxel_indices(
                                        i, j, k,
                                    );

                                let normalized_distance_squared = na::distance_squared(
                                    normalized_sphere.center(),
                                    &normalized_voxel_center_position,
                                );

                                if normalized_distance_squared < normalized_sphere_radius_squared {
                                    let voxel_idx =
                                        linear_voxel_idx_within_chunk_from_object_voxel_indices(
                                            i, j, k,
                                        );
                                    let voxel = &mut voxels[voxel_idx];
                                    let distance_squared =
                                        normalized_distance_squared * voxel_extent_squared;
                                    modify_voxel([i, j, k], distance_squared, voxel);
                                    chunk_touched = true;
                                }
                            }
                        }
                    }

                    if chunk_touched {
                        Self::handle_chunk_voxels_modified(
                            &mut self.voxels,
                            &mut self.split_detector,
                            &self.occupied_chunk_ranges,
                            chunk,
                            chunk_indices,
                            chunk_idx,
                            object_voxel_ranges_in_chunk,
                            touched_voxel_ranges_in_chunk,
                            &mut self.invalidated_mesh_chunk_indices,
                            &mut removed_chunks,
                        );
                    }
                }
            }
        }

        if removed_chunks {
            self.update_occupied_ranges();
        }

        self.update_upper_boundary_adjacencies_for_chunks_in_ranges(
            touched_chunk_ranges.map(|range| range.start.saturating_sub(1)..range.end),
        );
    }

    /// Finds all non-empty voxels whose center fall within the given capsule
    /// and calls the given closure with the voxel's indices, squared minimum
    /// distance between the voxel center and the line segment representing the
    /// central axis of the capsule's cylinder and a mutable reference to
    /// the voxel itself.
    ///
    /// The capsule should be specified in the model space of the voxel object,
    /// where the lower corner of the grid is at the origin and the cartesian
    /// axes are aligned with the grid.
    ///
    /// Since it is assumed that the given closure will modify the voxels, the
    /// adjacency information will be updated for all voxels within the capsule,
    /// and any chunk whose mesh data would be invalidated by changes to these
    /// voxels will be registered. The invalidated chunks can be obtained by
    /// calling [`Self::invalidated_mesh_chunk_indices`].
    ///
    /// Even though modifying the object will invalidate the connected region
    /// information, this method does not call
    /// [`Self::resolve_connected_regions_between_all_chunks`] to avoid
    /// duplicating work when this method is called multiple times. Make sure to
    /// call it once all modifications have been made.
    pub fn modify_voxels_within_capsule(
        &mut self,
        capsule: &Capsule<f64>,
        modify_voxel: &mut impl FnMut([usize; 3], f64, &mut Voxel),
    ) {
        let normalized_capsule = capsule.scaled(self.voxel_extent.recip());

        let touched_voxel_ranges =
            self.voxel_ranges_in_object_touching_aab(&normalized_capsule.compute_aabb());

        if touched_voxel_ranges.iter().any(Range::is_empty) {
            return;
        }

        let touched_chunk_ranges = touched_voxel_ranges
            .clone()
            .map(chunk_range_encompassing_voxel_range);

        let voxel_extent_squared = self.voxel_extent.powi(2);

        let containment_tester = normalized_capsule.create_point_containment_tester();

        let mut removed_chunks = false;

        for chunk_i in touched_chunk_ranges[0].clone() {
            for chunk_j in touched_chunk_ranges[1].clone() {
                for chunk_k in touched_chunk_ranges[2].clone() {
                    let chunk_indices = [chunk_i, chunk_j, chunk_k];
                    let chunk_idx = self.linear_chunk_idx(&chunk_indices);

                    let object_voxel_ranges_in_chunk =
                        chunk_indices.map(|index| index * CHUNK_SIZE..(index + 1) * CHUNK_SIZE);

                    let Some(normalized_chunk_subcapsule) = normalized_capsule
                        .with_segment_clamped_to_aab(&normalized_chunk_aabb_from_chunk_indices(
                            chunk_i, chunk_j, chunk_k,
                        ))
                    else {
                        continue;
                    };

                    let touched_voxel_ranges_in_chunk = voxel_ranges_touching_aab(
                        object_voxel_ranges_in_chunk.clone(),
                        &normalized_chunk_subcapsule.compute_aabb(),
                    );

                    if touched_voxel_ranges_in_chunk.iter().any(Range::is_empty) {
                        continue;
                    }

                    let chunk = &mut self.chunks[chunk_idx];

                    let data_offset = match chunk {
                        VoxelChunk::Empty => {
                            continue;
                        }
                        VoxelChunk::Uniform(_) => {
                            chunk.convert_to_non_uniform_if_uniform(
                                &mut self.voxels,
                                &mut self.split_detector,
                            );
                            if let VoxelChunk::NonUniform(chunk) = chunk {
                                chunk.data_offset
                            } else {
                                unreachable!()
                            }
                        }
                        VoxelChunk::NonUniform(chunk) => chunk.data_offset,
                    };

                    let voxels = chunk_voxels_mut(&mut self.voxels, data_offset);

                    let mut chunk_touched = false;

                    for i in touched_voxel_ranges_in_chunk[0].clone() {
                        for j in touched_voxel_ranges_in_chunk[1].clone() {
                            for k in touched_voxel_ranges_in_chunk[2].clone() {
                                let normalized_voxel_center_position =
                                    normalized_voxel_center_position_from_object_voxel_indices(
                                        i, j, k,
                                    );

                                if let Some(shortest_normalized_distance_squared) = containment_tester
                                    .shortest_squared_distance_from_point_to_segment_if_contained(
                                        &normalized_voxel_center_position,
                                    )
                                {
                                    let voxel_idx =
                                        linear_voxel_idx_within_chunk_from_object_voxel_indices(
                                            i, j, k,
                                        );
                                    let voxel = &mut voxels[voxel_idx];
                                    let shortest_distance_squared = shortest_normalized_distance_squared * voxel_extent_squared;
                                    modify_voxel([i, j, k], shortest_distance_squared, voxel);
                                    chunk_touched = true;
                                }
                            }
                        }
                    }

                    if chunk_touched {
                        Self::handle_chunk_voxels_modified(
                            &mut self.voxels,
                            &mut self.split_detector,
                            &self.occupied_chunk_ranges,
                            chunk,
                            chunk_indices,
                            chunk_idx,
                            object_voxel_ranges_in_chunk,
                            touched_voxel_ranges_in_chunk,
                            &mut self.invalidated_mesh_chunk_indices,
                            &mut removed_chunks,
                        );
                    }
                }
            }
        }

        if removed_chunks {
            self.update_occupied_ranges();
        }

        self.update_upper_boundary_adjacencies_for_chunks_in_ranges(
            touched_chunk_ranges.map(|range| range.start.saturating_sub(1)..range.end),
        );
    }

    /// Returns the object space position of the center of the voxel at the
    /// given object voxel indices.
    pub fn voxel_center_position_from_object_voxel_indices(
        &self,
        i: usize,
        j: usize,
        k: usize,
    ) -> Point3<f64> {
        voxel_center_position_from_object_voxel_indices(self.voxel_extent, i, j, k)
    }

    /// Returns the object space axis aligned bounding box of the voxel at the
    /// given object voxel indices.
    pub fn voxel_aabb_from_object_voxel_indices(
        &self,
        i: usize,
        j: usize,
        k: usize,
    ) -> AxisAlignedBox<f64> {
        voxel_aabb_from_object_voxel_indices(self.voxel_extent, i, j, k)
    }

    fn handle_chunk_voxels_modified(
        voxels: &mut [Voxel],
        split_detector: &mut SplitDetector,
        occupied_chunk_ranges: &ChunkRanges,
        chunk: &mut VoxelChunk,
        chunk_indices: [usize; 3],
        chunk_idx: usize,
        object_voxel_ranges_in_chunk: VoxelRanges,
        touched_voxel_ranges_in_chunk: VoxelRanges,
        invalidated_mesh_chunk_indices: &mut HashSet<[usize; 3]>,
        removed_chunks: &mut bool,
    ) {
        // We need to update the face distributions and internal adjacencies of the
        // touched chunk
        let non_empty_voxel_count = if let VoxelChunk::NonUniform(chunk) = chunk {
            chunk.update_face_distributions_and_internal_adjacencies_and_count_non_empty_voxels(
                voxels,
            )
        } else {
            unreachable!()
        };

        // Mark the chunk as empty if no non-empty voxels remain in the chunk
        if non_empty_voxel_count == 0 {
            *chunk = VoxelChunk::Empty;
            *removed_chunks = true;
        }

        // If the chunk has not been emptied, we also need to update the local
        // connected regions
        if let VoxelChunk::NonUniform(chunk) = chunk {
            split_detector.update_local_connected_regions_for_chunk(
                voxels,
                chunk,
                chunk_idx as u32,
            );
        }

        // The mesh of the touched chunk is invalidated
        invalidated_mesh_chunk_indices.insert(chunk_indices);

        for dim in 0..3 {
            // The meshes of adjacent chunks are invalidated if any voxel within 2
            // voxels of the relevant boundary was touched (that is, a boundary
            // voxel or a voxel adjacent to a boundary voxel)

            let voxel_range = &object_voxel_ranges_in_chunk[dim];
            let touched_voxel_range = &touched_voxel_ranges_in_chunk[dim];

            if chunk_indices[dim] > occupied_chunk_ranges[dim].start
                && touched_voxel_range.start - voxel_range.start < 2
            {
                let mut adjacent_chunk_indices = chunk_indices;
                adjacent_chunk_indices[dim] -= 1;
                invalidated_mesh_chunk_indices.insert(adjacent_chunk_indices);
            }

            if chunk_indices[dim] < occupied_chunk_ranges[dim].end - 1
                && voxel_range.end - touched_voxel_range.end < 2
            {
                let mut adjacent_chunk_indices = chunk_indices;
                adjacent_chunk_indices[dim] += 1;
                invalidated_mesh_chunk_indices.insert(adjacent_chunk_indices);
            }
        }
    }

    #[cfg(any(test, feature = "fuzzing"))]
    fn for_each_surface_voxel_touching_negative_halfspace_of_plane_brute_force(
        &self,
        plane: &Plane<f64>,
        f: &mut impl FnMut([usize; 3], &Voxel),
    ) {
        for i in self.occupied_voxel_ranges[0].clone() {
            for j in self.occupied_voxel_ranges[1].clone() {
                for k in self.occupied_voxel_ranges[2].clone() {
                    let voxel_aabb = self.voxel_aabb_from_object_voxel_indices(i, j, k);
                    if voxel_aabb
                        .all_corners()
                        .iter()
                        .any(|corner| !plane.point_lies_in_positive_halfspace(corner))
                        && let Some(voxel) = self.get_voxel(i, j, k)
                        && let Some(VoxelPlacement::Surface(_)) = voxel.placement()
                    {
                        f([i, j, k], voxel);
                    }
                }
            }
        }
    }

    #[cfg(any(test, feature = "fuzzing"))]
    fn for_each_surface_voxel_touching_sphere_brute_force(
        &self,
        sphere: &Sphere<f64>,
        f: &mut impl FnMut([usize; 3], &Voxel),
    ) {
        for i in self.occupied_voxel_ranges[0].clone() {
            for j in self.occupied_voxel_ranges[1].clone() {
                for k in self.occupied_voxel_ranges[2].clone() {
                    let voxel_center_position =
                        self.voxel_center_position_from_object_voxel_indices(i, j, k);
                    if na::distance(&voxel_center_position, sphere.center())
                        <= 0.5 * self.voxel_extent + sphere.radius()
                        && let Some(voxel) = self.get_voxel(i, j, k)
                        && let Some(VoxelPlacement::Surface(_)) = voxel.placement()
                    {
                        f([i, j, k], voxel);
                    }
                }
            }
        }
    }

    #[cfg(any(test, feature = "fuzzing"))]
    fn for_each_non_empty_voxel_in_sphere_brute_force(
        &self,
        sphere: &Sphere<f64>,
        f: &mut impl FnMut([usize; 3], &Voxel),
    ) {
        for i in self.occupied_voxel_ranges[0].clone() {
            for j in self.occupied_voxel_ranges[1].clone() {
                for k in self.occupied_voxel_ranges[2].clone() {
                    let voxel_center_position =
                        self.voxel_center_position_from_object_voxel_indices(i, j, k);
                    if sphere.contains_point(&voxel_center_position)
                        && let Some(voxel) = self.get_voxel(i, j, k)
                    {
                        f([i, j, k], voxel);
                    }
                }
            }
        }
    }

    #[cfg(any(test, feature = "fuzzing"))]
    fn for_each_non_empty_voxel_in_capsule_brute_force(
        &self,
        capsule: &Capsule<f64>,
        f: &mut impl FnMut([usize; 3], &Voxel),
    ) {
        let containment_tester = capsule.create_point_containment_tester();
        for i in self.occupied_voxel_ranges[0].clone() {
            for j in self.occupied_voxel_ranges[1].clone() {
                for k in self.occupied_voxel_ranges[2].clone() {
                    let voxel_center_position =
                        self.voxel_center_position_from_object_voxel_indices(i, j, k);
                    if containment_tester.contains_point(&voxel_center_position)
                        && let Some(voxel) = self.get_voxel(i, j, k)
                    {
                        f([i, j, k], voxel);
                    }
                }
            }
        }
    }

    /// The AAB should be in normalized voxel object space (where voxel extent
    /// is 1.0).
    fn voxel_ranges_in_object_touching_aab(
        &self,
        normalized_aab: &AxisAlignedBox<f64>,
    ) -> VoxelRanges {
        voxel_ranges_touching_aab(self.occupied_voxel_ranges.clone(), normalized_aab)
    }

    /// The plane should be in normalized voxel object space (where voxel extent
    /// is 1.0).
    fn voxel_ranges_in_object_within_plane(&self, normalized_plane: &Plane<f64>) -> VoxelRanges {
        voxel_ranges_within_plane(self.occupied_voxel_ranges.clone(), normalized_plane)
    }

    pub fn determine_voxel_ranges_encompassing_intersection(
        object_a: &Self,
        object_b: &Self,
        transform_from_b_to_a: &Isometry3<f64>,
    ) -> Option<(VoxelRanges, VoxelRanges)> {
        let object_a_aabb = normalized_aabb_from_voxel_ranges(&object_a.occupied_voxel_ranges)
            .scaled(object_a.voxel_extent);
        let object_b_aabb = normalized_aabb_from_voxel_ranges(&object_b.occupied_voxel_ranges)
            .scaled(object_b.voxel_extent);

        let object_b_obb = OrientedBox::from_axis_aligned_box(&object_b_aabb);

        let object_b_obb_in_a = object_b_obb.translated_and_rotated(transform_from_b_to_a);

        let (intersection_aabb_in_a, intersection_aabb_in_b_relative_to_center) =
            impact_geometry::compute_box_intersection_bounds(&object_a_aabb, &object_b_obb_in_a)?;

        // `compute_box_intersection_bounds` returns the second bounds relative
        // to the center of box B, but we need it relative to the lower corner
        let intersection_aabb_in_b =
            intersection_aabb_in_b_relative_to_center.translated(&object_b_obb.center().coords);

        let intersection_voxel_ranges_in_a = voxel_ranges_touching_aab(
            object_a.occupied_voxel_ranges.clone(),
            &intersection_aabb_in_a.scaled(object_a.voxel_extent.recip()),
        );

        let intersection_voxel_ranges_in_b = voxel_ranges_touching_aab(
            object_b.occupied_voxel_ranges.clone(),
            &intersection_aabb_in_b.scaled(object_b.voxel_extent.recip()),
        );

        Some((
            intersection_voxel_ranges_in_a,
            intersection_voxel_ranges_in_b,
        ))
    }
}

fn chunk_range_encompassing_voxel_range(voxel_range: Range<usize>) -> Range<usize> {
    let start = voxel_range.start / CHUNK_SIZE;
    let end = voxel_range.end.div_ceil(CHUNK_SIZE);
    start..end
}

/// The plane should be in normalized voxel object space (where voxel extent
/// is 1.0).
fn voxel_ranges_within_plane(
    max_voxel_ranges: VoxelRanges,
    normalized_plane: &Plane<f64>,
) -> VoxelRanges {
    let normalized_aabb = normalized_aabb_from_voxel_ranges(&max_voxel_ranges);

    let normalized_aabb_within_plane =
        normalized_aabb.projected_onto_negative_halfspace(normalized_plane);

    voxel_ranges_touching_aab(max_voxel_ranges, &normalized_aabb_within_plane)
}

/// The AAB should be in normalized voxel object space (where voxel extent is
/// 1.0).
fn voxel_ranges_touching_aab(
    max_voxel_ranges: VoxelRanges,
    normalized_aab: &AxisAlignedBox<f64>,
) -> VoxelRanges {
    let lower_corner = normalized_aab.lower_corner();
    let upper_corner = normalized_aab.upper_corner();

    let mut touched_voxel_ranges = max_voxel_ranges;

    for dim in 0..3 {
        let range = &mut touched_voxel_ranges[dim];
        range.start = range.start.max(lower_corner[dim].floor().max(0.0) as usize);
        range.end = range.end.min(upper_corner[dim].ceil() as usize);
    }

    touched_voxel_ranges
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

fn normalized_voxel_center_position_from_object_voxel_indices(
    i: usize,
    j: usize,
    k: usize,
) -> Point3<f64> {
    point![(i as f64 + 0.5), (j as f64 + 0.5), (k as f64 + 0.5)]
}

fn normalized_chunk_aabb_from_chunk_indices(
    chunk_i: usize,
    chunk_j: usize,
    chunk_k: usize,
) -> AxisAlignedBox<f64> {
    AxisAlignedBox::new(
        point![
            (chunk_i * CHUNK_SIZE) as f64,
            (chunk_j * CHUNK_SIZE) as f64,
            (chunk_k * CHUNK_SIZE) as f64
        ],
        point![
            ((chunk_i + 1) * CHUNK_SIZE) as f64,
            ((chunk_j + 1) * CHUNK_SIZE) as f64,
            ((chunk_k + 1) * CHUNK_SIZE) as f64
        ],
    )
}

fn voxel_aabb_from_object_voxel_indices(
    voxel_extent: f64,
    i: usize,
    j: usize,
    k: usize,
) -> AxisAlignedBox<f64> {
    AxisAlignedBox::new(
        point![
            i as f64 * voxel_extent,
            j as f64 * voxel_extent,
            k as f64 * voxel_extent
        ],
        point![
            (i as f64 + 1.0) * voxel_extent,
            (j as f64 + 1.0) * voxel_extent,
            (k as f64 + 1.0) * voxel_extent
        ],
    )
}

fn normalized_aabb_from_voxel_ranges(voxel_ranges: &VoxelRanges) -> AxisAlignedBox<f64> {
    let lower_corner = point![
        voxel_ranges[0].start as f64,
        voxel_ranges[1].start as f64,
        voxel_ranges[2].start as f64
    ];

    let upper_corner = point![
        voxel_ranges[0].end as f64,
        voxel_ranges[1].end as f64,
        voxel_ranges[2].end as f64
    ];

    AxisAlignedBox::new(lower_corner, upper_corner)
}

#[cfg(feature = "fuzzing")]
pub mod fuzzing {
    use super::*;
    use crate::{
        chunks::inertia::VoxelObjectInertialPropertyManager, generation::SDFVoxelGenerator,
    };
    use approx::abs_diff_eq;
    use arbitrary::{Arbitrary, Result, Unstructured};
    use nalgebra::{UnitVector3, vector};
    use std::mem;

    #[derive(Clone, Debug)]
    pub struct ArbitraryPlane(Plane<f64>);

    #[derive(Clone, Debug)]
    pub struct ArbitrarySphere(Sphere<f64>);

    #[derive(Clone, Debug)]
    pub struct ArbitraryCapsule(Capsule<f64>);

    impl Arbitrary<'_> for ArbitraryPlane {
        fn arbitrary(u: &mut Unstructured<'_>) -> Result<Self> {
            let displacement = 1e3 * (2.0 * arbitrary_norm_f64(u)? - 1.0);
            let nx = 2.0 * arbitrary_norm_f64(u)? - 1.0;
            let ny = 2.0 * arbitrary_norm_f64(u)? - 1.0;
            let mut nz = 2.0 * arbitrary_norm_f64(u)? - 1.0;
            if abs_diff_eq!(nx, 0.0) && abs_diff_eq!(ny, 0.0) && abs_diff_eq!(nz, 0.0) {
                nz = 1e-3;
            }
            Ok(Self(Plane::new(
                UnitVector3::new_normalize(vector![nx, ny, nz]),
                displacement,
            )))
        }

        fn size_hint(_depth: usize) -> (usize, Option<usize>) {
            let size = 5 * mem::size_of::<i32>();
            (size, Some(size))
        }
    }

    impl Arbitrary<'_> for ArbitrarySphere {
        fn arbitrary(u: &mut Unstructured<'_>) -> Result<Self> {
            let radius = u.arbitrary_len::<usize>()?.min(1000) as f64 + arbitrary_norm_f64(u)?;
            let x = 1e3 * arbitrary_norm_f64(u)?;
            let y = 1e3 * arbitrary_norm_f64(u)?;
            let z = 1e3 * arbitrary_norm_f64(u)?;
            Ok(Self(Sphere::new(point![x, y, z], radius)))
        }

        fn size_hint(_depth: usize) -> (usize, Option<usize>) {
            let size = 5 * mem::size_of::<i32>();
            (size, Some(size))
        }
    }

    impl Arbitrary<'_> for ArbitraryCapsule {
        fn arbitrary(u: &mut Unstructured<'_>) -> Result<Self> {
            let start_x = 1e3 * arbitrary_norm_f64(u)?;
            let start_y = 1e3 * arbitrary_norm_f64(u)?;
            let start_z = 1e3 * arbitrary_norm_f64(u)?;
            let segment_start = point![start_x, start_y, start_z];

            let dir_x = 2.0 * arbitrary_norm_f64(u)? - 1.0;
            let dir_y = 2.0 * arbitrary_norm_f64(u)? - 1.0;
            let dir_z = 2.0 * arbitrary_norm_f64(u)? - 1.0;
            let length = u.arbitrary_len::<usize>()?.min(1000) as f64 + arbitrary_norm_f64(u)?;
            let segment_vector = vector![dir_x, dir_y, dir_z].normalize() * length;

            let radius = u.arbitrary_len::<usize>()?.min(1000) as f64 + arbitrary_norm_f64(u)?;

            Ok(Self(Capsule::new(segment_start, segment_vector, radius)))
        }

        fn size_hint(_depth: usize) -> (usize, Option<usize>) {
            let size = 10 * mem::size_of::<i32>();
            (size, Some(size))
        }
    }

    pub fn fuzz_test_obtaining_surface_voxels_maybe_intersecting_negative_halfspace_of_plane(
        (generator, plane): (SDFVoxelGenerator, ArbitraryPlane),
    ) {
        let object = ChunkedVoxelObject::generate(&generator);
        let mut indices_of_touched_voxels = HashSet::default();

        object.for_each_surface_voxel_maybe_intersecting_negative_halfspace_of_plane(
            &plane.0,
            &mut |indices, voxel, placement| {
                assert!(!voxel.is_empty());
                assert!(matches!(
                    voxel.placement(),
                    Some(VoxelPlacement::Surface(pl)) if pl == placement
                ));
                let was_absent = indices_of_touched_voxels.insert(indices);
                assert!(
                    was_absent,
                    "Voxel in negative halfspace of plane found twice: {indices:?}"
                );
            },
        );

        object.for_each_surface_voxel_touching_negative_halfspace_of_plane_brute_force(
            &plane.0,
            &mut |indices, _| {
                let was_present = indices_of_touched_voxels.remove(&indices);
                assert!(
                    was_present,
                    "Voxel in negative halfspace of plane was not found: {indices:?}"
                );
            },
        );
    }

    pub fn fuzz_test_obtaining_surface_voxels_maybe_intersecting_sphere(
        (generator, sphere): (SDFVoxelGenerator, ArbitrarySphere),
    ) {
        let object = ChunkedVoxelObject::generate(&generator);
        let mut indices_of_touched_voxels = HashSet::default();

        object.for_each_surface_voxel_maybe_intersecting_sphere(
            &sphere.0,
            &mut |indices, voxel, placement| {
                assert!(!voxel.is_empty());
                assert!(matches!(
                    voxel.placement(),
                    Some(VoxelPlacement::Surface(pl)) if pl == placement
                ));
                let was_absent = indices_of_touched_voxels.insert(indices);
                assert!(was_absent, "Voxel in sphere found twice: {indices:?}");
            },
        );

        object.for_each_surface_voxel_touching_sphere_brute_force(&sphere.0, &mut |indices, _| {
            let was_present = indices_of_touched_voxels.remove(&indices);
            assert!(was_present, "Voxel in sphere was not found: {indices:?}");
        });
    }

    pub fn fuzz_test_obtaining_voxels_within_sphere(
        (generator, sphere): (SDFVoxelGenerator, ArbitrarySphere),
    ) {
        let mut object = ChunkedVoxelObject::generate(&generator);
        let mut indices_of_inside_voxels = HashSet::default();

        object.modify_voxels_within_sphere(&sphere.0, &mut |indices, _, voxel| {
            if !voxel.is_empty() {
                let was_absent = indices_of_inside_voxels.insert(indices);
                assert!(was_absent, "Voxel in sphere found twice: {indices:?}");
            }
        });
        object.resolve_connected_regions_between_all_chunks();

        object.for_each_non_empty_voxel_in_sphere_brute_force(&sphere.0, &mut |indices, _| {
            let was_present = indices_of_inside_voxels.remove(&indices);
            assert!(was_present, "Voxel in sphere was not found: {indices:?}");
        });

        assert!(
            indices_of_inside_voxels.is_empty(),
            "Found voxels not inside sphere: {:?}",
            &indices_of_inside_voxels
        );

        object.validate_region_count();
    }

    pub fn fuzz_test_obtaining_voxels_within_capsule(
        (generator, capsule): (SDFVoxelGenerator, ArbitraryCapsule),
    ) {
        let mut object = ChunkedVoxelObject::generate(&generator);
        let mut indices_of_inside_voxels = HashSet::default();

        object.modify_voxels_within_capsule(&capsule.0, &mut |indices, _, voxel| {
            if !voxel.is_empty() {
                let was_absent = indices_of_inside_voxels.insert(indices);
                assert!(was_absent, "Voxel in capsule found twice: {indices:?}");
            }
        });
        object.resolve_connected_regions_between_all_chunks();

        object.for_each_non_empty_voxel_in_capsule_brute_force(&capsule.0, &mut |indices, _| {
            let was_present = indices_of_inside_voxels.remove(&indices);
            assert!(was_present, "Voxel in capsule was not found: {indices:?}");
        });

        assert!(
            indices_of_inside_voxels.is_empty(),
            "Found voxels not inside capsule: {:?}",
            &indices_of_inside_voxels
        );

        object.validate_region_count();
    }

    pub fn fuzz_test_absorbing_voxels_within_sphere(
        (generator, sphere): (SDFVoxelGenerator, ArbitrarySphere),
    ) {
        let mut object = ChunkedVoxelObject::generate(&generator);
        let voxel_type_densities = vec![1.0; 256];

        let mut inertial_property_manager =
            VoxelObjectInertialPropertyManager::initialized_from(&object, &voxel_type_densities);

        let mut inertial_property_updater =
            inertial_property_manager.begin_update(object.voxel_extent(), &voxel_type_densities);

        object.modify_voxels_within_sphere(
            &sphere.0,
            &mut |object_voxel_indices, squared_distance, voxel| {
                let was_empty = voxel.is_empty();

                let signed_distance_delta =
                    3.0 * (1.0 - squared_distance * sphere.0.radius_squared().recip());

                voxel.increase_signed_distance(signed_distance_delta as f32, &mut |voxel| {
                    if !was_empty {
                        inertial_property_updater.remove_voxel(&object_voxel_indices, *voxel);
                    }
                });
            },
        );

        if !object.is_effectively_empty() {
            object.resolve_connected_regions_between_all_chunks();

            object.validate_adjacencies();
            object.validate_chunk_obscuredness();
            object.validate_sdf();
            object.validate_region_count();

            inertial_property_manager.validate_for_object(&object, &voxel_type_densities);
        }
    }

    pub fn fuzz_test_absorbing_voxels_within_capsule(
        (generator, capsule): (SDFVoxelGenerator, ArbitraryCapsule),
    ) {
        let mut object = ChunkedVoxelObject::generate(&generator);
        let voxel_type_densities = vec![1.0; 256];

        let mut inertial_property_manager =
            VoxelObjectInertialPropertyManager::initialized_from(&object, &voxel_type_densities);

        let mut inertial_property_updater =
            inertial_property_manager.begin_update(object.voxel_extent(), &voxel_type_densities);

        object.modify_voxels_within_capsule(
            &capsule.0,
            &mut |object_voxel_indices, squared_distance, voxel| {
                let was_empty = voxel.is_empty();

                let signed_distance_delta =
                    3.0 * (1.0 - squared_distance * capsule.0.radius().powi(2).recip());

                voxel.increase_signed_distance(signed_distance_delta as f32, &mut |voxel| {
                    if !was_empty {
                        inertial_property_updater.remove_voxel(&object_voxel_indices, *voxel);
                    }
                });
            },
        );

        if !object.is_effectively_empty() {
            object.resolve_connected_regions_between_all_chunks();

            object.validate_adjacencies();
            object.validate_chunk_obscuredness();
            object.validate_sdf();
            object.validate_region_count();

            inertial_property_manager.validate_for_object(&object, &voxel_type_densities);
        }
    }

    fn arbitrary_norm_f64(u: &mut Unstructured<'_>) -> Result<f64> {
        Ok(f64::from(u.int_in_range(0..=1000000)?) / 1000000.0)
    }
}

#[cfg(not(miri))]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        generation::{SDFVoxelGenerator, sdf::SphereSDF, voxel_type::SameVoxelTypeGenerator},
        voxel_types::VoxelType,
    };
    use nalgebra::{UnitVector3, vector};

    #[test]
    fn finding_surface_voxels_intersecting_negative_halfspace_of_plane_finds_correct_voxels() {
        let object_radius = 10.0;
        let plane_displacement = 0.8 * object_radius;

        let generator = SDFVoxelGenerator::new(
            0.5,
            SphereSDF::new(object_radius as f32).into(),
            SameVoxelTypeGenerator::new(VoxelType::default()).into(),
        );
        let object = ChunkedVoxelObject::generate(&generator);

        let plane = Plane::new(
            UnitVector3::new_normalize(vector![1.0, 1.0, 1.0]),
            plane_displacement,
        );

        let mut indices_of_touched_voxels = HashSet::default();

        object.for_each_surface_voxel_maybe_intersecting_negative_halfspace_of_plane(
            &plane,
            &mut |indices, voxel, placement| {
                assert!(!voxel.is_empty());
                assert!(matches!(
                    voxel.placement(),
                    Some(VoxelPlacement::Surface(pl)) if pl == placement
                ));
                let was_absent = indices_of_touched_voxels.insert(indices);
                assert!(
                    was_absent,
                    "Voxel in negative plane halfspace found twice: {indices:?}"
                );
            },
        );

        object.for_each_surface_voxel_touching_negative_halfspace_of_plane_brute_force(
            &plane,
            &mut |indices, _| {
                let was_present = indices_of_touched_voxels.remove(&indices);
                assert!(
                    was_present,
                    "Voxel in negative plane halfspace was not found: {indices:?}"
                );
            },
        );
    }

    #[test]
    fn finding_surface_voxels_intersecting_sphere_finds_correct_voxels() {
        let object_radius = 10.0;
        let sphere_radius = 0.6 * object_radius;

        let generator = SDFVoxelGenerator::new(
            0.5,
            SphereSDF::new(object_radius as f32).into(),
            SameVoxelTypeGenerator::new(VoxelType::default()).into(),
        );
        let object = ChunkedVoxelObject::generate(&generator);

        let sphere = Sphere::new(
            object.compute_aabb::<f64>().center()
                - UnitVector3::new_normalize(vector![1.0, 1.0, 1.0]).scale(object_radius),
            sphere_radius,
        );

        let mut indices_of_touched_voxels = HashSet::default();

        object.for_each_surface_voxel_maybe_intersecting_sphere(
            &sphere,
            &mut |indices, voxel, placement| {
                assert!(!voxel.is_empty());
                assert!(matches!(
                    voxel.placement(),
                    Some(VoxelPlacement::Surface(pl)) if pl == placement
                ));
                let was_absent = indices_of_touched_voxels.insert(indices);
                assert!(was_absent, "Voxel in sphere found twice: {indices:?}");
            },
        );

        object.for_each_surface_voxel_touching_sphere_brute_force(&sphere, &mut |indices, _| {
            let was_present = indices_of_touched_voxels.remove(&indices);
            assert!(was_present, "Voxel in sphere was not found: {indices:?}");
        });
    }

    #[test]
    fn modifying_voxels_within_sphere_finds_correct_voxels() {
        let object_radius = 10.0;
        let sphere_radius = 0.4 * object_radius;

        let generator = SDFVoxelGenerator::new(
            0.5,
            SphereSDF::new(object_radius as f32).into(),
            SameVoxelTypeGenerator::new(VoxelType::default()).into(),
        );
        let mut object = ChunkedVoxelObject::generate(&generator);

        let sphere = Sphere::new(
            object.compute_aabb::<f64>().center()
                - UnitVector3::new_normalize(vector![1.0, 1.0, 1.0]).scale(object_radius),
            sphere_radius,
        );

        let mut indices_of_inside_voxels = HashSet::default();

        object.modify_voxels_within_sphere(&sphere, &mut |indices, _, voxel| {
            if !voxel.is_empty() {
                let was_absent = indices_of_inside_voxels.insert(indices);
                assert!(was_absent, "Voxel in sphere found twice: {indices:?}");
            }
        });

        object.for_each_non_empty_voxel_in_sphere_brute_force(&sphere, &mut |indices, _| {
            let was_present = indices_of_inside_voxels.remove(&indices);
            assert!(was_present, "Voxel in sphere was not found: {indices:?}");
        });

        assert!(
            indices_of_inside_voxels.is_empty(),
            "Found voxels not inside sphere: {:?}",
            &indices_of_inside_voxels
        );
    }

    #[test]
    fn modifying_voxels_within_capsule_finds_correct_voxels() {
        let object_radius = 10.0;
        let capsule_direction = UnitVector3::new_normalize(-vector![1.0, 1.0, 1.0]);
        let capsule_vector = capsule_direction.scale(10.0);
        let capsule_radius = 0.4 * object_radius;

        let generator = SDFVoxelGenerator::new(
            0.5,
            SphereSDF::new(object_radius as f32).into(),
            SameVoxelTypeGenerator::new(VoxelType::default()).into(),
        );
        let mut object = ChunkedVoxelObject::generate(&generator);

        let capsule = Capsule::new(
            object.compute_aabb::<f64>().center() - capsule_direction.scale(-object_radius),
            capsule_vector,
            capsule_radius,
        );

        let mut indices_of_inside_voxels = HashSet::default();

        object.modify_voxels_within_capsule(&capsule, &mut |indices, _, voxel| {
            if !voxel.is_empty() {
                let was_absent = indices_of_inside_voxels.insert(indices);
                assert!(was_absent, "Voxel in capsule found twice: {indices:?}");
            }
        });

        object.for_each_non_empty_voxel_in_capsule_brute_force(&capsule, &mut |indices, _| {
            let was_present = indices_of_inside_voxels.remove(&indices);
            assert!(was_present, "Voxel in capsule was not found: {indices:?}");
        });

        assert!(
            indices_of_inside_voxels.is_empty(),
            "Found voxels not inside capsule: {:?}",
            &indices_of_inside_voxels
        );
    }
}
