#![no_main]

use impact_voxel::object::extraction::fuzzing::{
    ExtractPolyhedronInput, fuzz_test_voxel_object_extract_polyhedron,
};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|input: ExtractPolyhedronInput| {
    fuzz_test_voxel_object_extract_polyhedron(input);
});
