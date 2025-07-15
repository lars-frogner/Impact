#![no_main]

use impact_voxel::{
    chunks::disconnection::fuzzing::fuzz_test_voxel_object_connected_regions,
    generation::fuzzing::ArbitrarySDFVoxelGenerator,
};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|generator: ArbitrarySDFVoxelGenerator| {
    fuzz_test_voxel_object_connected_regions(generator);
});
