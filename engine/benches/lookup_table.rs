use impact::benchmark::benchmarks::lookup_table;
use impact_profiling::{benchmark::criterion, define_criterion_target};

define_criterion_target!(lookup_table, compute_specular_ggx_reflectance);
define_criterion_target!(lookup_table, compute_black_body_luminance);

criterion::criterion_group!(
    name = benches;
    config = criterion::config();
    targets =
        compute_specular_ggx_reflectance,
        compute_black_body_luminance
);
criterion::criterion_main!(benches);
