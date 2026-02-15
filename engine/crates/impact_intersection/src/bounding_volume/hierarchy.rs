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

    pub fn for_each_bounding_volume_in_axis_aligned_box(
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

    pub fn for_each_intersecting_bounding_volume_pair(
        &self,
        mut f: impl FnMut(BoundingVolumeID, BoundingVolumeID),
    ) {
        let aabbs = &self.primitive_volumes.bounding_volumes;
        let n_primitives = aabbs.len();

        if n_primitives < 2 {
            return;
        }

        for (i, aabb_i) in (0..n_primitives - 1).zip(&aabbs[0..n_primitives - 1]) {
            for (j, aabb_j) in (i + 1..n_primitives).zip(&aabbs[i + 1..n_primitives]) {
                if !aabb_i.aligned().box_lies_outside(&aabb_j.aligned()) {
                    let id_i = self.primitive_volumes.index_map.key_at_idx(i);
                    let id_j = self.primitive_volumes.index_map.key_at_idx(j);
                    f(id_i, id_j);
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
