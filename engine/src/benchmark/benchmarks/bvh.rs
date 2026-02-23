//! Benchmarks for bounding volume hierarchies.

use impact_geometry::{AxisAlignedBox, AxisAlignedBoxC};
use impact_intersection::bounding_volume::{
    BoundingVolumeID,
    hierarchy::{BVHBuildMethod, BoundingVolumeHierarchy},
};
use impact_math::{hash::Hash32, point::Point3C};
use impact_profiling::benchmark::Benchmarker;
use std::hint::black_box;

const N_PRIMITIVES: usize = 25000;
const N_QUERIES: usize = 1000;

pub fn build_naive_bottom_up(benchmarker: impl Benchmarker) {
    let mut bvh = BoundingVolumeHierarchy::new_with_build_method(BVHBuildMethod::NaiveBottomUp);
    add_primitive_volumes(&mut bvh, stratified_random_aabbs(N_PRIMITIVES, 1.0));
    benchmarker.benchmark(&mut || bvh.build());
}

pub fn build_fast_bottom_up(benchmarker: impl Benchmarker) {
    let mut bvh = BoundingVolumeHierarchy::new_with_build_method(BVHBuildMethod::FastBottomUp);
    add_primitive_volumes(&mut bvh, stratified_random_aabbs(N_PRIMITIVES, 1.0));
    benchmarker.benchmark(&mut || bvh.build());
}

pub fn query_many_external_intersections(benchmarker: impl Benchmarker) {
    let mut bvh = BoundingVolumeHierarchy::new();
    add_primitive_volumes(&mut bvh, stratified_random_aabbs(N_PRIMITIVES, 1.0));
    bvh.build();
    let queries = generate_query_aabbs(&bvh, N_QUERIES);
    benchmarker.benchmark(&mut || {
        for query in &queries {
            bvh.for_each_bounding_volume_in_axis_aligned_box(query, |id| {
                black_box(id);
            });
        }
    });
}

pub fn query_all_internal_intersections(benchmarker: impl Benchmarker) {
    let mut bvh = BoundingVolumeHierarchy::new();
    add_primitive_volumes(&mut bvh, stratified_random_aabbs(N_PRIMITIVES, 2.0));
    bvh.build();
    benchmarker.benchmark(&mut || {
        bvh.for_each_intersecting_bounding_volume_pair(|id_a, id_b| {
            black_box((id_a, id_b));
        });
    });
}

pub fn query_with_brute_force_many_external_intersections(benchmarker: impl Benchmarker) {
    let mut bvh = BoundingVolumeHierarchy::new();
    add_primitive_volumes(&mut bvh, stratified_random_aabbs(N_PRIMITIVES, 1.0));
    bvh.build();
    let queries = generate_query_aabbs(&bvh, N_QUERIES);
    benchmarker.benchmark(&mut || {
        for query in &queries {
            bvh.for_each_bounding_volume_in_axis_aligned_box_brute_force(query, |id| {
                black_box(id);
            });
        }
    });
}

pub fn query_with_brute_force_all_internal_intersections(benchmarker: impl Benchmarker) {
    let mut bvh = BoundingVolumeHierarchy::new();
    add_primitive_volumes(&mut bvh, stratified_random_aabbs(N_PRIMITIVES, 2.0));
    bvh.build();
    benchmarker.benchmark(&mut || {
        bvh.for_each_intersecting_bounding_volume_pair_brute_force(|id_a, id_b| {
            black_box((id_a, id_b));
        });
    });
}

fn add_primitive_volumes(
    bvh: &mut BoundingVolumeHierarchy,
    aabbs: impl Iterator<Item = AxisAlignedBoxC>,
) {
    for (i, aabb) in aabbs.enumerate() {
        bvh.add_primitive_volume(BoundingVolumeID::from_u64(i as u64), aabb)
            .unwrap();
    }
}

/// AABBs placed in grid cells with hashed offsets and sizes.
fn stratified_random_aabbs(count: usize, size_scale: f32) -> impl Iterator<Item = AxisAlignedBoxC> {
    let per_axis = (count as f32).cbrt().ceil() as usize;
    (0..count).map(move |i| {
        let cell_x = (i % per_axis) as f32;
        let cell_y = ((i / per_axis) % per_axis) as f32;
        let cell_z = (i / (per_axis * per_axis)) as f32;

        let idx_bytes = (i as u32).to_le_bytes();
        let h0 = Hash32::from_bytes(&idx_bytes).to_u32();
        let h1 = Hash32::from_bytes(&h0.to_le_bytes()).to_u32();

        // Extract three independent [0, 1) offsets and a [0, 1) size factor from the hash bits.
        let offset_x = (h0 & 0xFF) as f32 / 256.0;
        let offset_y = ((h0 >> 8) & 0xFF) as f32 / 256.0;
        let offset_z = ((h0 >> 16) & 0xFF) as f32 / 256.0;
        let size_factor = (h1 & 0xFF) as f32 / 256.0;

        let cell_size = 2.0;
        let half_extent = 0.25 + size_scale * size_factor * 0.5;
        let center_x = cell_x * cell_size + offset_x;
        let center_y = cell_y * cell_size + offset_y;
        let center_z = cell_z * cell_size + offset_z;

        AxisAlignedBoxC::new(
            Point3C::new(
                center_x - half_extent,
                center_y - half_extent,
                center_z - half_extent,
            ),
            Point3C::new(
                center_x + half_extent,
                center_y + half_extent,
                center_z + half_extent,
            ),
        )
    })
}

/// Generates query AABBs with varying positions and sizes that span the extent
/// of the BVH.
fn generate_query_aabbs(bvh: &BoundingVolumeHierarchy, count: usize) -> Vec<AxisAlignedBox> {
    let root = bvh.root_bounding_volume();
    let lower = root.lower_corner();
    let upper = root.upper_corner();
    let extent = [
        upper.x() - lower.x(),
        upper.y() - lower.y(),
        upper.z() - lower.z(),
    ];

    (0..count)
        .map(|i| {
            let idx_bytes = (i as u32).to_le_bytes();
            let h0 = Hash32::from_bytes(&idx_bytes).to_u32();
            let h1 = Hash32::from_bytes(&h0.to_le_bytes()).to_u32();

            // Position within the BVH extent (center of query box).
            let cx = lower.x() + (h0 & 0xFF) as f32 / 255.0 * extent[0];
            let cy = lower.y() + ((h0 >> 8) & 0xFF) as f32 / 255.0 * extent[1];
            let cz = lower.z() + ((h0 >> 16) & 0xFF) as f32 / 255.0 * extent[2];

            // Half-extent varies from small fraction to full extent of the BVH.
            let max_extent = extent[0].max(extent[1]).max(extent[2]);
            let half_extent = 0.5 + (h1 & 0xFF) as f32 / 255.0 * max_extent * 0.5;

            AxisAlignedBox::new(
                Point3C::new(cx - half_extent, cy - half_extent, cz - half_extent).aligned(),
                Point3C::new(cx + half_extent, cy + half_extent, cz + half_extent).aligned(),
            )
        })
        .collect()
}
