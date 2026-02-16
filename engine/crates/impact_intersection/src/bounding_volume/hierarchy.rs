//! Bounding volume hierarchy.

use crate::bounding_volume::{AxisAlignedBoundingBox, AxisAlignedBoundingBoxC, BoundingVolumeID};
use anyhow::{Result, anyhow};
use impact_containers::KeyIndexMapper;
use impact_geometry::{AxisAlignedBox, Frustum};
use impact_math::point::Point3;

#[derive(Debug)]
pub struct BoundingVolumeHierarchy {
    primitive_volumes: BVHPrimitiveVolumes,
}

#[derive(Debug)]
struct BVHPrimitiveVolumes {
    bounding_volumes: Vec<AxisAlignedBoundingBoxC>,
    index_map: KeyIndexMapper<BoundingVolumeID>,
}

impl BoundingVolumeHierarchy {
    pub fn new() -> Self {
        Self {
            primitive_volumes: BVHPrimitiveVolumes::new(),
        }
    }

    pub fn add_primitive_volume(
        &mut self,
        id: BoundingVolumeID,
        aabb: AxisAlignedBoundingBoxC,
    ) -> Result<()> {
        self.primitive_volumes.insert(id, aabb)
    }

    pub fn primitive_count(&self) -> usize {
        self.primitive_volumes.bounding_volumes.len()
    }

    pub fn build(&mut self) {}

    pub fn root_bounding_volume(&self) -> AxisAlignedBoundingBox {
        self.primitive_volumes
            .bounding_volumes
            .iter()
            .map(|aabb| aabb.aligned())
            .reduce(|a, b| AxisAlignedBoundingBox::aabb_from_pair(&a, &b))
            .unwrap_or_else(|| AxisAlignedBoundingBox::new(Point3::origin(), Point3::origin()))
    }

    pub fn for_each_bounding_volume_in_axis_aligned_box(
        &self,
        axis_aligned_box: &AxisAlignedBox,
        f: impl FnMut(BoundingVolumeID),
    ) {
        self.for_each_bounding_volume_in_axis_aligned_box_brute_force(axis_aligned_box, f);
    }

    pub fn for_each_bounding_volume_in_axis_aligned_box_brute_force(
        &self,
        axis_aligned_box: &AxisAlignedBox,
        mut f: impl FnMut(BoundingVolumeID),
    ) {
        for (idx, aabb) in self.primitive_volumes.bounding_volumes.iter().enumerate() {
            let aabb = aabb.aligned();
            if !axis_aligned_box.box_lies_outside(&aabb) {
                let id = self.primitive_volumes.index_map.key_at_idx(idx);
                f(id);
            }
        }
    }

    pub fn for_each_bounding_volume_maybe_in_frustum(
        &self,
        frustum: &Frustum,
        mut f: impl FnMut(BoundingVolumeID),
    ) {
        for (idx, aabb) in self.primitive_volumes.bounding_volumes.iter().enumerate() {
            let aabb = aabb.aligned();
            if frustum.could_contain_part_of_axis_aligned_box(&aabb) {
                let id = self.primitive_volumes.index_map.key_at_idx(idx);
                f(id);
            }
        }
    }

    pub fn for_each_intersecting_bounding_volume_pair<R>(
        &self,
        mut filter_map_first: impl FnMut(BoundingVolumeID) -> Option<R>,
        mut process_intersection: impl FnMut(&R, BoundingVolumeID),
    ) {
        let aabbs = &self.primitive_volumes.bounding_volumes;
        let n_primitives = aabbs.len();

        if n_primitives < 2 {
            return;
        }

        for (i, aabb_i) in (0..n_primitives - 1).zip(&aabbs[0..n_primitives - 1]) {
            let id_i = self.primitive_volumes.index_map.key_at_idx(i);
            let Some(mapped_first) = filter_map_first(id_i) else {
                continue;
            };
            for (j, aabb_j) in (i + 1..n_primitives).zip(&aabbs[i + 1..n_primitives]) {
                if !aabb_i.aligned().box_lies_outside(&aabb_j.aligned()) {
                    let id_j = self.primitive_volumes.index_map.key_at_idx(j);
                    process_intersection(&mapped_first, id_j);
                }
            }
        }
    }

