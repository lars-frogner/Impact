#![no_main]

use impact_voxel::{
    generation::SDFVoxelGenerator,
    object::extraction::fuzzing::fuzz_test_voxel_object_split_off_disconnected_region,
};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|generator: SDFVoxelGenerator| {
    fuzz_test_voxel_object_split_off_disconnected_region(generator);
});
