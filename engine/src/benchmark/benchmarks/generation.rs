//! Benchmarks for SDF generation.

use super::benchmark_data_path;
use bumpalo::Bump;
use impact_profiling::benchmark::Benchmarker;
use impact_thread::rayon::RayonThreadPool;
use impact_voxel::{
    chunks::{CHUNK_SIZE, ChunkedVoxelObject},
    generation::{
        SDFVoxelGenerator, VoxelGenerator,
        sdf::{BoxSDF, SDFGenerator, SDFGraph, SDFNode, SphereSDF},
        voxel_type::{GradientNoiseVoxelTypeGenerator, SameVoxelTypeGenerator},
    },
    voxel_types::VoxelType,
};
use nalgebra::{Matrix4, Point3, UnitQuaternion, Vector3, vector};
use std::{hint::black_box, num::NonZeroUsize};

pub fn generate_box(benchmarker: impl Benchmarker) {
    let generator = SDFVoxelGenerator::new(
        1.0,
        BoxSDF::new([80.0; 3]).into(),
        SameVoxelTypeGenerator::new(VoxelType::default()).into(),
    );
    benchmarker.benchmark(&mut || ChunkedVoxelObject::generate_without_derived_state(&generator));
}

pub fn generate_sphere_union(benchmarker: impl Benchmarker) {
    let mut graph = SDFGraph::new();
    let sphere_1_id = graph.add_node(SDFNode::new_sphere(60.0));
    let sphere_2_id = graph.add_node(SDFNode::new_sphere(60.0));
    let sphere_2_id = graph.add_node(SDFNode::new_translation(
        sphere_2_id,
        vector![50.0, 0.0, 0.0],
    ));
    graph.add_node(SDFNode::new_union(sphere_1_id, sphere_2_id, 1.0));
    let sdf_generator = graph.build().unwrap();

    let generator = SDFVoxelGenerator::new(
        1.0,
        sdf_generator,
        SameVoxelTypeGenerator::new(VoxelType::default()).into(),
    );
    benchmarker.benchmark(&mut || ChunkedVoxelObject::generate_without_derived_state(&generator));
}

pub fn generate_complex_object(benchmarker: impl Benchmarker) {
    let mut graph = SDFGraph::new();
    let sphere_id = graph.add_node(SDFNode::new_sphere(60.0));
    let sphere_id = graph.add_node(SDFNode::new_translation(sphere_id, vector![50.0, 0.0, 0.0]));
    let box_id = graph.add_node(SDFNode::new_box([50.0, 60.0, 70.0]));
    let box_id = graph.add_node(SDFNode::new_scaling(box_id, 0.9));
    let box_id = graph.add_node(SDFNode::new_rotation(
        box_id,
        UnitQuaternion::from_axis_angle(&Vector3::y_axis(), 10.0),
    ));
    graph.add_node(SDFNode::new_union(sphere_id, box_id, 1.0));
    let sdf_generator = graph.build().unwrap();

    let generator = SDFVoxelGenerator::new(
        1.0,
        sdf_generator,
        SameVoxelTypeGenerator::new(VoxelType::default()).into(),
    );
    benchmarker.benchmark(&mut || ChunkedVoxelObject::generate_without_derived_state(&generator));
}

pub fn generate_object_with_multifractal_noise(benchmarker: impl Benchmarker) {
    let mut graph = SDFGraph::new();
    let sphere_id = graph.add_node(SDFNode::new_sphere(80.0));
    graph.add_node(SDFNode::new_multifractal_noise(
        sphere_id, 8, 0.02, 2.0, 0.6, 4.0, 0,
    ));
    let sdf_generator = graph.build().unwrap();

    let generator = SDFVoxelGenerator::new(
        1.0,
        sdf_generator,
        SameVoxelTypeGenerator::new(VoxelType::default()).into(),
    );
    benchmarker.benchmark(&mut || ChunkedVoxelObject::generate_without_derived_state(&generator));
}

