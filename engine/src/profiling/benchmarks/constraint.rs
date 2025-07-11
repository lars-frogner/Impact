//! Benchmarks for constraint resolution.

use crate::physics::{
    collision::{
        CollidableKind, Collision,
        components::CollidableComp,
        geometry::basic::{CollidableGeometry, CollisionWorld},
    },
    constraint::{ConstraintManager, solver::ConstraintSolverConfig},
    fph,
    inertia::InertialProperties,
    material::{ContactResponseParameters, components::UniformContactResponseComp},
    motion::{
        AngularVelocity, Orientation, Position, Velocity,
        components::{ReferenceFrameComp, VelocityComp},
    },
    rigid_body::{RigidBody, components::RigidBodyComp},
};
use impact_ecs::world::{EntityID, World as ECSWorld};
use impact_geometry::Sphere;
use impact_profiling::Profiler;
use nalgebra::point;

pub fn prepare_contacts(profiler: impl Profiler) {
    let mut ecs_world = ECSWorld::default();
    let mut collision_world = CollisionWorld::new();

    setup_stationary_overlapping_spheres(&mut ecs_world, &mut collision_world);

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
                    collider_a.entity_id(),
                    collider_b.entity_id(),
                    contact.clone(),
                ));
            }
        },
    );

    let mut constraint_manager = ConstraintManager::new(ConstraintSolverConfig::default());

    profiler.profile(&mut || {
        constraint_manager.prepare_specific_contacts_only(
            &ecs_world,
            contacts
                .iter()
                .map(|(entity_a, entity_b, contact)| (*entity_a, *entity_b, contact)),
        );
    });
}

pub fn solve_contact_velocities(profiler: impl Profiler) {
    let mut ecs_world = ECSWorld::default();
    let mut collision_world = CollisionWorld::new();

    setup_stationary_overlapping_spheres(&mut ecs_world, &mut collision_world);

    let mut constraint_manager = ConstraintManager::new(ConstraintSolverConfig {
        n_iterations: 10,
        ..Default::default()
    });
    constraint_manager.prepare_constraints(&ecs_world, &collision_world, &());

    profiler.profile(&mut || {
        let mut solver = constraint_manager.solver().clone();
        solver.compute_constrained_velocities();
        solver
    });
}

pub fn correct_contact_configurations(profiler: impl Profiler) {
    let mut ecs_world = ECSWorld::default();
    let mut collision_world = CollisionWorld::new();

    setup_stationary_overlapping_spheres(&mut ecs_world, &mut collision_world);

    let mut constraint_manager = ConstraintManager::new(ConstraintSolverConfig {
        n_positional_correction_iterations: 10,
        ..Default::default()
    });
    constraint_manager.prepare_constraints(&ecs_world, &collision_world, &());

    profiler.profile(&mut || {
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
    ecs_world: &mut ECSWorld,
    collision_world: &mut CollisionWorld,
    bodies: impl IntoIterator<Item = SphereBody>,
) -> Vec<EntityID> {
    bodies
        .into_iter()
        .map(
            |SphereBody {
                 sphere,
                 mass_density,
                 velocity,
             }| {
                let collidable_id = collision_world.add_collidable(
                    CollidableKind::Dynamic,
                    CollidableGeometry::local_sphere(Sphere::new(
                        Position::origin(),
                        sphere.radius(),
                    )),
                );

                let frame =
                    ReferenceFrameComp::for_rigid_body(*sphere.center(), Orientation::identity());

                let entity_id = ecs_world
                    .create_entity((
                        &frame,
                        &VelocityComp::linear(velocity),
                        &RigidBodyComp(RigidBody::new(
                            InertialProperties::of_uniform_sphere(mass_density),
                            Orientation::identity(),
                            1.0,
                            &velocity,
                            &AngularVelocity::zero(),
                        )),
                        &UniformContactResponseComp(ContactResponseParameters {
                            restitution_coef: 0.6,
                            ..Default::default()
                        }),
                        &CollidableComp { collidable_id },
                    ))
                    .unwrap();

                collision_world.synchronize_collidable(
                    collidable_id,
                    entity_id,
                    frame.create_transform_to_parent_space(),
                );

                entity_id
            },
        )
        .collect()
}

fn setup_stationary_overlapping_spheres(
    ecs_world: &mut ECSWorld,
    collision_world: &mut CollisionWorld,
) {
    setup_sphere_bodies(
        ecs_world,
        collision_world,
        (0..500).map(|i| {
            SphereBody::stationary(Sphere::new(point![fph::from(i) - 0.05, 0.0, 0.0], 0.5), 1.0)
        }),
    );
}
