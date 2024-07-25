use criterion::{criterion_group, criterion_main, Criterion};
use impact::{
    geometry::{Degrees, Frustum, PerspectiveTransform},
    model::{
        transform::InstanceModelViewTransform, DynamicInstanceFeatureBuffer, InstanceFeatureStorage,
    },
    util::bounds::UpperExclusiveBounds,
    voxel::{generation::UniformSphereVoxelGenerator, VoxelTree, VoxelType},
};
use nalgebra::{vector, Similarity3, UnitQuaternion, Vector3};
use num_traits::FloatConst;
use pprof::criterion::{Output, PProfProfiler};
use rand::{self, Rng};

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
            let generator = UniformSphereVoxelGenerator::new(VoxelType::Default, 0.25_f32, 128, 0);
            VoxelTree::build(&generator).unwrap();
        })
    });
}

pub fn bench_voxel_transform_buffering(c: &mut Criterion) {
    let generator = UniformSphereVoxelGenerator::new(VoxelType::Default, 0.25_f32, 430, 4);
    let tree = VoxelTree::build(&generator).unwrap();

    let view_frustum = Frustum::from_transform(
        PerspectiveTransform::new(1.0, Degrees(70.0), UpperExclusiveBounds::new(0.01, 500.0))
            .as_projective(),
    );
    let view_transform = Similarity3::identity();
    let radial_distance = 2.0 * generator.radius();

    let mut rng = rand::thread_rng();

    let mut transform_buffer = DynamicInstanceFeatureBuffer::new::<InstanceModelViewTransform>();

    let mut total_transform_buffer_len = 0;
    let mut iter_count = 0;

    c.bench_function("voxel_transform_buffering", |b| {
        b.iter(|| {
            let phi = rng.gen::<f32>() * f32::TAU();
            let theta = 0.5 * f32::PI(); // rng.gen::<f32>() * f32::PI();

            let offset = vector![
                radial_distance * f32::sin(phi) * f32::sin(theta),
                radial_distance * f32::cos(theta),
                radial_distance * f32::cos(phi) * f32::sin(theta)
            ];
            let camera_position = generator.center() + offset;
            let transformation = Similarity3::from_parts(
                camera_position.into(),
                UnitQuaternion::rotation_between(&(-Vector3::z()), &(-offset)).unwrap_or_default(),
                1.0,
            );
            let transformed_view_frustum = view_frustum.transformed(&transformation);

            tree.buffer_visible_voxel_instances(
                &transformed_view_frustum,
                &view_transform,
                &|voxel_position| voxel_position - camera_position,
                &|_, _| 0,
                &mut |storage, camera_space_axes_in_tree_space| {
                    storage.buffer_all_transforms(
                        &mut transform_buffer,
                        &view_transform,
                        camera_space_axes_in_tree_space,
                    )
                },
            );

            total_transform_buffer_len += transform_buffer.n_valid_features();
            iter_count += 1;

            transform_buffer.clear();
        });
    });

    println!(
        "Average transform count: {}",
        f64::round((total_transform_buffer_len as f64) / (iter_count as f64)) as u64
    );
}

criterion_group!(
    name = benches;
    config = Criterion::default().with_profiler(PProfProfiler::new(100, Output::Flamegraph(None)));
    targets =
        bench_dynamic_instance_feature_buffer_add_feature_from_storage,
        bench_dynamic_instance_feature_buffer_add_feature_from_storage_repeatedly,
        bench_voxel_tree_construction,
        bench_voxel_transform_buffering
);
criterion_main!(benches);
