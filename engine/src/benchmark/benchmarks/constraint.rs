//! Benchmarks for constraint resolution.

use impact_geometry::{ReferenceFrame, Sphere};
use impact_physics::{
    anchor::AnchorManager,
    collision::{
        self, CollidableKind, Collision,
        collidable::basic::{CollisionWorld, LocalCollidable},
        setup::SphericalCollidable,
    },
    constraint::{ConstraintManager, solver::ConstraintSolverConfig},
    fph,
    inertia::InertialProperties,
    material::ContactResponseParameters,
    quantities::{Motion, Position, Velocity},
    rigid_body::{self, DynamicRigidBodyID, RigidBodyManager},
};
use impact_profiling::benchmark::Benchmarker;
use nalgebra::point;

pub fn prepare_contacts(benchmarker: impl Benchmarker) {
    let mut rigid_body_manager = RigidBodyManager::new();
    let mut collision_world = CollisionWorld::new();

    setup_stationary_overlapping_spheres(&mut rigid_body_manager, &mut collision_world);

    let mut contacts = Vec::new();
    collision_world.for_each_non_phantom_collision_involving_dynamic_collidable(
        &(),
        &mut |Collision {
                  collider_a,
                  collider_b,
                  contact_manifold,
              }| {
            for contact in contact_manifold.contacts() {
                contacts.push((
                    collider_a.rigid_body_id(),
                    collider_b.rigid_body_id(),
                    contact.clone(),
                ));
            }
        },
    );

    let mut constraint_manager = ConstraintManager::new(ConstraintSolverConfig::default());

    benchmarker.benchmark(&mut || {
        constraint_manager.prepare_specific_contacts_only(
            &rigid_body_manager,
            contacts
                .iter()
                .map(|(entity_a, entity_b, contact)| (*entity_a, *entity_b, contact)),
        );
    });
}

pub fn solve_contact_velocities(benchmarker: impl Benchmarker) {
    let mut rigid_body_manager = RigidBodyManager::new();
    let anchor_manager = AnchorManager::new();
    let mut collision_world = CollisionWorld::new();

    setup_stationary_overlapping_spheres(&mut rigid_body_manager, &mut collision_world);

    let mut constraint_manager = ConstraintManager::new(ConstraintSolverConfig {
        n_iterations: 10,
        ..Default::default()
    });
    constraint_manager.prepare_constraints(
        &rigid_body_manager,
        &anchor_manager,
        &collision_world,
        &(),
    );

    benchmarker.benchmark(&mut || {
        let mut solver = constraint_manager.solver().clone();
        solver.compute_constrained_velocities();
        solver
    });
}

pub fn correct_contact_configurations(benchmarker: impl Benchmarker) {
    let mut rigid_body_manager = RigidBodyManager::new();
    let anchor_manager = AnchorManager::new();
    let mut collision_world = CollisionWorld::new();

    setup_stationary_overlapping_spheres(&mut rigid_body_manager, &mut collision_world);

    let mut constraint_manager = ConstraintManager::new(ConstraintSolverConfig {
        n_positional_correction_iterations: 10,
        ..Default::default()
    });
    constraint_manager.prepare_constraints(
        &rigid_body_manager,
        &anchor_manager,
        &collision_world,
        &(),
    );

    benchmarker.benchmark(&mut || {
        let mut solver = constraint_manager.solver().clone();
        solver.compute_corrected_configurations();
        solver
    });
}

struct SphereBody {
    sphere: Sphere<fph>,
    mass_density: fph,
    velocity: Velocity,
}

impl SphereBody {
    fn new(sphere: Sphere<fph>, mass_density: fph, velocity: Velocity) -> Self {
        Self {
            sphere,
            mass_density,
            velocity,
        }
    }

    fn stationary(sphere: Sphere<fph>, mass_density: fph) -> Self {
        Self::new(sphere, mass_density, Velocity::zeros())
    }
}

fn setup_sphere_bodies(
    rigid_body_manager: &mut RigidBodyManager,
    collision_world: &mut CollisionWorld,
    bodies: impl IntoIterator<Item = SphereBody>,
) -> Vec<DynamicRigidBodyID> {
    bodies
        .into_iter()
        .map(
            |SphereBody {
                 sphere,
                 mass_density,
                 velocity,
             }| {
                let frame = ReferenceFrame::unoriented(*sphere.center());
                let motion = Motion::linear(velocity);

                let inertial_properties = InertialProperties::of_uniform_sphere(0.5, mass_density);

                let rigid_body_id = rigid_body::setup::setup_dynamic_rigid_body(
                    rigid_body_manager,
                    inertial_properties,
                    frame,
                    motion,
                );

                let collidable = SphericalCollidable::new(
                    CollidableKind::Dynamic,
                    Sphere::new(Position::origin(), sphere.radius()),
                    ContactResponseParameters {
                        restitution_coef: 0.6,
                        ..Default::default()
                    },
                );

                collision::setup::setup_spherical_collidable(
                    collision_world,
                    rigid_body_id.into(),
                    &collidable,
                    LocalCollidable::Sphere,
                );

                rigid_body_id
            },
        )
        .collect()
}

fn setup_stationary_overlapping_spheres(
    rigid_body_manager: &mut RigidBodyManager,
    collision_world: &mut CollisionWorld,
) {
    setup_sphere_bodies(
        rigid_body_manager,
        collision_world,
        (0..500).map(|i| {
            SphereBody::stationary(Sphere::new(point![fph::from(i) - 0.05, 0.0, 0.0], 0.5), 1.0)
        }),
    );

    collision_world.synchronize_collidables_with_rigid_bodies(rigid_body_manager);
}
