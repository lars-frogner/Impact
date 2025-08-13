#![no_main]

use impact_voxel::{
    chunks::intersection::fuzzing::{
        ArbitraryPlane,
        fuzz_test_obtaining_surface_voxels_maybe_intersecting_negative_halfspace_of_plane,
    },
    generation::fuzzing::ArbitrarySDFVoxelGenerator,
};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|input: (ArbitrarySDFVoxelGenerator, ArbitraryPlane)| {
    fuzz_test_obtaining_surface_voxels_maybe_intersecting_negative_halfspace_of_plane(input);
});
