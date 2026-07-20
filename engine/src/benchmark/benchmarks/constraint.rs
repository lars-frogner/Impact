//! Benchmarks for constraint resolution.

use super::voxel_object::{generate_voronoi_region_voxel_objects, setup_generated_voxel_objects};
use impact_geometry::{AxisAlignedBoxC, ReferenceFrame, SphereC};
use impact_id::{EntityID, EntityIDManager};
use impact_intersection::{IntersectionManager, bounding_volume::BoundingVolumeID};
use impact_math::{point::Point3C, transform::Similarity3, vector::Vector3C};
use impact_physics::{
    anchor::AnchorManager,
    collision::{
        self, CollidableKind, CollisionCacheUsage,
        collidable::basic::{CollisionWorld, LocalCollidable},
        setup::SphericalCollidable,
    },
    constraint::{ConstraintManager, solver::ConstraintSolverConfig},
    inertia::InertialProperties,
    material::ContactResponseParameters,
    quantities::{Motion, PositionC, VelocityC},
    rigid_body::{self, RigidBodyManager, RigidBodyType},
};
use impact_profiling::benchmark::Benchmarker;
use impact_voxel::VoxelObjectManager;

pub fn prepare_contacts(benchmarker: impl Benchmarker) {
    let mut intersection_manager = IntersectionManager::new();
    let mut rigid_body_manager = RigidBodyManager::new();
    let anchor_manager = AnchorManager::new();
    let mut collision_world = CollisionWorld::new();

    setup_stationary_overlapping_spheres(
        &mut intersection_manager,
        &mut rigid_body_manager,
        &mut collision_world,
    );

    let mut constraint_manager = ConstraintManager::new(ConstraintSolverConfig::default());

    benchmarker.benchmark(&mut || {
        constraint_manager.prepare_constraints(
            &intersection_manager,
            &rigid_body_manager,
            &anchor_manager,
            &collision_world,
            &(),
            CollisionCacheUsage::IgnoreCached,
        );
    });
}

pub fn solve_contact_velocities(benchmarker: impl Benchmarker) {
    let mut intersection_manager = IntersectionManager::new();
    let mut rigid_body_manager = RigidBodyManager::new();
    let anchor_manager = AnchorManager::new();
    let mut collision_world = CollisionWorld::new();

    setup_stationary_overlapping_spheres(
        &mut intersection_manager,
        &mut rigid_body_manager,
        &mut collision_world,
    );

    let mut constraint_manager = ConstraintManager::new(ConstraintSolverConfig {
        n_iterations: 10,
        ..Default::default()
    });
    constraint_manager.prepare_constraints(
        &intersection_manager,
        &rigid_body_manager,
        &anchor_manager,
        &collision_world,
        &(),
        CollisionCacheUsage::IgnoreCached,
    );

    benchmarker.benchmark(&mut || {
        let mut solver = constraint_manager.solver().clone();
        solver.compute_constrained_velocities();
        solver
    });
}

pub fn correct_contact_configurations(benchmarker: impl Benchmarker) {
    let mut intersection_manager = IntersectionManager::new();
    let mut rigid_body_manager = RigidBodyManager::new();
    let anchor_manager = AnchorManager::new();
    let mut collision_world = CollisionWorld::new();

    setup_stationary_overlapping_spheres(
        &mut intersection_manager,
        &mut rigid_body_manager,
        &mut collision_world,
    );

    let mut constraint_manager = ConstraintManager::new(ConstraintSolverConfig {
        n_positional_correction_iterations: 10,
        ..Default::default()
    });
    constraint_manager.prepare_constraints(
        &intersection_manager,
        &rigid_body_manager,
        &anchor_manager,
        &collision_world,
        &(),
        CollisionCacheUsage::IgnoreCached,
    );

    benchmarker.benchmark(&mut || {
        let mut solver = constraint_manager.solver().clone();
        solver.compute_corrected_configurations();
        solver
    });
}

pub fn prepare_voxel_object_contacts(benchmarker: impl Benchmarker) {
    let mut entity_id_manager = EntityIDManager::new();
    let mut voxel_object_manager = VoxelObjectManager::new();
    let mut intersection_manager = IntersectionManager::new();
    let mut rigid_body_manager = RigidBodyManager::new();
    let anchor_manager = AnchorManager::new();
    let mut collision_world = impact_voxel::collidable::CollisionWorld::new();

    setup_voronoi_region_voxel_objects(
        &mut entity_id_manager,
        &mut voxel_object_manager,
        &mut intersection_manager,
        &mut rigid_body_manager,
        &mut collision_world,
    );

    collision_world.cache_all_collisions(&voxel_object_manager, &intersection_manager);

    let mut constraint_manager = ConstraintManager::new(ConstraintSolverConfig::default());

    benchmarker.benchmark(&mut || {
        constraint_manager.prepare_constraints(
            &intersection_manager,
            &rigid_body_manager,
            &anchor_manager,
            &collision_world,
            &voxel_object_manager,
            CollisionCacheUsage::UseCached,
        );
    });
}

