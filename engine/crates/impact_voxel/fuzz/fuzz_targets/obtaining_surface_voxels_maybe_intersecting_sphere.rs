#![no_main]

use impact_voxel::{
    generation::SDFVoxelGenerator,
    object::intersection::fuzzing::{
        ArbitrarySphere, fuzz_test_obtaining_surface_voxels_maybe_intersecting_sphere,
    },
};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|input: (SDFVoxelGenerator, ArbitrarySphere)| {
    fuzz_test_obtaining_surface_voxels_maybe_intersecting_sphere(input);
});
