use criterion::{black_box, criterion_group, criterion_main, Criterion};
use impact::{
    geometry::Sphere,
    voxel::{
        chunks::ChunkedVoxelObject,
        generation::{
            BoxSDFGenerator, SDFVoxelGenerator, SameVoxelTypeGenerator, SphereSDFGenerator,
        },
        mesh::ChunkedVoxelObjectMesh,
        voxel_types::VoxelType,
    },
};
use nalgebra::{vector, UnitVector3};
use pprof::criterion::{Output, PProfProfiler};

pub fn bench_chunked_voxel_object_construction(c: &mut Criterion) {
    c.bench_function("chunked_voxel_object_construction", |b| {
        b.iter(|| {
            let generator = SDFVoxelGenerator::new(
                1.0,
                BoxSDFGenerator::new([200.0; 3]),
                SameVoxelTypeGenerator::new(VoxelType::default()),
            );
            ChunkedVoxelObject::generate_without_adjacencies(&generator).unwrap();
        })
    });
}

pub fn bench_chunked_voxel_object_get_each_voxel(c: &mut Criterion) {
    let generator = SDFVoxelGenerator::new(
        1.0,
        SphereSDFGenerator::new(100.0),
        SameVoxelTypeGenerator::new(VoxelType::default()),
    );
    let object = ChunkedVoxelObject::generate_without_adjacencies(&generator).unwrap();
    let ranges = object.occupied_voxel_ranges();
    c.bench_function("chunked_voxel_object_get_each_voxel", |b| {
        b.iter(|| {
            for i in ranges[0].clone() {
                for j in ranges[1].clone() {
                    for k in ranges[2].clone() {
                        let _ = black_box(object.get_voxel(i, j, k));
                    }
                }
            }
        })
    });
}

pub fn bench_chunked_voxel_object_initialize_adjacencies(c: &mut Criterion) {
    let generator = SDFVoxelGenerator::new(
        1.0,
        SphereSDFGenerator::new(100.0),
        SameVoxelTypeGenerator::new(VoxelType::default()),
    );
    let object = ChunkedVoxelObject::generate_without_adjacencies(&generator).unwrap();
    c.bench_function("chunked_voxel_object_initialize_adjacencies", |b| {
        b.iter(|| {
            let mut object = object.clone();
            object.initialize_adjacencies();
            black_box(object);
        })
    });
}

pub fn bench_chunked_voxel_object_for_each_exposed_chunk_with_sdf(c: &mut Criterion) {
    let generator = SDFVoxelGenerator::new(
        1.0,
        SphereSDFGenerator::new(100.0),
        SameVoxelTypeGenerator::new(VoxelType::default()),
    );
    let object = ChunkedVoxelObject::generate(&generator).unwrap();
    c.bench_function(
        "bench_chunked_voxel_object_for_each_exposed_chunk_with_sdf",
        |b| {
            b.iter(|| {
                let mut count = 0;
                object.for_each_exposed_chunk_with_sdf(&mut |chunk, sdf| {
                    black_box(chunk);
                    black_box(sdf);
                    count += 1;
                });
                black_box(count);
            })
        },
    );
}

pub fn bench_chunked_voxel_object_create_mesh(c: &mut Criterion) {
    let generator = SDFVoxelGenerator::new(
        1.0,
        SphereSDFGenerator::new(100.0),
        SameVoxelTypeGenerator::new(VoxelType::default()),
    );
    let object = ChunkedVoxelObject::generate(&generator).unwrap();
    c.bench_function("bench_chunked_voxel_object_create_mesh", |b| {
        b.iter(|| {
            black_box(ChunkedVoxelObjectMesh::create(&object));
        })
    });
}

pub fn bench_chunked_voxel_object_modify_voxels_within_sphere(c: &mut Criterion) {
    let object_radius = 100.0;
    let sphere_radius = 0.15 * object_radius;
    let generator = SDFVoxelGenerator::new(
        1.0,
        SphereSDFGenerator::new(object_radius),
        SameVoxelTypeGenerator::new(VoxelType::default()),
    );
    let mut object = ChunkedVoxelObject::generate(&generator).unwrap();
    let sphere = Sphere::new(
        object.compute_aabb::<f64>().center()
            - UnitVector3::new_normalize(vector![1.0, 1.0, 1.0]).scale(object_radius),
        sphere_radius,
    );
    c.bench_function(
        "bench_chunked_voxel_object_modify_voxels_within_sphere",
        |b| {
            b.iter(|| {
                object.modify_voxels_within_sphere(&sphere, &mut |indices, position, voxel| {
                    black_box((indices, position, voxel));
                });
            })
        },
    );
}

criterion_group!(
    name = benches;
    config = Criterion::default().with_profiler(PProfProfiler::new(100, Output::Flamegraph(None)));
    targets =
        bench_chunked_voxel_object_construction,
        bench_chunked_voxel_object_get_each_voxel,
        bench_chunked_voxel_object_initialize_adjacencies,
        bench_chunked_voxel_object_for_each_exposed_chunk_with_sdf,
        bench_chunked_voxel_object_create_mesh,
        bench_chunked_voxel_object_modify_voxels_within_sphere,
);
criterion_main!(benches);
