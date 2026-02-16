use impact::benchmark::benchmarks::bvh;
use impact_profiling::{benchmark::criterion, define_criterion_target};

define_criterion_target!(bvh, build_stratified_random);
define_criterion_target!(bvh, query_many_external_intersections);
define_criterion_target!(bvh, query_all_internal_intersections);
define_criterion_target!(bvh, query_with_brute_force_many_external_intersections);
define_criterion_target!(bvh, query_with_brute_force_all_internal_intersections);

criterion::criterion_group!(
    name = benches;
    config = criterion::config();
    targets =
        build_stratified_random,
        query_many_external_intersections,
        query_all_internal_intersections,
        query_with_brute_force_many_external_intersections,
        query_with_brute_force_all_internal_intersections,
);
criterion::criterion_main!(benches);
