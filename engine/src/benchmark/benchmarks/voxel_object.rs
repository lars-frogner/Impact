//! Benchmarks for chunked voxel object functionality.

use impact_geometry::{Plane, Sphere};
use impact_physics::quantities::Position;
use impact_profiling::benchmark::Benchmarker;
use impact_voxel::{
    chunks::{ChunkedVoxelObject, inertia::VoxelObjectInertialPropertyManager},
    collidable,
    generation::{
        SDFVoxelGenerator,
        sdf::{SDFGraph, SDFNode, SphereSDF},
        voxel_type::SameVoxelTypeGenerator,
    },
    mesh::ChunkedVoxelObjectMesh,
    voxel_types::VoxelType,
};
use nalgebra::{Isometry3, Translation, UnitQuaternion, UnitVector3, Vector3, vector};
use std::hint::black_box;

pub fn update_internal_adjacencies_for_all_chunks(benchmarker: impl Benchmarker) {
    let generator = SDFVoxelGenerator::new(
        1.0,
        SphereSDF::new(100.0).into(),
        SameVoxelTypeGenerator::new(VoxelType::default()).into(),
    );
    let mut object = ChunkedVoxelObject::generate_without_derived_state(&generator);
    benchmarker.benchmark(&mut || {
        object.update_internal_adjacencies_for_all_chunks();
    });
    black_box(object);
}

pub fn update_connected_regions_for_all_chunks(benchmarker: impl Benchmarker) {
    let generator = SDFVoxelGenerator::new(
        1.0,
        SphereSDF::new(100.0).into(),
        SameVoxelTypeGenerator::new(VoxelType::default()).into(),
    );
    let mut object = ChunkedVoxelObject::generate_without_derived_state(&generator);
    object.update_internal_adjacencies_for_all_chunks();
    benchmarker.benchmark(&mut || {
        object.update_local_connected_regions_for_all_chunks();
    });
    black_box(object);
}

pub fn update_all_chunk_boundary_adjacencies(benchmarker: impl Benchmarker) {
    let generator = SDFVoxelGenerator::new(
        1.0,
        SphereSDF::new(100.0).into(),
        SameVoxelTypeGenerator::new(VoxelType::default()).into(),
    );
    let mut object = ChunkedVoxelObject::generate_without_derived_state(&generator);
    object.update_internal_adjacencies_for_all_chunks();
    object.update_local_connected_regions_for_all_chunks();
    benchmarker.benchmark(&mut || {
        object.update_all_chunk_boundary_adjacencies();
    });
    black_box(object);
}

pub fn resolve_connected_regions_between_all_chunks(benchmarker: impl Benchmarker) {
    let generator = SDFVoxelGenerator::new(
        1.0,
        SphereSDF::new(100.0).into(),
        SameVoxelTypeGenerator::new(VoxelType::default()).into(),
    );
    let mut object = ChunkedVoxelObject::generate_without_derived_state(&generator);
    object.update_internal_adjacencies_for_all_chunks();
    object.update_local_connected_regions_for_all_chunks();
    object.update_all_chunk_boundary_adjacencies();
    benchmarker.benchmark(&mut || {
        object.resolve_connected_regions_between_all_chunks();
    });
    black_box(object);
}

pub fn update_occupied_voxel_ranges(benchmarker: impl Benchmarker) {
    let generator = SDFVoxelGenerator::new(
        1.0,
        SphereSDF::new(100.0).into(),
        SameVoxelTypeGenerator::new(VoxelType::default()).into(),
    );
    let mut object = ChunkedVoxelObject::generate_without_derived_state(&generator);
    benchmarker.benchmark(&mut || {
        object.update_occupied_voxel_ranges();
    });
    black_box(object);
}

pub fn compute_all_derived_state(benchmarker: impl Benchmarker) {
    let generator = SDFVoxelGenerator::new(
        1.0,
        SphereSDF::new(100.0).into(),
        SameVoxelTypeGenerator::new(VoxelType::default()).into(),
    );
    let mut object = ChunkedVoxelObject::generate_without_derived_state(&generator);
    benchmarker.benchmark(&mut || {
        object.compute_all_derived_state();
    });
    black_box(object);
}

