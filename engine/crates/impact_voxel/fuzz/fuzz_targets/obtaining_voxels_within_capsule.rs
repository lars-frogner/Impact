#![no_main]

use impact_voxel::{
    generation::SDFVoxelGenerator,
    object::intersection::fuzzing::{ArbitraryCapsule, fuzz_test_obtaining_voxels_within_capsule},
};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|input: (SDFVoxelGenerator, ArbitraryCapsule)| {
    fuzz_test_obtaining_voxels_within_capsule(input);
});
