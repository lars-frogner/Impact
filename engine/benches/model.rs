use criterion::{Criterion, criterion_group, criterion_main};
use impact::profiling::benchmarks::model;
use impact_profiling::define_criterion_target;
use pprof::criterion::{Output, PProfProfiler};

define_criterion_target!(model, add_feature_to_dynamic_instance_buffer_from_storage);
define_criterion_target!(
    model,
    add_feature_to_dynamic_instance_buffer_from_storage_repeatedly
);

criterion_group!(
    name = benches;
    config = Criterion::default().with_profiler(PProfProfiler::new(100, Output::Flamegraph(None)));
    targets =
        add_feature_to_dynamic_instance_buffer_from_storage,
        add_feature_to_dynamic_instance_buffer_from_storage_repeatedly,
);
criterion_main!(benches);
