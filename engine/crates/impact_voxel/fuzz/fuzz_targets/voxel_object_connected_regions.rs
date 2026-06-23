#![no_main]

use impact_voxel::{
    chunks::split_detection::fuzzing::fuzz_test_voxel_object_connected_regions,
    generation::SDFVoxelGenerator,
};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|generator: SDFVoxelGenerator| {
    fuzz_test_voxel_object_connected_regions(generator);
});
