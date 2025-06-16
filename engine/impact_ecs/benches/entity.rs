use impact_ecs::profiling::benchmarks::entity;
use impact_profiling::{criterion, define_criterion_target};

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

criterion::criterion_group!(
    name = benches;
    config = criterion::config();
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
criterion::criterion_main!(benches);
