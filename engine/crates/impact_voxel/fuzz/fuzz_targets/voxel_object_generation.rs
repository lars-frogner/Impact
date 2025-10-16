#![no_main]

use impact_voxel::{
    chunks::fuzzing::fuzz_test_voxel_object_generation, generation::SDFVoxelGenerator,
};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|generator: SDFVoxelGenerator| {
    fuzz_test_voxel_object_generation(generator);
});
