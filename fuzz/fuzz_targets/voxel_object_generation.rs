#![no_main]

use impact::voxel::{
    chunks::fuzz_test_voxel_object_generation, generation::fuzzing::ArbitrarySDFVoxelGenerator,
};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|generator: ArbitrarySDFVoxelGenerator| {
    fuzz_test_voxel_object_generation(generator);
});
