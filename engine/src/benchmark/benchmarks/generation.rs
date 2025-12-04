//! Benchmarks for SDF generation.

use super::benchmark_data_path;
use bumpalo::Bump;
use impact_profiling::benchmark::Benchmarker;
use impact_voxel::{
    chunks::ChunkedVoxelObject,
    generation::{
        SDFVoxelGenerator, VoxelGenerator,
        sdf::{BoxSDF, SDFGraph, SDFNode},
        voxel_type::{GradientNoiseVoxelTypeGenerator, SameVoxelTypeGenerator},
    },
    voxel_types::VoxelType,
};
use nalgebra::{UnitQuaternion, Vector3, vector};
use std::hint::black_box;

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
