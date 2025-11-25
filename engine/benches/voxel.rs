use impact::benchmark::benchmarks::chunked_voxel_object;
use impact_profiling::{
    benchmark::criterion::{self, Criterion},
    define_criterion_target,
};
use impact_voxel::{
    chunks::{ChunkedVoxelObject, sdf::VoxelChunkSignedDistanceField},
    generation::{SDFVoxelGenerator, sdf::SphereSDF, voxel_type::SameVoxelTypeGenerator},
    voxel_types::VoxelType,
};
use std::hint::black_box;

pub fn clone_object(c: &mut Criterion) {
    let generator = SDFVoxelGenerator::new(
        1.0,
        SphereSDF::new(100.0).into(),
        SameVoxelTypeGenerator::new(VoxelType::default()).into(),
    );
    let object = ChunkedVoxelObject::generate(&generator);
    c.bench_function("clone_object", |b| {
        b.iter(|| {
            black_box(object.clone());
        });
    });
}

pub fn get_each_voxel(c: &mut Criterion) {
    let generator = SDFVoxelGenerator::new(
        1.0,
        SphereSDF::new(100.0).into(),
        SameVoxelTypeGenerator::new(VoxelType::default()).into(),
    );
    let object = ChunkedVoxelObject::generate_without_derived_state(&generator);
    let ranges = object.occupied_voxel_ranges();
    c.bench_function("get_each_voxel", |b| {
        b.iter(|| {
            for i in ranges[0].clone() {
                for j in ranges[1].clone() {
                    for k in ranges[2].clone() {
                        let _ = black_box(object.get_voxel(i, j, k));
                    }
                }
            }
        });
    });
}

pub fn for_each_exposed_chunk_with_sdf(c: &mut Criterion) {
    let generator = SDFVoxelGenerator::new(
        1.0,
        SphereSDF::new(100.0).into(),
        SameVoxelTypeGenerator::new(VoxelType::default()).into(),
    );
    let object = ChunkedVoxelObject::generate(&generator);
    c.bench_function("for_each_exposed_chunk_with_sdf", |b| {
        b.iter(|| {
            let mut count = 0;
            let mut sdf = VoxelChunkSignedDistanceField::default();
            object.for_each_exposed_chunk_with_sdf(&mut sdf, &mut |chunk, sdf| {
                black_box(chunk);
                black_box(sdf);
                count += 1;
            });
            black_box(count);
        });
    });
}

define_criterion_target!(chunked_voxel_object, generate_box);
define_criterion_target!(chunked_voxel_object, generate_sphere_union);
define_criterion_target!(chunked_voxel_object, generate_complex_object);
define_criterion_target!(
    chunked_voxel_object,
    generate_object_with_multifractal_noise
);
define_criterion_target!(
    chunked_voxel_object,
    generate_object_with_multiscale_spheres
);
define_criterion_target!(
    chunked_voxel_object,
    generate_box_with_gradient_noise_voxel_types
);
define_criterion_target!(
    chunked_voxel_object,
    update_internal_adjacencies_for_all_chunks
);
define_criterion_target!(
    chunked_voxel_object,
    update_connected_regions_for_all_chunks
);
define_criterion_target!(chunked_voxel_object, update_all_chunk_boundary_adjacencies);
define_criterion_target!(
    chunked_voxel_object,
    resolve_connected_regions_between_all_chunks
);
define_criterion_target!(chunked_voxel_object, update_occupied_voxel_ranges);
define_criterion_target!(chunked_voxel_object, compute_all_derived_state);
define_criterion_target!(chunked_voxel_object, initialize_inertial_properties);
define_criterion_target!(chunked_voxel_object, create_mesh);
define_criterion_target!(
    chunked_voxel_object,
    obtain_surface_voxels_within_negative_halfspace_of_plane
);
define_criterion_target!(chunked_voxel_object, obtain_surface_voxels_within_sphere);
define_criterion_target!(chunked_voxel_object, modify_voxels_within_sphere);
define_criterion_target!(chunked_voxel_object, split_off_disconnected_region);
define_criterion_target!(
    chunked_voxel_object,
    split_off_disconnected_region_with_inertial_property_transfer
);
define_criterion_target!(chunked_voxel_object, update_mesh);
define_criterion_target!(chunked_voxel_object, obtain_sphere_voxel_object_contacts);

criterion::criterion_group!(
    name = benches;
    config = criterion::config();
    targets =
        generate_box,
        generate_sphere_union,
        generate_complex_object,
        generate_object_with_multifractal_noise,
        generate_object_with_multiscale_spheres,
        generate_box_with_gradient_noise_voxel_types,
        clone_object,
        update_internal_adjacencies_for_all_chunks,
        update_connected_regions_for_all_chunks,
        update_all_chunk_boundary_adjacencies,
        resolve_connected_regions_between_all_chunks,
        update_occupied_voxel_ranges,
        compute_all_derived_state,
        initialize_inertial_properties,
        for_each_exposed_chunk_with_sdf,
        create_mesh,
        obtain_surface_voxels_within_negative_halfspace_of_plane,
        obtain_surface_voxels_within_sphere,
        modify_voxels_within_sphere,
        split_off_disconnected_region,
        split_off_disconnected_region_with_inertial_property_transfer,
        update_mesh,
        obtain_sphere_voxel_object_contacts,
);
criterion::criterion_main!(benches);
