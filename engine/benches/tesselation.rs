use impact::benchmark::benchmarks::tesselation;
use impact_profiling::{benchmark::criterion, define_criterion_target};

define_criterion_target!(tesselation, delaunay_tetrahedralize_randomized_grid_points);
define_criterion_target!(tesselation, delaunay_tetrahedralize_regular_grid_points);
define_criterion_target!(
    tesselation,
    voronoi_diagram_from_randomized_delaunay_tetrahedralization
);
define_criterion_target!(
    tesselation,
    voronoi_diagram_from_regular_delaunay_tetrahedralization
);

criterion::criterion_group!(
    name = benches;
    config = criterion::config();
    targets =
        delaunay_tetrahedralize_randomized_grid_points,
        delaunay_tetrahedralize_regular_grid_points,
        voronoi_diagram_from_randomized_delaunay_tetrahedralization,
        voronoi_diagram_from_regular_delaunay_tetrahedralization
);
criterion::criterion_main!(benches);
