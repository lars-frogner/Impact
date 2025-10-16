#![no_main]

use impact_voxel::{
    chunks::intersection::fuzzing::{
        ArbitraryPlane,
        fuzz_test_obtaining_surface_voxels_maybe_intersecting_negative_halfspace_of_plane,
    },
    generation::SDFVoxelGenerator,
};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|input: (SDFVoxelGenerator, ArbitraryPlane)| {
    fuzz_test_obtaining_surface_voxels_maybe_intersecting_negative_halfspace_of_plane(input);
});
