use criterion::{black_box, criterion_group, criterion_main, Criterion};
use impact::{
    geometry::Sphere,
    voxel::{
        chunks::{sdf::VoxelChunkSignedDistanceField, ChunkedVoxelObject},
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
    let generator = SDFVoxelGenerator::new(
        1.0,
        BoxSDFGenerator::new([200.0; 3]),
        SameVoxelTypeGenerator::new(VoxelType::default()),
    );
    c.bench_function("chunked_voxel_object_construction", |b| {
        b.iter(|| {
            ChunkedVoxelObject::generate_without_derived_state(&generator).unwrap();
        })
    });
}

pub fn bench_chunked_voxel_object_clone(c: &mut Criterion) {
    let generator = SDFVoxelGenerator::new(
        1.0,
        SphereSDFGenerator::new(100.0),
        SameVoxelTypeGenerator::new(VoxelType::default()),
    );
    let object = ChunkedVoxelObject::generate(&generator).unwrap();
    c.bench_function("bench_chunked_voxel_object_clone", |b| {
        b.iter(|| {
            black_box(object.clone());
        })
    });
}

pub fn bench_chunked_voxel_object_get_each_voxel(c: &mut Criterion) {
    let generator = SDFVoxelGenerator::new(
        1.0,
        SphereSDFGenerator::new(100.0),
        SameVoxelTypeGenerator::new(VoxelType::default()),
    );
    let object = ChunkedVoxelObject::generate_without_derived_state(&generator).unwrap();
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

pub fn bench_chunked_voxel_object_update_internal_adjacencies_for_all_chunks(c: &mut Criterion) {
    let generator = SDFVoxelGenerator::new(
        1.0,
        SphereSDFGenerator::new(100.0),
        SameVoxelTypeGenerator::new(VoxelType::default()),
    );
    let mut object = ChunkedVoxelObject::generate_without_derived_state(&generator).unwrap();
    c.bench_function(
        "bench_chunked_voxel_object_update_internal_adjacencies_for_all_chunks",
        |b| {
            b.iter(|| {
                object.update_internal_adjacencies_for_all_chunks();
            })
        },
    );
}

pub fn bench_chunked_voxel_object_update_connected_regions_for_all_chunks(c: &mut Criterion) {
    let generator = SDFVoxelGenerator::new(
        1.0,
        SphereSDFGenerator::new(100.0),
        SameVoxelTypeGenerator::new(VoxelType::default()),
    );
    let mut object = ChunkedVoxelObject::generate_without_derived_state(&generator).unwrap();
    object.update_internal_adjacencies_for_all_chunks();
    c.bench_function(
        "bench_chunked_voxel_object_update_connected_regions_for_all_chunks",
        |b| {
            b.iter(|| {
                object.update_local_connected_regions_for_all_chunks();
            })
        },
    );
}

pub fn bench_chunked_voxel_object_update_all_chunk_boundary_adjacencies(c: &mut Criterion) {
    let generator = SDFVoxelGenerator::new(
        1.0,
        SphereSDFGenerator::new(100.0),
        SameVoxelTypeGenerator::new(VoxelType::default()),
    );
    let mut object = ChunkedVoxelObject::generate_without_derived_state(&generator).unwrap();
    object.update_internal_adjacencies_for_all_chunks();
    object.update_local_connected_regions_for_all_chunks();
    c.bench_function(
        "bench_chunked_voxel_object_update_all_chunk_boundary_adjacencies",
        |b| {
            b.iter(|| {
                object.update_all_chunk_boundary_adjacencies();
            })
        },
    );
}

pub fn bench_chunked_voxel_object_resolve_connected_regions_between_all_chunks(c: &mut Criterion) {
    let generator = SDFVoxelGenerator::new(
        1.0,
        SphereSDFGenerator::new(100.0),
        SameVoxelTypeGenerator::new(VoxelType::default()),
    );
    let mut object = ChunkedVoxelObject::generate_without_derived_state(&generator).unwrap();
    object.update_internal_adjacencies_for_all_chunks();
    object.update_local_connected_regions_for_all_chunks();
    object.update_all_chunk_boundary_adjacencies();
    c.bench_function(
        "bench_chunked_voxel_object_resolve_connected_regions_between_all_chunks",
        |b| {
            b.iter(|| {
                object.resolve_connected_regions_between_all_chunks();
                black_box(object.find_two_disconnected_regions());
            })
        },
    );
}

pub fn bench_chunked_voxel_object_compute_all_derived_state(c: &mut Criterion) {
    let generator = SDFVoxelGenerator::new(
        1.0,
        SphereSDFGenerator::new(100.0),
        SameVoxelTypeGenerator::new(VoxelType::default()),
    );
    let mut object = ChunkedVoxelObject::generate_without_derived_state(&generator).unwrap();
    c.bench_function(
        "bench_chunked_voxel_object_compute_all_derived_state",
        |b| {
            b.iter(|| {
                object.compute_all_derived_state();
            })
        },
    );
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
                let mut sdf = VoxelChunkSignedDistanceField::default();
                object.for_each_exposed_chunk_with_sdf(&mut sdf, &mut |chunk, sdf| {
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

pub fn bench_chunked_voxel_object_update_mesh(c: &mut Criterion) {
    let object_radius = 100.0;
    let sphere_radius = 0.15 * object_radius;
    let generator = SDFVoxelGenerator::new(
        1.0,
        SphereSDFGenerator::new(object_radius),
        SameVoxelTypeGenerator::new(VoxelType::default()),
    );
    let mut object = ChunkedVoxelObject::generate(&generator).unwrap();
    let mut mesh = ChunkedVoxelObjectMesh::create(&object);

    let sphere = Sphere::new(
        object.compute_aabb::<f64>().center()
            - UnitVector3::new_normalize(vector![1.0, 1.0, 1.0]).scale(object_radius),
        sphere_radius,
    );

    c.bench_function("bench_chunked_voxel_object_update_mesh", |b| {
        b.iter(|| {
            object.modify_voxels_within_sphere(&sphere, &mut |indices, position, voxel| {
                black_box((indices, position, voxel));
            });
            mesh.sync_with_voxel_object(&mut object);
            black_box((&object, &mesh));
        })
    });
}

criterion_group!(
    name = benches;
    config = Criterion::default().with_profiler(PProfProfiler::new(100, Output::Flamegraph(None)));
    targets =
        bench_chunked_voxel_object_construction,
        bench_chunked_voxel_object_clone,
        bench_chunked_voxel_object_get_each_voxel,
        bench_chunked_voxel_object_update_internal_adjacencies_for_all_chunks,
        bench_chunked_voxel_object_update_connected_regions_for_all_chunks,
        bench_chunked_voxel_object_update_all_chunk_boundary_adjacencies,
        bench_chunked_voxel_object_resolve_connected_regions_between_all_chunks,
        bench_chunked_voxel_object_compute_all_derived_state,
        bench_chunked_voxel_object_for_each_exposed_chunk_with_sdf,
        bench_chunked_voxel_object_create_mesh,
        bench_chunked_voxel_object_modify_voxels_within_sphere,
        bench_chunked_voxel_object_update_mesh,
);
criterion_main!(benches);
