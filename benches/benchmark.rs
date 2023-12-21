use criterion::{criterion_group, criterion_main, Criterion};
use impact::geometry::{
    DynamicInstanceFeatureBuffer, InstanceFeatureStorage, InstanceModelViewTransform,
    UniformSphereVoxelGenerator, VoxelTree, VoxelType,
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

pub fn bench_voxel_tree_construction(c: &mut Criterion) {
    c.bench_function("voxel_tree_construction", |b| {
        b.iter(|| {
            let generator = UniformSphereVoxelGenerator::new(VoxelType::Default, 0.25, 128);
            VoxelTree::build(&generator).unwrap();
        })
    });
}

criterion_group!(
    name = benches;
    config = Criterion::default().with_profiler(PProfProfiler::new(100, Output::Flamegraph(None)));
    targets =
        bench_dynamic_instance_feature_buffer_add_feature_from_storage,
        bench_dynamic_instance_feature_buffer_add_feature_from_storage_repeatedly,
        bench_voxel_tree_construction
);
criterion_main!(benches);
