//! Benchmarks for voxel object functionality.

use super::tesselation::create_randomized_grid_points;
use impact_alloc::Global;
use impact_geometry::{AxisAlignedBox, Plane, ReferenceFrame, Sphere};
use impact_id::EntityIDManager;
use impact_intersection::{IntersectionManager, bounding_volume::BoundingVolumeID};
use impact_math::{
    point::Point3C,
    quaternion::UnitQuaternion,
    random::Rng,
    transform::{Isometry3, Similarity3},
    vector::{UnitVector3, Vector3},
};
use impact_physics::{
    collision::CollidableKind,
    material::ContactResponseParameters,
    quantities::Position,
    rigid_body::{RigidBodyManager, RigidBodyType},
};
use impact_profiling::benchmark::Benchmarker;
use impact_tesselation::{delaunay::DelaunayTetrahedralization, voronoi::VoronoiPolyhedron};
use impact_voxel::{
    VoxelObjectID, VoxelObjectManager, VoxelObjectPhysicsContext,
    collidable::{
        self, CollisionWorld, VoxelObjectCollisionProbes,
        setup::{VoxelCollidable, setup_voxel_collidable},
    },
    generation::{
        SDFVoxelGenerator,
        sdf::{SDFGraph, SDFNode},
        voxel_type::SameVoxelTypeGenerator,
    },
    mesh::{MeshedVoxelObject, VoxelObjectMesh, VoxelObjectMeshBuffers},
    object::{
        VoxelObject, VoxelObjectBuffers, extraction::ExtractionResult,
        inertia::VoxelObjectInertialPropertyManager, sdf::VoxelChunkSignedDistanceField,
    },
    setup,
    voxel_types::VoxelType,
};
use std::hint::black_box;

pub fn update_internal_adjacencies_for_all_chunks(benchmarker: impl Benchmarker) {
    let generator = create_sphere_generator(100.0);
    let mut object =
        VoxelObject::generate_without_derived_state(VoxelObjectBuffers::new(), &generator);
    benchmarker.benchmark(&mut || {
        object.update_internal_adjacencies_for_all_chunks();
    });
    black_box(object);
}

pub fn update_connected_regions_for_all_chunks(benchmarker: impl Benchmarker) {
    let generator = create_sphere_generator(100.0);
    let mut object =
        VoxelObject::generate_without_derived_state(VoxelObjectBuffers::new(), &generator);
    object.update_internal_adjacencies_for_all_chunks();
    benchmarker.benchmark(&mut || {
        object.update_local_connected_regions_for_all_chunks();
    });
    black_box(object);
}

pub fn update_all_chunk_boundary_adjacencies(benchmarker: impl Benchmarker) {
    let generator = create_sphere_generator(100.0);
    let mut object =
        VoxelObject::generate_without_derived_state(VoxelObjectBuffers::new(), &generator);
    object.update_internal_adjacencies_for_all_chunks();
    object.update_local_connected_regions_for_all_chunks();
    benchmarker.benchmark(&mut || {
        object.update_all_chunk_boundary_adjacencies();
    });
    black_box(object);
}

pub fn resolve_connected_regions_between_all_chunks(benchmarker: impl Benchmarker) {
    let generator = create_sphere_generator(100.0);
    let mut object =
        VoxelObject::generate_without_derived_state(VoxelObjectBuffers::new(), &generator);
    object.update_internal_adjacencies_for_all_chunks();
    object.update_local_connected_regions_for_all_chunks();
    object.update_all_chunk_boundary_adjacencies();
    benchmarker.benchmark(&mut || {
        object.resolve_connected_regions_between_all_chunks();
    });
    black_box(object);
}

pub fn update_occupied_voxel_ranges(benchmarker: impl Benchmarker) {
    let generator = create_sphere_generator(100.0);
    let mut object =
        VoxelObject::generate_without_derived_state(VoxelObjectBuffers::new(), &generator);
    benchmarker.benchmark(&mut || {
        object.update_occupied_voxel_ranges();
    });
    black_box(object);
}

pub fn compute_all_derived_state(benchmarker: impl Benchmarker) {
    let generator = create_sphere_generator(100.0);
    let mut object =
        VoxelObject::generate_without_derived_state(VoxelObjectBuffers::new(), &generator);
    benchmarker.benchmark(&mut || {
        object.compute_all_derived_state();
    });
    black_box(object);
}

