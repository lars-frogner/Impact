#![no_main]

use impact_voxel::{
    chunks::intersection::fuzzing::{
        ArbitrarySphere, fuzz_test_obtaining_surface_voxels_maybe_intersecting_sphere,
    },
    generation::fuzzing::ArbitrarySDFVoxelGenerator,
};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|input: (ArbitrarySDFVoxelGenerator, ArbitrarySphere)| {
    fuzz_test_obtaining_surface_voxels_maybe_intersecting_sphere(input);
});
