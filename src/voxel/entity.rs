//! Management of voxel-related components for entities.

use crate::{
    gpu::rendering::fre,
    voxel::{
        components::{VoxelBoxComp, VoxelSphereComp, VoxelTreeComp, VoxelTypeComp},
        generation::{UniformBoxVoxelGenerator, UniformSphereVoxelGenerator},
        VoxelManager, VoxelTree,
    },
};
use impact_ecs::{archetype::ArchetypeComponentStorage, setup};
use std::sync::RwLock;

pub fn add_voxel_tree_component_for_entity(
    voxel_manager: &RwLock<VoxelManager<fre>>,
    components: &mut ArchetypeComponentStorage,
    voxel_extent: fre,
) {
    setup!(
        {
            let mut voxel_manager = voxel_manager.write().unwrap();
        },
        components,
        |voxel_box: &VoxelBoxComp, voxel_type: &VoxelTypeComp| -> VoxelTreeComp {
            let generator = UniformBoxVoxelGenerator::new(
                voxel_type.voxel_type(),
                voxel_extent,
                voxel_box.size_x,
                voxel_box.size_y,
                voxel_box.size_z,
            );

            let voxel_tree =
                VoxelTree::build(&generator).expect("Tried to build tree for empty voxel box");

            let voxel_tree_id = voxel_manager.add_voxel_tree(voxel_tree);

            VoxelTreeComp { voxel_tree_id }
        },
        ![VoxelTreeComp]
    );

    setup!(
        {
            let mut voxel_manager = voxel_manager.write().unwrap();
        },
        components,
        |voxel_sphere: &VoxelSphereComp, voxel_type: &VoxelTypeComp| -> VoxelTreeComp {
            let generator = UniformSphereVoxelGenerator::new(
                voxel_type.voxel_type(),
                voxel_extent,
                voxel_sphere.n_voxels_across(),
                voxel_sphere.instance_group_height(),
            );

            let voxel_tree =
                VoxelTree::build(&generator).expect("Tried to build tree for empty voxel sphere");

            let voxel_tree_id = voxel_manager.add_voxel_tree(voxel_tree);

            VoxelTreeComp { voxel_tree_id }
        },
        ![VoxelTreeComp]
    );
}
