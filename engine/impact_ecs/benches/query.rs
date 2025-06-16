use impact_ecs::profiling::benchmarks::query;
use impact_profiling::{criterion, define_criterion_target};

define_criterion_target!(query, query_single_comp_single_entity);
define_criterion_target!(query, query_single_comp_multiple_identical_entities);
define_criterion_target!(query, query_multiple_comps_single_entity);
define_criterion_target!(query, query_multiple_comps_multiple_identical_entities);
define_criterion_target!(query, query_single_comp_multiple_different_entities);
define_criterion_target!(query, query_multiple_comps_multiple_different_entities);

criterion::criterion_group!(
    name = benches;
    config = impact_profiling::criterion::config();
    targets =
        query_single_comp_single_entity,
        query_single_comp_multiple_identical_entities,
        query_multiple_comps_single_entity,
        query_multiple_comps_multiple_identical_entities,
        query_single_comp_multiple_different_entities,
        query_multiple_comps_multiple_different_entities,
);
criterion::criterion_main!(benches);