pub fn initialize_inertial_properties(benchmarker: impl Benchmarker) {
    let generator = SDFVoxelGenerator::new(
        1.0,
        SphereSDF::new(100.0).into(),
        SameVoxelTypeGenerator::new(VoxelType::default()).into(),
    );
    let voxel_type_densities = [1.0; 256];
    let object = ChunkedVoxelObject::generate_without_derived_state(&generator);
    benchmarker.benchmark(&mut || {
        VoxelObjectInertialPropertyManager::initialized_from(&object, &voxel_type_densities)
    });
}

pub fn create_mesh(benchmarker: impl Benchmarker) {
    let generator = SDFVoxelGenerator::new(
        1.0,
        SphereSDF::new(100.0).into(),
        SameVoxelTypeGenerator::new(VoxelType::default()).into(),
    );
    let object = ChunkedVoxelObject::generate(&generator);
    benchmarker.benchmark(&mut || ChunkedVoxelObjectMesh::create(&object));
}

pub fn obtain_surface_voxels_within_negative_halfspace_of_plane(benchmarker: impl Benchmarker) {
    let object_radius = 100.0;
    let plane_displacement = 0.4 * object_radius;
    let generator = SDFVoxelGenerator::new(
        1.0,
        SphereSDF::new(object_radius as f32).into(),
        SameVoxelTypeGenerator::new(VoxelType::default()).into(),
    );
    let object = ChunkedVoxelObject::generate(&generator);
    let plane = Plane::new(
        UnitVector3::new_normalize(vector![1.0, 1.0, 1.0]),
        plane_displacement,
    );
    benchmarker.benchmark(&mut || {
        object.for_each_surface_voxel_maybe_intersecting_negative_halfspace_of_plane(
            &plane,
            &mut |indices, position, voxel| {
                black_box((indices, position, voxel));
            },
        );
    });
}

pub fn obtain_surface_voxels_within_sphere(benchmarker: impl Benchmarker) {
    let object_radius = 100.0;
    let sphere_radius = 0.15 * object_radius;
    let generator = SDFVoxelGenerator::new(
        1.0,
        SphereSDF::new(object_radius as f32).into(),
        SameVoxelTypeGenerator::new(VoxelType::default()).into(),
    );
    let object = ChunkedVoxelObject::generate(&generator);
    let sphere = Sphere::new(
        object.compute_aabb::<f64>().center()
            - UnitVector3::new_normalize(vector![1.0, 1.0, 1.0]).scale(object_radius),
        sphere_radius,
    );
    benchmarker.benchmark(&mut || {
        object.for_each_surface_voxel_maybe_intersecting_sphere(
            &sphere,
            &mut |indices, position, voxel| {
                black_box((indices, position, voxel));
            },
        );
    });
}

pub fn modify_voxels_within_sphere(benchmarker: impl Benchmarker) {
    let object_radius = 100.0;
    let sphere_radius = 0.15 * object_radius;
    let generator = SDFVoxelGenerator::new(
        1.0,
        SphereSDF::new(object_radius as f32).into(),
        SameVoxelTypeGenerator::new(VoxelType::default()).into(),
    );
    let mut object = ChunkedVoxelObject::generate(&generator);
    let sphere = Sphere::new(
        object.compute_aabb::<f64>().center()
            - UnitVector3::new_normalize(vector![1.0, 1.0, 1.0]).scale(object_radius),
        sphere_radius,
    );
    benchmarker.benchmark(&mut || {
        object.modify_voxels_within_sphere(&sphere, &mut |indices, position, voxel| {
            black_box((indices, position, voxel));
        });
    });
}

pub fn split_off_disconnected_region(benchmarker: impl Benchmarker) {
    let mut graph = SDFGraph::new();
    let sphere_1_id = graph.add_node(SDFNode::new_sphere(50.0));
    let sphere_2_id = graph.add_node(SDFNode::new_sphere(50.0));
    let sphere_2_id = graph.add_node(SDFNode::new_translation(
        sphere_2_id,
        vector![120.0, 0.0, 0.0],
    ));
    graph.add_node(SDFNode::new_union(sphere_1_id, sphere_2_id, 1.0));
    let sdf_generator = graph.build().unwrap();

    let generator = SDFVoxelGenerator::new(
        1.0,
        sdf_generator,
        SameVoxelTypeGenerator::new(VoxelType::default()).into(),
    );
    let object = ChunkedVoxelObject::generate(&generator);
    benchmarker.benchmark(&mut || object.clone().split_off_any_disconnected_region().unwrap());
}

