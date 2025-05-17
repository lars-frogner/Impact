//! Utilities for initiating profiling.

impact_profiling::define_target_enum! {
    Target,
    crate::profiling::benchmarks,
    world => {
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
    },
}

pub fn profile(target: Target, duration: f64, delay: f64) {
    impact_profiling::profile::profile(|profiler| target.execute(profiler), duration, delay);
}