pub fn generate_object_with_multiscale_spheres(benchmarker: impl Benchmarker) {
    let mut graph = SDFGraph::new();
    let sphere_id = graph.add_node(SDFNode::new_sphere(40.0));
    graph.add_node(SDFNode::new_multiscale_sphere(
        sphere_id, 4, 10.0, 0.5, 1.0, 1.0, 0.3, 0,
    ));
    let sdf_generator = graph.build().unwrap();

    let generator = SDFVoxelGenerator::new(
        1.0,
        sdf_generator,
        SameVoxelTypeGenerator::new(VoxelType::default()).into(),
    );
    benchmarker.benchmark(&mut || ChunkedVoxelObject::generate_without_derived_state(&generator));
}

pub fn generate_box_with_gradient_noise_voxel_types(benchmarker: impl Benchmarker) {
    let generator = SDFVoxelGenerator::new(
        1.0,
        BoxSDF::new([80.0; 3]).into(),
        GradientNoiseVoxelTypeGenerator::new(
            vec![
                VoxelType::from_idx(0),
                VoxelType::from_idx(1),
                VoxelType::from_idx(2),
                VoxelType::from_idx(3),
            ],
            0.02,
            1.0,
            0,
        )
        .into(),
    );
    benchmarker.benchmark(&mut || ChunkedVoxelObject::generate_without_derived_state(&generator));
}

pub fn compile_complex_meta_graph(benchmarker: impl Benchmarker) {
    let mut arena = Bump::new();

    let generator: VoxelGenerator =
        impact_io::parse_ron_file(benchmark_data_path("asteroid.vgen.ron")).unwrap();

    benchmarker.benchmark(&mut || {
        black_box(generator.sdf_graph.build(&arena, 0).unwrap());
        arena.reset();
    });
}

pub fn build_complex_atomic_graph(benchmarker: impl Benchmarker) {
    let generator: VoxelGenerator =
        impact_io::parse_ron_file(benchmark_data_path("asteroid.vgen.ron")).unwrap();

    let arena_for_meta = Bump::new();
    let atomic_graph = generator.sdf_graph.build(&arena_for_meta, 0).unwrap();

    let mut arena = Bump::new();

    benchmarker.benchmark(&mut || {
        black_box(atomic_graph.build_with_arena(&arena).unwrap());
        arena.reset();
    });
}

pub fn generate_object_from_complex_graph(benchmarker: impl Benchmarker) {
    let generator: VoxelGenerator =
        impact_io::parse_ron_file(benchmark_data_path("asteroid.vgen.ron")).unwrap();

    let arena = Bump::new();
    let atomic_graph = generator.sdf_graph.build(&arena, 0).unwrap();
    let sdf_generator = atomic_graph.build_with_arena(&arena).unwrap();

    let generator = SDFVoxelGenerator::new(
        1.0,
        sdf_generator,
        SameVoxelTypeGenerator::new(VoxelType::default()).into(),
    );

    let thread_pool = RayonThreadPool::new(NonZeroUsize::new(16).unwrap());

    benchmarker.benchmark(&mut || {
        black_box(
            ChunkedVoxelObject::generate_without_derived_state_in_parallel(
                &thread_pool,
                &generator,
            ),
        );
    });
}

pub fn update_signed_distances_for_block(benchmarker: impl Benchmarker) {
    let sphere_sdf = &SphereSDF::new(8.0);
    let sdf_generator = SDFGenerator::from(sphere_sdf.clone());
    let mut buffers = sdf_generator.create_buffers_for_chunk();

    let transform = Matrix4::identity();
    let origin = Point3::origin();

    const COUNT: usize = ChunkedVoxelObject::chunk_voxel_count();

    benchmarker.benchmark(&mut || {
        impact_voxel::generation::sdf::atomic::update_signed_distances_for_block::<
            CHUNK_SIZE,
            COUNT,
        >(
            black_box(&mut buffers.signed_distance_stack[0]),
            black_box(&transform),
            black_box(&origin),
            &|signed_distance, position| {
                *signed_distance = sphere_sdf.compute_signed_distance(position);
            },
        );
    });
}