pub fn split_off_disconnected_region_with_inertial_property_transfer(
    benchmarker: impl Benchmarker,
) {
    let mut graph = SDFGraph::new();
    let sphere_1_id = graph.add_node(SDFNode::new_sphere(50.0));
    let sphere_2_id = graph.add_node(SDFNode::new_sphere(50.0));
    let sphere_2_id = graph.add_node(SDFNode::new_translation(
        sphere_2_id,
        vector![120.0, 0.0, 0.0],
    ));
    graph.add_node(SDFNode::new_union(sphere_1_id, sphere_2_id, 1.0));
    let sdf_generator = graph.build().unwrap();

    let generator = SDFVoxelGenerator::new(
        1.0,
        sdf_generator,
        SameVoxelTypeGenerator::new(VoxelType::default()).into(),
    );
    let voxel_type_densities = [1.0; 256];
    let object = ChunkedVoxelObject::generate(&generator);
    let inertial_property_manager =
        VoxelObjectInertialPropertyManager::initialized_from(&object, &voxel_type_densities);
    benchmarker.benchmark(&mut || {
        let mut inertial_property_manager = inertial_property_manager.clone();
        let mut disconnected_inertial_property_manager =
            VoxelObjectInertialPropertyManager::zeroed();
        let mut inertial_property_transferrer = inertial_property_manager.begin_transfer_to(
            &mut disconnected_inertial_property_manager,
            object.voxel_extent(),
            &voxel_type_densities,
        );
        let disconnected_object = object
            .clone()
            .split_off_any_disconnected_region_with_property_transferrer(
                &mut inertial_property_transferrer,
            )
            .unwrap();
        (
            disconnected_object,
            inertial_property_manager,
            disconnected_inertial_property_manager,
        )
    });
}

pub fn update_mesh(benchmarker: impl Benchmarker) {
    let object_radius = 100.0;
    let sphere_radius = 0.15 * object_radius;
    let generator = SDFVoxelGenerator::new(
        1.0,
        SphereSDF::new(object_radius as f32).into(),
        SameVoxelTypeGenerator::new(VoxelType::default()).into(),
    );
    let mut object = ChunkedVoxelObject::generate(&generator);
    let mut mesh = ChunkedVoxelObjectMesh::create(&object);

    let sphere = Sphere::new(
        object.compute_aabb::<f64>().center()
            - UnitVector3::new_normalize(vector![1.0, 1.0, 1.0]).scale(object_radius),
        sphere_radius,
    );

    benchmarker.benchmark(&mut || {
        object.modify_voxels_within_sphere(&sphere, &mut |indices, position, voxel| {
            black_box((indices, position, voxel));
        });
        mesh.sync_with_voxel_object(&mut object);
        black_box((&object, &mesh));
    });
}

pub fn obtain_sphere_voxel_object_contacts(benchmarker: impl Benchmarker) {
    let object_radius = 100.0;
    let sphere_radius = 0.15 * object_radius;
    let generator = SDFVoxelGenerator::new(
        1.0,
        SphereSDF::new(object_radius as f32).into(),
        SameVoxelTypeGenerator::new(VoxelType::default()).into(),
    );
    let object = ChunkedVoxelObject::generate(&generator);
    let sphere = Sphere::new(Position::origin(), sphere_radius);
    let transform_to_object_space = Isometry3::from_parts(
        Translation::from(
            object.compute_aabb::<f64>().center()
                - UnitVector3::new_normalize(vector![1.0, 1.0, 1.0]).scale(object_radius),
        ),
        UnitQuaternion::from_axis_angle(&Vector3::z_axis(), 1.0),
    );
    benchmarker.benchmark(&mut || {
        collidable::for_each_sphere_voxel_object_contact(
            &object,
            &transform_to_object_space,
            &sphere,
            &mut |indices, geometry| {
                black_box((indices, geometry));
            },
        );
    });
}
