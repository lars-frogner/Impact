#![no_main]

use impact_voxel::chunks::extraction::fuzzing::{
    CopyPolyhedronInput, fuzz_test_voxel_object_copy_polyhedron,
};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|input: CopyPolyhedronInput| {
    fuzz_test_voxel_object_copy_polyhedron(input);
});
