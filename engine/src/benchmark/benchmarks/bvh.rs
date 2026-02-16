//! Benchmarks for bounding volume hierarchies.

use impact_geometry::{AxisAlignedBox, AxisAlignedBoxC};
use impact_intersection::bounding_volume::{BoundingVolumeID, hierarchy::BoundingVolumeHierarchy};
use impact_math::{hash::Hash32, point::Point3C, vector::Vector3C};
use impact_profiling::benchmark::Benchmarker;

fn create_bvh_with_aabbs(aabbs: impl Iterator<Item = AxisAlignedBoxC>) -> BoundingVolumeHierarchy {
    let mut bvh = BoundingVolumeHierarchy::new();
    for (i, aabb) in aabbs.enumerate() {
        bvh.add_primitive_volume(BoundingVolumeID::from_u64(i as u64), aabb)
            .unwrap();
    }
    bvh
}

/// AABBs uniformly distributed along the x-axis with no overlap.
fn non_overlapping_aabbs(count: usize) -> impl Iterator<Item = AxisAlignedBoxC> {
    (0..count).map(|i| {
        let x = i as f32 * 2.0;
        AxisAlignedBoxC::new(Point3C::new(x, 0.0, 0.0), Point3C::new(x + 1.0, 1.0, 1.0))
    })
}

/// Identical AABBs all at the origin.
fn fully_overlapping_aabbs(count: usize) -> impl Iterator<Item = AxisAlignedBoxC> {
    (0..count)
        .map(|_| AxisAlignedBoxC::new(Vector3C::same(-0.5).into(), Vector3C::same(0.5).into()))
}

/// AABBs spread across 3D space in a grid pattern.
fn grid_distributed_aabbs(count: usize) -> impl Iterator<Item = AxisAlignedBoxC> {
    let per_axis = (count as f32).cbrt().ceil() as usize;
    (0..count).map(move |i| {
        let x = (i % per_axis) as f32 * 2.0;
        let y = ((i / per_axis) % per_axis) as f32 * 2.0;
        let z = (i / (per_axis * per_axis)) as f32 * 2.0;
        AxisAlignedBoxC::new(
            Point3C::new(x, y, z),
            Point3C::new(x + 1.0, y + 1.0, z + 1.0),
        )
    })
}

/// AABBs with varying sizes, partially overlapping along the x-axis.
fn varying_size_aabbs(count: usize) -> impl Iterator<Item = AxisAlignedBoxC> {
    (0..count).map(|i| {
        let half_extent = 0.5 + (i % 5) as f32 * 0.5;
        let x = i as f32 * 1.5;
        AxisAlignedBoxC::new(
            Point3C::new(x - half_extent, -half_extent, -half_extent),
            Point3C::new(x + half_extent, half_extent, half_extent),
        )
    })
}

/// AABBs placed in grid cells with hashed offsets and sizes.
fn stratified_random_aabbs(count: usize) -> impl Iterator<Item = AxisAlignedBoxC> {
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
        let half_extent = 0.25 + size_factor * 0.5;
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

const N: usize = 1000;

pub fn build_non_overlapping(benchmarker: impl Benchmarker) {
    let mut bvh = create_bvh_with_aabbs(non_overlapping_aabbs(N));
    benchmarker.benchmark(&mut || bvh.build());
}

pub fn build_fully_overlapping(benchmarker: impl Benchmarker) {
    let mut bvh = create_bvh_with_aabbs(fully_overlapping_aabbs(N));
    benchmarker.benchmark(&mut || bvh.build());
}

pub fn build_grid_distributed(benchmarker: impl Benchmarker) {
    let mut bvh = create_bvh_with_aabbs(grid_distributed_aabbs(N));
    benchmarker.benchmark(&mut || bvh.build());
}

pub fn build_varying_size(benchmarker: impl Benchmarker) {
    let mut bvh = create_bvh_with_aabbs(varying_size_aabbs(N));
    benchmarker.benchmark(&mut || bvh.build());
}

pub fn build_stratified_random(benchmarker: impl Benchmarker) {
    let mut bvh = create_bvh_with_aabbs(stratified_random_aabbs(N));
    benchmarker.benchmark(&mut || bvh.build());
}

fn create_built_stratified_bvh() -> BoundingVolumeHierarchy {
    let mut bvh = create_bvh_with_aabbs(stratified_random_aabbs(N));
    bvh.build();
    bvh
}

/// Query with a small AABB near the origin (few hits).
pub fn query_small_aabb(benchmarker: impl Benchmarker) {
    let bvh = create_built_stratified_bvh();
    let query = AxisAlignedBox::new(
        Point3C::new(0.0, 0.0, 0.0).aligned(),
        Point3C::new(2.0, 2.0, 2.0).aligned(),
    );
    benchmarker.benchmark(&mut || {
        bvh.for_each_bounding_volume_in_axis_aligned_box(&query, |_| {});
    });
}

/// Query with a medium AABB covering roughly a quarter of the volume (moderate hits).
pub fn query_medium_aabb(benchmarker: impl Benchmarker) {
    let bvh = create_built_stratified_bvh();
    let query = AxisAlignedBox::new(
        Point3C::new(0.0, 0.0, 0.0).aligned(),
        Point3C::new(10.0, 10.0, 10.0).aligned(),
    );
    benchmarker.benchmark(&mut || {
        bvh.for_each_bounding_volume_in_axis_aligned_box(&query, |_| {});
    });
}

/// Query with an AABB encompassing the full hierarchy (all hits).
pub fn query_full_aabb(benchmarker: impl Benchmarker) {
    let bvh = create_built_stratified_bvh();
    let query = bvh.root_bounding_volume();
    benchmarker.benchmark(&mut || {
        bvh.for_each_bounding_volume_in_axis_aligned_box(&query, |_| {});
    });
}