pub fn initialize_inertial_properties(benchmarker: impl Benchmarker) {
    let generator = create_sphere_generator(100.0);
    let voxel_type_densities = [1.0; 256];
    let object = VoxelObject::generate_without_derived_state(VoxelObjectBuffers::new(), &generator);
    benchmarker.benchmark(&mut || {
        VoxelObjectInertialPropertyManager::initialized_from(&object, &voxel_type_densities)
    });
}

pub fn clone_object(benchmarker: impl Benchmarker) {
    let generator = create_sphere_generator(100.0);
    let object = VoxelObject::generate(VoxelObjectBuffers::new(), &generator);
    benchmarker.benchmark(&mut || object.clone());
}

pub fn create_mesh(benchmarker: impl Benchmarker) {
    let generator = create_sphere_generator(100.0);
    let object = VoxelObject::generate(VoxelObjectBuffers::new(), &generator);
    benchmarker.benchmark(&mut || VoxelObjectMesh::create(&object));
}

pub fn compute_collision_probes(benchmarker: impl Benchmarker) {
    let generator = create_sphere_generator(100.0);
    let object = VoxelObject::generate(VoxelObjectBuffers::new(), &generator);
    let mesh = VoxelObjectMesh::create(&object);
    benchmarker
        .benchmark(&mut || VoxelObjectCollisionProbes::compute_for_all_chunks(&object, &mesh));
}

pub fn get_each_voxel(benchmarker: impl Benchmarker) {
    let generator = create_sphere_generator(100.0);
    let object = VoxelObject::generate_without_derived_state(VoxelObjectBuffers::new(), &generator);
    let ranges = object.occupied_voxel_ranges();
    benchmarker.benchmark(&mut || {
        for i in ranges[0].clone() {
            for j in ranges[1].clone() {
                for k in ranges[2].clone() {
                    let _ = black_box(object.get_voxel_if_occupied(i, j, k));
                }
            }
        }
    });
}

