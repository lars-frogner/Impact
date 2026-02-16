//! Geometry intersection testing for the Impact engine.

#[macro_use]
mod macros;

pub mod bounding_volume;

use anyhow::Result;
use bounding_volume::{
    BoundingVolumeID, BoundingVolumeManager, hierarchy::BoundingVolumeHierarchy,
};
use impact_geometry::{AxisAlignedBox, Frustum};
use impact_math::transform::Similarity3;

use crate::bounding_volume::AxisAlignedBoundingBox;

#[derive(Debug)]
pub struct IntersectionManager {
    pub bounding_volume_manager: BoundingVolumeManager,
    bvh: BoundingVolumeHierarchy,
}

impl IntersectionManager {
    pub fn new() -> Self {
        Self {
            bounding_volume_manager: BoundingVolumeManager::new(),
            bvh: BoundingVolumeHierarchy::new(),
        }
    }

    /// Transforms the bounding volume with the given ID to world space using
    /// the given transform and adds it to the bounding volume hierarchy.
    ///
    /// Does nothing if the bounding volume does not exist.
    ///
    /// # Errors
    /// Returns an error if the bounding volume has already been added to the
    /// hierarchy.
    pub fn add_bounding_volume_to_hierarchy(
        &mut self,
        id: BoundingVolumeID,
        model_to_world_transform: &Similarity3,
    ) -> Result<()> {
        let Some(bounding_volume) = self.bounding_volume_manager.get_bounding_volume(id) else {
            return Ok(());
        };

        let world_space_aabb = bounding_volume
            .aligned()
            .aabb_of_transformed(&model_to_world_transform.to_matrix());

        self.bvh
            .add_primitive_volume(id, world_space_aabb.compact())
    }

    pub fn n_bounding_volumes_in_hierarchy(&self) -> usize {
        self.bvh.primitive_count()
    }

    pub fn build_bounding_volume_hierarchy(&mut self) {
        self.bvh.build();
    }

    pub fn total_bounding_volume(&self) -> AxisAlignedBoundingBox {
        self.bvh.root_bounding_volume()
    }

    pub fn for_each_bounding_volume_in_axis_aligned_box(
        &self,
        axis_aligned_box: &AxisAlignedBox,
        f: impl FnMut(BoundingVolumeID),
    ) {
        self.bvh
            .for_each_bounding_volume_in_axis_aligned_box(axis_aligned_box, f);
    }

    pub fn for_each_bounding_volume_maybe_in_frustum(
        &self,
        frustum: &Frustum,
        f: impl FnMut(BoundingVolumeID),
    ) {
        self.bvh
            .for_each_bounding_volume_maybe_in_frustum(frustum, f);
    }

    pub fn for_each_intersecting_bounding_volume_pair<R>(
        &self,
        filter_map_first: impl FnMut(BoundingVolumeID) -> Option<R>,
        process_intersection: impl FnMut(&R, BoundingVolumeID),
    ) {
        self.bvh
            .for_each_intersecting_bounding_volume_pair(filter_map_first, process_intersection);
    }

    pub fn reset_bounding_volume_hierarchy(&mut self) {
        self.bvh.clear();
    }

    /// Removes all intersection state.
    pub fn remove_all_intersection_state(&mut self) {
        self.bounding_volume_manager.remove_all_bounding_volumes();
        self.bvh.clear();
    }
}
