use criterion::{Criterion, criterion_group, criterion_main};
use impact_ecs::profiling::benchmarks::query;
use impact_profiling::define_criterion_target;
use pprof::criterion::{Output, PProfProfiler};

define_criterion_target!(query, query_single_comp_single_entity);
define_criterion_target!(query, query_single_comp_multiple_identical_entities);
define_criterion_target!(query, query_multiple_comps_single_entity);
define_criterion_target!(query, query_multiple_comps_multiple_identical_entities);
define_criterion_target!(query, query_single_comp_multiple_different_entities);
define_criterion_target!(query, query_multiple_comps_multiple_different_entities);

criterion_group!(
    name = benches;
    config = Criterion::default().with_profiler(PProfProfiler::new(100, Output::Flamegraph(None)));
    targets =
        query_single_comp_single_entity,
        query_single_comp_multiple_identical_entities,
        query_multiple_comps_single_entity,
        query_multiple_comps_multiple_identical_entities,
        query_single_comp_multiple_different_entities,
        query_multiple_comps_multiple_different_entities,
);
criterion_main!(benches);
