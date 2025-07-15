#![no_main]

use impact_voxel::{
    chunks::intersection::fuzzing::{ArbitraryCapsule, fuzz_test_obtaining_voxels_within_capsule},
    generation::fuzzing::ArbitrarySDFVoxelGenerator,
};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|input: (ArbitrarySDFVoxelGenerator, ArbitraryCapsule)| {
    fuzz_test_obtaining_voxels_within_capsule(input);
});
