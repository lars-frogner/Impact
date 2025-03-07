use criterion::{Criterion, criterion_group, criterion_main};
use impact::model::{
    DynamicInstanceFeatureBuffer, InstanceFeatureStorage, transform::InstanceModelViewTransform,
};
use pprof::criterion::{Output, PProfProfiler};

pub fn bench_dynamic_instance_feature_buffer_add_feature_from_storage(c: &mut Criterion) {
    c.bench_function("instance_feature_buffer_add_feature_from_storage", |b| {
        b.iter(|| {
            let mut storage = InstanceFeatureStorage::new::<InstanceModelViewTransform>();
            let id = storage.add_feature(&InstanceModelViewTransform::identity());
            let mut buffer = DynamicInstanceFeatureBuffer::new_for_storage(&storage);
            for _ in 0..200000 {
                buffer.add_feature_from_storage(&storage, id);
            }
        })
    });
}

pub fn bench_dynamic_instance_feature_buffer_add_feature_from_storage_repeatedly(
    c: &mut Criterion,
) {
    c.bench_function(
        "instance_feature_buffer_add_feature_from_storage_repeatedly",
        |b| {
            b.iter(|| {
                let mut storage = InstanceFeatureStorage::new::<InstanceModelViewTransform>();
                let id = storage.add_feature(&InstanceModelViewTransform::identity());
                let mut buffer = DynamicInstanceFeatureBuffer::new_for_storage(&storage);
                buffer.add_feature_from_storage_repeatedly(&storage, id, 200000);
            })
        },
    );
}

criterion_group!(
    name = benches;
    config = Criterion::default().with_profiler(PProfProfiler::new(100, Output::Flamegraph(None)));
    targets =
        bench_dynamic_instance_feature_buffer_add_feature_from_storage,
        bench_dynamic_instance_feature_buffer_add_feature_from_storage_repeatedly,
);
criterion_main!(benches);
