use impact::profiling::benchmarks::model;
use impact_profiling::{criterion, define_criterion_target};

define_criterion_target!(model, add_feature_to_dynamic_instance_buffer_from_storage);
define_criterion_target!(
    model,
    add_feature_to_dynamic_instance_buffer_from_storage_repeatedly
);

criterion::criterion_group!(
    name = benches;
    config = criterion::config();
    targets =
        add_feature_to_dynamic_instance_buffer_from_storage,
        add_feature_to_dynamic_instance_buffer_from_storage_repeatedly,
);
criterion::criterion_main!(benches);
