#![no_main]

use impact_tesselation::delaunay::fuzzing::{DelaunayPoint, fuzz_test_delaunay_tetrahedralization};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|input: Vec<DelaunayPoint>| {
    fuzz_test_delaunay_tetrahedralization(input);
});
