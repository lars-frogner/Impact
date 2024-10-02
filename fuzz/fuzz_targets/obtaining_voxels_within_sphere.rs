#![no_main]

use impact::voxel::{
    chunks::intersection::fuzzing::{fuzz_test_obtaining_voxels_within_sphere, ArbitrarySphere},
    generation::fuzzing::ArbitrarySDFVoxelGenerator,
};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|input: (ArbitrarySDFVoxelGenerator, ArbitrarySphere)| {
    fuzz_test_obtaining_voxels_within_sphere(input);
});