pub fn solve_voxel_object_contact_velocities(benchmarker: impl Benchmarker) {
    let mut entity_id_manager = EntityIDManager::new();
    let mut voxel_object_manager = VoxelObjectManager::new();
    let mut intersection_manager = IntersectionManager::new();
    let mut rigid_body_manager = RigidBodyManager::new();
    let anchor_manager = AnchorManager::new();
    let mut collision_world = impact_voxel::collidable::CollisionWorld::new();

    setup_voronoi_region_voxel_objects(
        &mut entity_id_manager,
        &mut voxel_object_manager,
        &mut intersection_manager,
        &mut rigid_body_manager,
        &mut collision_world,
    );

    let mut constraint_manager = ConstraintManager::new(ConstraintSolverConfig {
        n_iterations: 10,
        ..Default::default()
    });
    constraint_manager.prepare_constraints(
        &intersection_manager,
        &rigid_body_manager,
        &anchor_manager,
        &collision_world,
        &voxel_object_manager,
        CollisionCacheUsage::IgnoreCached,
    );

    benchmarker.benchmark(&mut || {
        let mut solver = constraint_manager.solver().clone();
        solver.compute_constrained_velocities();
        solver
    });
}

pub fn correct_voxel_object_contact_configurations(benchmarker: impl Benchmarker) {
    let mut entity_id_manager = EntityIDManager::new();
    let mut voxel_object_manager = VoxelObjectManager::new();
    let mut intersection_manager = IntersectionManager::new();
    let mut rigid_body_manager = RigidBodyManager::new();
    let anchor_manager = AnchorManager::new();
    let mut collision_world = impact_voxel::collidable::CollisionWorld::new();

    setup_voronoi_region_voxel_objects(
        &mut entity_id_manager,
        &mut voxel_object_manager,
        &mut intersection_manager,
        &mut rigid_body_manager,
        &mut collision_world,
    );

    let mut constraint_manager = ConstraintManager::new(ConstraintSolverConfig {
        n_positional_correction_iterations: 10,
        ..Default::default()
    });
    constraint_manager.prepare_constraints(
        &intersection_manager,
        &rigid_body_manager,
        &anchor_manager,
        &collision_world,
        &voxel_object_manager,
        CollisionCacheUsage::IgnoreCached,
    );

    benchmarker.benchmark(&mut || {
        let mut solver = constraint_manager.solver().clone();
        solver.compute_corrected_configurations();
        solver
    });
}

struct SphereBody {
    entity_id: EntityID,
    sphere: SphereC,
    mass_density: f32,
    velocity: VelocityC,
}

impl SphereBody {
    fn new(entity_id: EntityID, sphere: SphereC, mass_density: f32, velocity: VelocityC) -> Self {
        Self {
            entity_id,
            sphere,
            mass_density,
            velocity,
        }
    }

    fn stationary(entity_id: EntityID, sphere: SphereC, mass_density: f32) -> Self {
        Self::new(entity_id, sphere, mass_density, VelocityC::zeros())
    }
}

fn setup_sphere_bodies(
    intersection_manager: &mut IntersectionManager,
    rigid_body_manager: &mut RigidBodyManager,
    collision_world: &mut CollisionWorld,
    bodies: impl IntoIterator<Item = SphereBody>,
) {
    for SphereBody {
        entity_id,
        sphere,
        mass_density,
        velocity,
    } in bodies
    {
        let frame = ReferenceFrame::unoriented(*sphere.center());
        let motion = Motion::linear(velocity);

        let inertial_properties = InertialProperties::of_uniform_sphere(0.5, mass_density);

        rigid_body::setup::setup_dynamic_rigid_body(
            rigid_body_manager,
            entity_id,
            inertial_properties,
            frame,
            motion,
        )
        .unwrap();

        let collidable = SphericalCollidable::new(
            CollidableKind::Dynamic,
            SphereC::new(PositionC::origin(), sphere.radius()),
            ContactResponseParameters {
                restitution_coef: 0.6,
                ..Default::default()
            },
        );

        let bounding_volume_id = BoundingVolumeID::from_entity_id(entity_id);

        intersection_manager
            .bounding_volume_manager
            .insert_bounding_volume(
                bounding_volume_id,
                AxisAlignedBoxC::new(
                    Vector3C::same(-sphere.radius()).into(),
                    Vector3C::same(sphere.radius()).into(),
                ),
            )
            .unwrap();

        intersection_manager
            .add_bounding_volume_to_hierarchy(
                bounding_volume_id,
                &Similarity3::from_isometry(frame.create_transform_to_parent_space()),
            )
            .unwrap();

        collision::setup::setup_spherical_collidable(
            collision_world,
            entity_id,
            RigidBodyType::Dynamic,
            &collidable,
            LocalCollidable::Sphere,
            None,
        )
        .unwrap();
    }
}

fn setup_stationary_overlapping_spheres(
    intersection_manager: &mut IntersectionManager,
    rigid_body_manager: &mut RigidBodyManager,
    collision_world: &mut CollisionWorld,
) {
    setup_sphere_bodies(
        intersection_manager,
        rigid_body_manager,
        collision_world,
        (0..500).map(|i| {
            SphereBody::stationary(
                EntityID::from_u64(i),
                SphereC::new(Point3C::new(i as f32 - 0.05, 0.0, 0.0), 0.5),
                1.0,
            )
        }),
    );

    intersection_manager.build_bounding_volume_hierarchy();

    collision_world.synchronize_collidables_with_rigid_bodies(rigid_body_manager);
}

fn setup_voronoi_region_voxel_objects(
    entity_id_manager: &mut EntityIDManager,
    voxel_object_manager: &mut VoxelObjectManager,
    intersection_manager: &mut IntersectionManager,
    rigid_body_manager: &mut RigidBodyManager,
    collision_world: &mut impact_voxel::collidable::CollisionWorld,
) {
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

    setup_generated_voxel_objects(
        entity_id_manager,
        voxel_object_manager,
        intersection_manager,
        rigid_body_manager,
        collision_world,
        objects,
    );

    intersection_manager.build_bounding_volume_hierarchy();

    collision_world.synchronize_collidables_with_rigid_bodies(rigid_body_manager);
}
