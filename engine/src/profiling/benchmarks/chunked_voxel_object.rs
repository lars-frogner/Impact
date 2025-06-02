//! Benchmarks for chunked voxel object functionality.

use crate::{
    scene::RenderResourcesDesynchronized,
    voxel::{
        chunks::{ChunkedVoxelObject, inertia::VoxelObjectInertialPropertyManager},
        generation::{
            BoxSDFGenerator, SDFUnion, SDFVoxelGenerator, SameVoxelTypeGenerator,
            SphereSDFGenerator,
        },
        mesh::ChunkedVoxelObjectMesh,
        voxel_types::VoxelType,
    },
};
use impact_geometry::Sphere;
use impact_profiling::Profiler;
use nalgebra::{UnitVector3, vector};
use std::hint::black_box;

pub fn construction(profiler: impl Profiler) {
    let generator = SDFVoxelGenerator::new(
        1.0,
        BoxSDFGenerator::new([200.0; 3]),
        SameVoxelTypeGenerator::new(VoxelType::default()),
    );
    profiler
        .profile(&mut || ChunkedVoxelObject::generate_without_derived_state(&generator).unwrap());
}

pub fn update_internal_adjacencies_for_all_chunks(profiler: impl Profiler) {
    let generator = SDFVoxelGenerator::new(
        1.0,
        SphereSDFGenerator::new(100.0),
        SameVoxelTypeGenerator::new(VoxelType::default()),
    );
    let mut object = ChunkedVoxelObject::generate_without_derived_state(&generator).unwrap();
    profiler.profile(&mut || {
        object.update_internal_adjacencies_for_all_chunks();
    });
    black_box(object);
}

pub fn update_connected_regions_for_all_chunks(profiler: impl Profiler) {
    let generator = SDFVoxelGenerator::new(
        1.0,
        SphereSDFGenerator::new(100.0),
        SameVoxelTypeGenerator::new(VoxelType::default()),
    );
    let mut object = ChunkedVoxelObject::generate_without_derived_state(&generator).unwrap();
    object.update_internal_adjacencies_for_all_chunks();
    profiler.profile(&mut || {
        object.update_local_connected_regions_for_all_chunks();
    });
    black_box(object);
}

pub fn update_all_chunk_boundary_adjacencies(profiler: impl Profiler) {
    let generator = SDFVoxelGenerator::new(
        1.0,
        SphereSDFGenerator::new(100.0),
        SameVoxelTypeGenerator::new(VoxelType::default()),
    );
    let mut object = ChunkedVoxelObject::generate_without_derived_state(&generator).unwrap();
    object.update_internal_adjacencies_for_all_chunks();
    object.update_local_connected_regions_for_all_chunks();
    profiler.profile(&mut || {
        object.update_all_chunk_boundary_adjacencies();
    });
    black_box(object);
}

pub fn resolve_connected_regions_between_all_chunks(profiler: impl Profiler) {
    let generator = SDFVoxelGenerator::new(
        1.0,
        SphereSDFGenerator::new(100.0),
        SameVoxelTypeGenerator::new(VoxelType::default()),
    );
    let mut object = ChunkedVoxelObject::generate_without_derived_state(&generator).unwrap();
    object.update_internal_adjacencies_for_all_chunks();
    object.update_local_connected_regions_for_all_chunks();
    object.update_all_chunk_boundary_adjacencies();
    profiler.profile(&mut || {
        object.resolve_connected_regions_between_all_chunks();
    });
    black_box(object);
}

pub fn compute_all_derived_state(profiler: impl Profiler) {
    let generator = SDFVoxelGenerator::new(
        1.0,
        SphereSDFGenerator::new(100.0),
        SameVoxelTypeGenerator::new(VoxelType::default()),
    );
    let mut object = ChunkedVoxelObject::generate_without_derived_state(&generator).unwrap();
    profiler.profile(&mut || {
        object.compute_all_derived_state();
    });
    black_box(object);
}

pub fn initialize_inertial_properties(profiler: impl Profiler) {
    let generator = SDFVoxelGenerator::new(
        1.0,
        SphereSDFGenerator::new(100.0),
        SameVoxelTypeGenerator::new(VoxelType::default()),
    );
    let voxel_type_densities = [1.0; 256];
    let object = ChunkedVoxelObject::generate_without_derived_state(&generator).unwrap();
    profiler.profile(&mut || {
        VoxelObjectInertialPropertyManager::initialized_from(&object, &voxel_type_densities)
    });
}

pub fn create_mesh(profiler: impl Profiler) {
    let generator = SDFVoxelGenerator::new(
        1.0,
        SphereSDFGenerator::new(100.0),
        SameVoxelTypeGenerator::new(VoxelType::default()),
    );
    let object = ChunkedVoxelObject::generate(&generator).unwrap();
    profiler.profile(&mut || ChunkedVoxelObjectMesh::create(&object));
}

pub fn modify_voxels_within_sphere(profiler: impl Profiler) {
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
    profiler.profile(&mut || {
        object.modify_voxels_within_sphere(&sphere, &mut |indices, position, voxel| {
            black_box((indices, position, voxel));
        });
    });
}

pub fn split_off_disconnected_region(profiler: impl Profiler) {
    let generator = SDFVoxelGenerator::new(
        1.0,
        SDFUnion::new(
            SphereSDFGenerator::new(50.0),
            SphereSDFGenerator::new(50.0),
            [120.0, 0.0, 0.0],
            1.0,
        ),
        SameVoxelTypeGenerator::new(VoxelType::default()),
    );
    let object = ChunkedVoxelObject::generate(&generator).unwrap();
    profiler.profile(&mut || object.clone().split_off_any_disconnected_region().unwrap());
}

pub fn split_off_disconnected_region_with_inertial_property_transfer(profiler: impl Profiler) {
    let generator = SDFVoxelGenerator::new(
        1.0,
        SDFUnion::new(
            SphereSDFGenerator::new(50.0),
            SphereSDFGenerator::new(50.0),
            [120.0, 0.0, 0.0],
            1.0,
        ),
        SameVoxelTypeGenerator::new(VoxelType::default()),
    );
    let voxel_type_densities = [1.0; 256];
    let object = ChunkedVoxelObject::generate(&generator).unwrap();
    let inertial_property_manager =
        VoxelObjectInertialPropertyManager::initialized_from(&object, &voxel_type_densities);
    profiler.profile(&mut || {
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

pub fn update_mesh(profiler: impl Profiler) {
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

    profiler.profile(&mut || {
        object.modify_voxels_within_sphere(&sphere, &mut |indices, position, voxel| {
            black_box((indices, position, voxel));
        });
        let mut desynchronized = RenderResourcesDesynchronized::No;
        mesh.sync_with_voxel_object(&mut object, &mut desynchronized);
        black_box((&object, &mesh));
    });
}
