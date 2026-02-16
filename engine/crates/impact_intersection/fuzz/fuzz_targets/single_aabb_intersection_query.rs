#![no_main]

use impact_intersection::bounding_volume::hierarchy::fuzzing::{
    ArbitraryAABB, fuzz_test_single_aabb_intersection_query,
};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|input: (Vec<ArbitraryAABB>, ArbitraryAABB)| {
    fuzz_test_single_aabb_intersection_query(input);
});
