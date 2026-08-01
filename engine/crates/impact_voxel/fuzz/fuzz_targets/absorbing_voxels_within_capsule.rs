#![no_main]

use impact_voxel::{
    generation::SDFVoxelGenerator,
    object::intersection::fuzzing::{ArbitraryCapsule, fuzz_test_absorbing_voxels_within_capsule},
};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|input: (SDFVoxelGenerator, Vec<ArbitraryCapsule>)| {
    fuzz_test_absorbing_voxels_within_capsule(input);
});
