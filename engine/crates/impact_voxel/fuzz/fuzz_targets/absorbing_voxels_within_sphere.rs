#![no_main]

use impact_voxel::{
    chunks::intersection::fuzzing::{ArbitrarySphere, fuzz_test_absorbing_voxels_within_sphere},
    generation::SDFVoxelGenerator,
};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|input: (SDFVoxelGenerator, ArbitrarySphere)| {
    fuzz_test_absorbing_voxels_within_sphere(input);
});
