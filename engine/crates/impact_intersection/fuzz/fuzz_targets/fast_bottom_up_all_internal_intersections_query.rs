#![no_main]

use impact_intersection::bounding_volume::hierarchy::{
    BVHBuildMethod,
    fuzzing::{ArbitraryAABB, fuzz_test_all_internal_intersections_query},
};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|input: Vec<ArbitraryAABB>| {
    fuzz_test_all_internal_intersections_query(BVHBuildMethod::FastBottomUp, input);
});
