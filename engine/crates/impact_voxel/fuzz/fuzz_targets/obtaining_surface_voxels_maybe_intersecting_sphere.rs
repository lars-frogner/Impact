#![no_main]

use impact_voxel::{
    chunks::intersection::fuzzing::{
        ArbitrarySphere, fuzz_test_obtaining_surface_voxels_maybe_intersecting_sphere,
    },
    generation::SDFVoxelGenerator,
};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|input: (SDFVoxelGenerator, ArbitrarySphere)| {
    fuzz_test_obtaining_surface_voxels_maybe_intersecting_sphere(input);
});