pub fn obtain_surface_voxels_within_negative_halfspace_of_plane(benchmarker: impl Benchmarker) {
    let object_radius = 100.0;
    let plane_displacement = 0.4 * object_radius;
    let generator = create_sphere_generator(object_radius);
    let object = VoxelObject::generate(VoxelObjectBuffers::new(), &generator);
    let plane = Plane::new(
        UnitVector3::normalized_from(Vector3::same(1.0)),
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
    let generator = create_sphere_generator(object_radius);
    let object = VoxelObject::generate(VoxelObjectBuffers::new(), &generator);
    let sphere = Sphere::new(
        object.compute_aabb().center()
            - object_radius * UnitVector3::normalized_from(Vector3::same(1.0)),
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

pub fn for_each_exposed_chunk_with_sdf(benchmarker: impl Benchmarker) {
    let generator = create_sphere_generator(100.0);
    let object = VoxelObject::generate(VoxelObjectBuffers::new(), &generator);
    benchmarker.benchmark(&mut || {
        let mut count = 0;
        let mut sdf = VoxelChunkSignedDistanceField::new();
        object.for_each_exposed_chunk_with_sdf(&mut sdf, &mut |chunk, sdf| {
            black_box(chunk);
            black_box(sdf);
            count += 1;
        });
        black_box(count);
    });
}

pub fn modify_voxels_within_sphere(benchmarker: impl Benchmarker) {
    let object_radius = 100.0;
    let sphere_radius = 0.15 * object_radius;
    let generator = create_sphere_generator(object_radius);
    let mut object = VoxelObject::generate(VoxelObjectBuffers::new(), &generator);
    let sphere = Sphere::new(
        object.compute_aabb().center()
            - object_radius * UnitVector3::normalized_from(Vector3::same(1.0)),
        sphere_radius,
    );
    benchmarker.benchmark(&mut || {
        object.modify_voxels_within_sphere(&sphere, &mut |indices, position, voxel| {
            black_box((indices, position, voxel));
        });
    });
}

pub fn split_off_disconnected_region(benchmarker: impl Benchmarker) {
    let mut graph = SDFGraph::new_in(Global);
    let sphere_1_id = graph.add_node(SDFNode::new_sphere(50.0));
    let sphere_2_id = graph.add_node(SDFNode::new_sphere(50.0));
    let sphere_2_id = graph.add_node(SDFNode::new_translation(
        sphere_2_id,
        Vector3::new(120.0, 0.0, 0.0),
    ));
    graph.add_node(SDFNode::new_union(sphere_1_id, sphere_2_id, 1.0));
    let sdf_generator = graph.build_in(Global).unwrap();

    let generator = SDFVoxelGenerator::new(
        1.0,
        sdf_generator,
        SameVoxelTypeGenerator::new(VoxelType::default()).into(),
    );
    let object = VoxelObject::generate(VoxelObjectBuffers::new(), &generator);
    benchmarker.benchmark(&mut || {
        object
            .clone()
            .extract_any_disconnected_region(VoxelObjectBuffers::new())
    });
}

pub fn split_off_disconnected_region_with_inertial_property_transfer(
    benchmarker: impl Benchmarker,
) {
    let mut graph = SDFGraph::new_in(Global);
    let sphere_1_id = graph.add_node(SDFNode::new_sphere(50.0));
    let sphere_2_id = graph.add_node(SDFNode::new_sphere(50.0));
    let sphere_2_id = graph.add_node(SDFNode::new_translation(
        sphere_2_id,
        Vector3::new(120.0, 0.0, 0.0),
    ));
    graph.add_node(SDFNode::new_union(sphere_1_id, sphere_2_id, 1.0));
    let sdf_generator = graph.build_in(Global).unwrap();

    let generator = SDFVoxelGenerator::new(
        1.0,
        sdf_generator,
        SameVoxelTypeGenerator::new(VoxelType::default()).into(),
    );
    let object = VoxelObject::generate(VoxelObjectBuffers::new(), &generator);

    let voxel_type_densities = [1.0; 256];
    let inertial_property_manager =
        VoxelObjectInertialPropertyManager::initialized_from(&object, &voxel_type_densities);

    benchmarker.benchmark(&mut || {
        let mut object = object.clone();
        let mut inertial_property_manager = inertial_property_manager.clone();

        let mut disconnected_inertial_property_manager =
            VoxelObjectInertialPropertyManager::zeroed();

        let mut inertial_property_transferrer = inertial_property_manager.begin_transfer_to(
            &mut disconnected_inertial_property_manager,
            object.voxel_extent(),
            &voxel_type_densities,
        );

        let result = object.extract_any_disconnected_region_with_property_transferrer(
            VoxelObjectBuffers::new(),
            &mut inertial_property_transferrer,
        );
        (
            result,
            inertial_property_manager,
            disconnected_inertial_property_manager,
        )
    });
}

pub fn update_mesh(benchmarker: impl Benchmarker) {
    let object_radius = 100.0;
    let sphere_radius = 0.15 * object_radius;
    let generator = create_sphere_generator(object_radius);
    let mut object = VoxelObject::generate(VoxelObjectBuffers::new(), &generator);
    let mut mesh = VoxelObjectMesh::create(&object);

    let sphere = Sphere::new(
        object.compute_aabb().center()
            - object_radius * UnitVector3::normalized_from(Vector3::same(1.0)),
        sphere_radius,
    );

    benchmarker.benchmark(&mut || {
        object.modify_voxels_within_sphere(&sphere, &mut |indices, position, voxel| {
            black_box((indices, position, voxel));
        });
        mesh.sync_with_voxel_object(&object);
        black_box((&object, &mesh));
    });
}

pub fn obtain_sphere_voxel_object_contacts(benchmarker: impl Benchmarker) {
    let object_radius = 100.0;
    let sphere_radius = 0.15 * object_radius;
    let generator = create_sphere_generator(object_radius);
    let object = VoxelObject::generate(VoxelObjectBuffers::new(), &generator);
    let sphere = Sphere::new(Position::origin(), sphere_radius);
    let transform_to_object_space = Isometry3::from_parts(
        *(object.compute_aabb().center()
            - object_radius * UnitVector3::normalized_from(Vector3::same(1.0)))
        .as_vector(),
        UnitQuaternion::from_axis_angle(&UnitVector3::unit_z(), 1.0),
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

pub fn obtain_plane_voxel_object_contacts(benchmarker: impl Benchmarker) {
    let object_radius = 100.0;
    let plane_displacement = -0.92 * object_radius;
    let generator = create_sphere_generator(object_radius);
    let object = VoxelObject::generate(VoxelObjectBuffers::new(), &generator);
    let plane = Plane::new(UnitVector3::unit_y(), plane_displacement);
    let transform_to_object_space =
        Isometry3::from_translation(*object.compute_aabb().center().as_vector());

    benchmarker.benchmark(&mut || {
        collidable::for_each_voxel_object_plane_contact(
            &object,
            &transform_to_object_space,
            &plane,
            &mut |indices, geometry| {
                black_box((indices, geometry));
            },
        );
    });
}

pub fn obtain_mutual_voxel_object_contacts(benchmarker: impl Benchmarker) {
    let object_a_radius = 100.0;
    let object_b_radius = 0.15 * object_a_radius;
    let generator_a = create_sphere_generator(object_a_radius);
    let generator_b = create_sphere_generator(object_b_radius);
    let object_a = VoxelObject::generate(VoxelObjectBuffers::new(), &generator_a);
    let object_b = VoxelObject::generate(VoxelObjectBuffers::new(), &generator_b);
    let transform_to_object_a_space = Isometry3::from_parts(
        *(object_a.compute_aabb().center()
            - object_a_radius * UnitVector3::normalized_from(Vector3::same(1.0)))
        .as_vector(),
        UnitQuaternion::from_axis_angle(&UnitVector3::unit_z(), 1.0),
    );
    let transform_to_object_b_space =
        Isometry3::from_translation(*object_b.compute_aabb().center().as_vector());

    let voxel_type_densities = [1.0; 256];
    let inertial_properties_a =
        VoxelObjectInertialPropertyManager::initialized_from(&object_a, &voxel_type_densities);
    let inertial_properties_b =
        VoxelObjectInertialPropertyManager::initialized_from(&object_b, &voxel_type_densities);

    let meshed_object_a = MeshedVoxelObject::create(VoxelObjectMeshBuffers::new(), object_a);
    let meshed_object_b = MeshedVoxelObject::create(VoxelObjectMeshBuffers::new(), object_b);

    benchmarker.benchmark(&mut || {
        collidable::for_each_mutual_voxel_object_contact(
            &meshed_object_a,
            &inertial_properties_a,
            &meshed_object_b,
            &inertial_properties_b,
            &transform_to_object_a_space,
            &transform_to_object_b_space,
            &mut |indices, geometry| {
                black_box((indices, geometry));
            },
        );
    });
}

pub fn obtain_mutual_voronoi_region_contacts(benchmarker: impl Benchmarker) {
    let box_extent = 100.0;
    let points_per_dim = 4;
    let face_plane_shift = -0.1;
    let max_perturbation_angle = 0.03;
    let max_perturbation_translation = 0.5;

    let objects = generate_voronoi_region_voxel_objects(
        box_extent,
        points_per_dim,
        face_plane_shift,
        max_perturbation_angle,
        max_perturbation_translation,
        0,
    );

    let mut overlapping_pairs = Vec::new();
    for a in 0..objects.len() {
        for b in (a + 1)..objects.len() {
            let object_a = &objects[a];
            let object_b = &objects[b];

            let aabb_a = object_a
                .aabb
                .aabb_of_transformed(&object_a.transform_to_object_space.inverted().to_matrix());
            let aabb_b = object_b
                .aabb
                .aabb_of_transformed(&object_b.transform_to_object_space.inverted().to_matrix());

            if aabb_a.compute_overlap_with(&aabb_b).is_some() {
                overlapping_pairs.push([a, b]);
            }
        }
    }

    benchmarker.benchmark(&mut || {
        for [a, b] in &overlapping_pairs {
            let object_a = &objects[*a];
            let object_b = &objects[*b];

            collidable::for_each_mutual_voxel_object_contact(
                &object_a.voxel_object,
                &object_a.inertial_property_manager,
                &object_b.voxel_object,
                &object_b.inertial_property_manager,
                &object_a.transform_to_object_space,
                &object_b.transform_to_object_space,
                &mut |indices, geometry| {
                    black_box((indices, geometry));
                },
            );
        }
    });
}

pub fn copy_voronoi_regions(benchmarker: impl Benchmarker) {
    let object_radius = 100.0;
    let points_per_dim = 4;

    let generator = create_sphere_generator(object_radius);
    let object = VoxelObject::generate(VoxelObjectBuffers::new(), &generator);

    let aabb = object.compute_normalized_chunk_grid_bounds();

    let points = create_randomized_grid_points(points_per_dim, &aabb.compact());
    let tetrahedralization = DelaunayTetrahedralization::construct(&points).unwrap();

    let mut polyhedron = VoronoiPolyhedron::empty_in(Global);

    benchmarker.benchmark(&mut || {
        for dual_vertex_idx in tetrahedralization.internal_vertex_indices() {
            polyhedron.extract_from_delaunay_tetrahedra(&tetrahedralization, dual_vertex_idx);
            let polyhedron_aabb = polyhedron.compute_bounded_aabb(&aabb).unwrap();

            let result = object.copy_polyhedron(
                VoxelObjectBuffers::new(),
                &polyhedron_aabb,
                &polyhedron.face_planes,
            );

            black_box(result);
        }
    });
}

pub fn copy_voronoi_regions_with_inertial_property_transfer(benchmarker: impl Benchmarker) {
    let object_radius = 100.0;
    let points_per_dim = 4;

    let generator = create_sphere_generator(object_radius);
    let object = VoxelObject::generate(VoxelObjectBuffers::new(), &generator);

    let voxel_type_densities = [1.0; 256];

    let aabb = object.compute_normalized_chunk_grid_bounds();

    let points = create_randomized_grid_points(points_per_dim, &aabb.compact());
    let tetrahedralization = DelaunayTetrahedralization::construct(&points).unwrap();

    let mut polyhedron = VoronoiPolyhedron::empty_in(Global);

    benchmarker.benchmark(&mut || {
        for dual_vertex_idx in tetrahedralization.internal_vertex_indices() {
            polyhedron.extract_from_delaunay_tetrahedra(&tetrahedralization, dual_vertex_idx);
            let polyhedron_aabb = polyhedron.compute_bounded_aabb(&aabb).unwrap();

            let mut poly_inertial_property_manager = VoxelObjectInertialPropertyManager::zeroed();
            let mut inertial_property_copier = poly_inertial_property_manager
                .begin_computation(object.voxel_extent(), &voxel_type_densities);

            let result = object.copy_polyhedron_with_property_computer(
                VoxelObjectBuffers::new(),
                &polyhedron_aabb,
                &polyhedron.face_planes,
                &mut inertial_property_copier,
            );

            black_box(result);
        }
    });
}

fn create_sphere_generator(radius: f32) -> SDFVoxelGenerator<Global> {
    let mut graph = SDFGraph::new_in(Global);
    graph.add_node(SDFNode::new_sphere(radius));
    let sdf_generator = graph.build_in(Global).unwrap();

    SDFVoxelGenerator::new(
        1.0,
        sdf_generator,
        SameVoxelTypeGenerator::new(VoxelType::default()).into(),
    )
}

fn create_box_generator(extent: f32) -> SDFVoxelGenerator<Global> {
    let mut graph = SDFGraph::new_in(Global);
    graph.add_node(SDFNode::new_box([extent; 3]));
    let sdf_generator = graph.build_in(Global).unwrap();

    SDFVoxelGenerator::new(
        1.0,
        sdf_generator,
        SameVoxelTypeGenerator::new(VoxelType::default()).into(),
    )
}

#[derive(Debug)]
pub struct GeneratedVoxelObject {
    voxel_object: MeshedVoxelObject,
    inertial_property_manager: VoxelObjectInertialPropertyManager,
    transform_to_object_space: Isometry3,
    aabb: AxisAlignedBox,
}

pub fn generate_voronoi_region_voxel_objects(
    box_extent: f32,
    points_per_dim: usize,
    face_plane_shift: f32,
    max_perturbation_angle: f32,       // Radians per axis
    max_perturbation_translation: f32, // Voxels per axis
    perturbation_seed: u64,
) -> Vec<GeneratedVoxelObject> {
    let generator = create_box_generator(box_extent);
    let object = VoxelObject::generate(VoxelObjectBuffers::new(), &generator);

    let voxel_type_densities = [1.0; 256];

    let voxel_extent = object.voxel_extent();

    let aabb = object.compute_normalized_chunk_grid_bounds();

    let points = create_randomized_grid_points(points_per_dim, &aabb.compact());
    let tetrahedralization = DelaunayTetrahedralization::construct(&points).unwrap();

    let mut polyhedron = VoronoiPolyhedron::empty_in(Global);

    let rand_sym = |rng: &mut Rng| 2.0 * rng.random_f32_fraction() - 1.0;
    let rand_angle = |rng: &mut Rng| max_perturbation_angle * rand_sym(rng);
    let rand_translation = |rng: &mut Rng| max_perturbation_translation * rand_sym(rng);
    let rand_perturbation = |vertices: &[Point3C], rng: &mut Rng| {
        let mut centroid = Vector3::zeros();
        for vertex in vertices {
            centroid += *vertex.aligned().as_vector();
        }
        centroid /= vertices.len() as f32;

        let rotation = UnitQuaternion::from_axis_angle(&UnitVector3::unit_x(), rand_angle(rng))
            * UnitQuaternion::from_axis_angle(&UnitVector3::unit_y(), rand_angle(rng))
            * UnitQuaternion::from_axis_angle(&UnitVector3::unit_z(), rand_angle(rng));
        let translation = Vector3::new(
            rand_translation(rng),
            rand_translation(rng),
            rand_translation(rng),
        );

        Isometry3::from_parts(
            centroid - rotation.rotate_vector(&centroid) + translation,
            rotation,
        )
    };

    let mut rng = Rng::with_seed(perturbation_seed);

    let mut generated_objects = Vec::new();

    for dual_vertex_idx in tetrahedralization.internal_vertex_indices() {
        polyhedron.extract_from_delaunay_tetrahedra(&tetrahedralization, dual_vertex_idx);
        if polyhedron.vertices.is_empty() {
            continue;
        }

        polyhedron.iso_transform(&rand_perturbation(&polyhedron.vertices, &mut rng));

        let Some(polyhedron_aabb) = polyhedron.compute_bounded_aabb(&aabb) else {
            continue;
        };
        polyhedron.shift_face_planes(face_plane_shift);

        let mut inertial_property_manager = VoxelObjectInertialPropertyManager::zeroed();
        let mut inertial_property_copier = inertial_property_manager
            .begin_computation(object.voxel_extent(), &voxel_type_densities);

        let ExtractionResult::Extracted(extracted) = object.copy_polyhedron_with_property_computer(
            VoxelObjectBuffers::new(),
            &polyhedron_aabb,
            &polyhedron.face_planes,
            &mut inertial_property_copier,
        ) else {
            continue;
        };

        let [offset_i, offset_j, offset_k] = extracted.origin_offset_in_parent;
        let origin_offset =
            voxel_extent * Vector3::new(offset_i as f32, offset_j as f32, offset_k as f32);

        let transform_to_object_space = Isometry3::from_translation(-origin_offset);

        let aabb = extracted.voxel_object.compute_aabb();

        let voxel_object =
            MeshedVoxelObject::create(VoxelObjectMeshBuffers::new(), extracted.voxel_object);

        generated_objects.push(GeneratedVoxelObject {
            voxel_object,
            inertial_property_manager,
            transform_to_object_space,
            aabb,
        });
    }

    generated_objects
}

pub fn setup_generated_voxel_objects(
    entity_id_manager: &mut EntityIDManager,
    voxel_object_manager: &mut VoxelObjectManager,
    intersection_manager: &mut IntersectionManager,
    rigid_body_manager: &mut RigidBodyManager,
    collision_world: &mut CollisionWorld,
    objects: impl IntoIterator<Item = GeneratedVoxelObject>,
) {
    for GeneratedVoxelObject {
        voxel_object,
        inertial_property_manager,
        transform_to_object_space,
        aabb,
    } in objects
    {
        let transform_to_world_space = transform_to_object_space.inverted();
        let inertial_properties = inertial_property_manager.derive_inertial_properties();

        let world_center_of_mass =
            transform_to_world_space.transform_point(inertial_properties.center_of_mass());

        let frame = ReferenceFrame::new(
            world_center_of_mass.compact(),
            transform_to_world_space.rotation().compact(),
        );

        let entity_id = entity_id_manager.provide_id();

        let voxel_object_id = VoxelObjectID::from_entity_id(entity_id);

        voxel_object_manager
            .add_voxel_object(voxel_object_id, voxel_object)
            .unwrap();

        voxel_object_manager
            .add_physics_context_for_voxel_object(
                voxel_object_id,
                VoxelObjectPhysicsContext {
                    inertial_property_manager,
                },
            )
            .unwrap();

        let (model_transform, _, _) = setup::setup_rigid_body_for_new_voxel_object(
            rigid_body_manager,
            entity_id,
            inertial_properties,
            None,
            Some(&frame),
            None,
        )
        .unwrap();

        let bounding_volume_id = BoundingVolumeID::from_entity_id(entity_id);

        intersection_manager
            .bounding_volume_manager
            .insert_bounding_volume(bounding_volume_id, aabb.compact())
            .unwrap();

        intersection_manager
            .add_bounding_volume_to_hierarchy(
                bounding_volume_id,
                &Similarity3::from_isometry(transform_to_world_space),
            )
            .unwrap();

        setup_voxel_collidable(
            collision_world,
            entity_id,
            RigidBodyType::Dynamic,
            &VoxelCollidable::new(
                CollidableKind::Dynamic,
                ContactResponseParameters {
                    restitution_coef: 0.4,
                    ..Default::default()
                },
            ),
            Some(&model_transform),
        )
        .unwrap();
    }
}
