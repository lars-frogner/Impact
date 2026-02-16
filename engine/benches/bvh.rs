use impact::benchmark::benchmarks::bvh;
use impact_profiling::{benchmark::criterion, define_criterion_target};

define_criterion_target!(bvh, build_non_overlapping);
define_criterion_target!(bvh, build_fully_overlapping);
define_criterion_target!(bvh, build_grid_distributed);
define_criterion_target!(bvh, build_varying_size);
define_criterion_target!(bvh, build_stratified_random);
define_criterion_target!(bvh, query_small_aabb);
define_criterion_target!(bvh, query_medium_aabb);
define_criterion_target!(bvh, query_full_aabb);

criterion::criterion_group!(
    name = benches;
    config = criterion::config();
    targets =
        build_non_overlapping,
        build_fully_overlapping,
        build_grid_distributed,
        build_varying_size,
        build_stratified_random,
        query_small_aabb,
        query_medium_aabb,
        query_full_aabb,
);
criterion::criterion_main!(benches);
