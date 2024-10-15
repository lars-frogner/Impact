#![no_main]

use impact::voxel::{
    chunks::split::fuzzing::fuzz_test_voxel_object_split_off_disconnected_region,
    generation::fuzzing::ArbitrarySDFVoxelGenerator,
};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|generator: ArbitrarySDFVoxelGenerator| {
    fuzz_test_voxel_object_split_off_disconnected_region(generator);
});