    pub fn clear(&mut self) {
        self.primitive_volumes.clear();
    }
}

impl BVHPrimitiveVolumes {
    fn new() -> Self {
        Self {
            bounding_volumes: Vec::new(),
            index_map: KeyIndexMapper::new(),
        }
    }

    fn insert(&mut self, id: BoundingVolumeID, aabb: AxisAlignedBoundingBoxC) -> Result<()> {
        self.index_map
            .try_push_key(id)
            .map_err(|_idx| anyhow!("A bounding volume with ID {id} is already present"))?;
        self.bounding_volumes.push(aabb);
        Ok(())
    }

    fn clear(&mut self) {
        self.bounding_volumes.clear();
        self.index_map.clear();
    }
}

#[cfg(feature = "fuzzing")]
pub mod fuzzing {
    use super::*;
    use arbitrary::{Arbitrary, Result, Unstructured};
    use impact_containers::HashSet;
    use impact_math::vector::Vector3;
    use std::mem;

    #[derive(Clone, Debug)]
    pub struct ArbitraryAABB(AxisAlignedBoundingBox);

    impl Arbitrary<'_> for ArbitraryAABB {
        fn arbitrary(u: &mut Unstructured<'_>) -> Result<Self> {
            let x = 2.0 * arbitrary_norm_f32(u)? - 1.0;
            let y = 2.0 * arbitrary_norm_f32(u)? - 1.0;
            let z = 2.0 * arbitrary_norm_f32(u)? - 1.0;
            let center = Point3::new(x, y, z);

            let ex = arbitrary_norm_f32(u)?;
            let ey = arbitrary_norm_f32(u)?;
            let ez = arbitrary_norm_f32(u)?;
            let half_extents = Vector3::new(ex, ey, ez);

            Ok(Self(AxisAlignedBox::new(
                center - half_extents,
                center + half_extents,
            )))
        }

        fn size_hint(_depth: usize) -> (usize, Option<usize>) {
            let size = 6 * mem::size_of::<i32>();
            (size, Some(size))
        }
    }

    pub fn fuzz_test_single_aabb_intersection_query(
        (aabbs_for_hierarchy, test_aabb): (Vec<ArbitraryAABB>, ArbitraryAABB),
    ) {
        let mut bvh = BoundingVolumeHierarchy::new();
        for (idx, aabb) in aabbs_for_hierarchy.iter().enumerate() {
            bvh.add_primitive_volume(BoundingVolumeID::from_u64(idx as u64), aabb.0.compact())
                .unwrap();
        }
        bvh.build();

        let mut intersected_ids = Vec::new();

        bvh.for_each_bounding_volume_in_axis_aligned_box(&test_aabb.0, |id| {
            intersected_ids.push(id);
        });

        let mut intersected_ids_brute_force = Vec::new();

        bvh.for_each_bounding_volume_in_axis_aligned_box_brute_force(&test_aabb.0, |id| {
            intersected_ids_brute_force.push(id);
        });

        assert_eq!(intersected_ids.len(), intersected_ids_brute_force.len());

        let intersected_ids: HashSet<BoundingVolumeID> = HashSet::from_iter(intersected_ids);
        let intersected_ids_brute_force = HashSet::from_iter(intersected_ids_brute_force);

        assert_eq!(intersected_ids, intersected_ids_brute_force);
    }

    fn arbitrary_norm_f32(u: &mut Unstructured<'_>) -> Result<f32> {
        Ok((f64::from(u.int_in_range(0..=1000000)?) / 1000000.0) as f32)
    }
}
