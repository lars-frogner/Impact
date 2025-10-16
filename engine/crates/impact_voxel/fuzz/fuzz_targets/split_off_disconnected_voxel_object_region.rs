#![no_main]

use impact_voxel::{
    chunks::disconnection::fuzzing::fuzz_test_voxel_object_split_off_disconnected_region,
    generation::SDFVoxelGenerator,
};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|generator: SDFVoxelGenerator| {
    fuzz_test_voxel_object_split_off_disconnected_region(generator);
});
