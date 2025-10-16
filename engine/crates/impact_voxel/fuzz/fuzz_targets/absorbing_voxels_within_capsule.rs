#![no_main]

use impact_voxel::{
    chunks::intersection::fuzzing::{ArbitraryCapsule, fuzz_test_absorbing_voxels_within_capsule},
    generation::SDFVoxelGenerator,
};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|input: (SDFVoxelGenerator, ArbitraryCapsule)| {
    fuzz_test_absorbing_voxels_within_capsule(input);
});
