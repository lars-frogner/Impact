use criterion::{Criterion, criterion_group, criterion_main};
use impact_ecs::profiling::benchmarks::entity;
use impact_profiling::define_criterion_target;
use pprof::criterion::{Output, PProfProfiler};

define_criterion_target!(entity, create_single_entity_single_comp);
define_criterion_target!(entity, create_single_entity_multiple_comps);
define_criterion_target!(entity, create_multiple_identical_entities);
define_criterion_target!(entity, create_multiple_different_entities);
define_criterion_target!(entity, get_only_entity);
define_criterion_target!(entity, get_one_of_many_different_entities);
define_criterion_target!(entity, get_component_of_only_entity);
define_criterion_target!(entity, get_component_of_one_of_many_different_entities);
define_criterion_target!(entity, modify_component_of_only_entity);
define_criterion_target!(entity, modify_component_of_one_of_many_different_entities);

criterion_group!(
    name = benches;
    config = Criterion::default().with_profiler(PProfProfiler::new(100, Output::Flamegraph(None)));
    targets =
        create_single_entity_single_comp,
        create_single_entity_multiple_comps,
        create_multiple_identical_entities,
        create_multiple_different_entities,
        get_only_entity,
        get_one_of_many_different_entities,
        get_component_of_only_entity,
        get_component_of_one_of_many_different_entities,
        modify_component_of_only_entity,
        modify_component_of_one_of_many_different_entities,
);
criterion_main!(benches);
