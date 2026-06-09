use impact::benchmark::benchmarks::delaunay;
use impact_profiling::{benchmark::criterion, define_criterion_target};

define_criterion_target!(delaunay, construct_from_randomized_grid_points);
define_criterion_target!(delaunay, construct_from_regular_grid_points);

criterion::criterion_group!(
    name = benches;
    config = criterion::config();
    targets =
        construct_from_randomized_grid_points,
        construct_from_regular_grid_points,
);
criterion::criterion_main!(benches);
