//! Constraint resolution tests.

use approx::assert_abs_diff_eq;
use impact::{
    geometry::{Angle, Plane, Radians, Sphere},
    physics::{
        collision::{CollidableKind, CollisionWorld, components::CollidableComp},
        constraint::{ConstraintManager, solver::ConstraintSolverConfig},
        fph,
        inertia::InertialProperties,
        material::{ContactResponseParameters, components::UniformContactResponseComp},
        motion::{
            AngularVelocity, Orientation, Position, Velocity,
            components::{ReferenceFrameComp, VelocityComp},
        },
        rigid_body::{RigidBody, components::RigidBodyComp},
    },
    voxel::VoxelObjectManager,
};
use impact_ecs::world::{EntityID, World as ECSWorld};
use nalgebra::{point, vector};

#[derive(Clone, Debug)]
struct SphereBody {
    sphere: Sphere<fph>,
    velocity: Velocity,
    mass_density: fph,
    restitution_coef: fph,
}

#[derive(Clone, Debug)]
struct PlaneBody {
    origin: Position,
    orientation: Orientation,
    restitution_coef: fph,
}

impl SphereBody {
    fn new(
        sphere: Sphere<fph>,
        velocity: Velocity,
        mass_density: fph,
        restitution_coef: fph,
    ) -> Self {
        Self {
            sphere,
            velocity,
            mass_density,
            restitution_coef,
        }
    }

    fn center(&self) -> &Position {
        self.sphere.center()
    }
}

impl PlaneBody {
    fn new(origin: Position, orientation: Orientation, restitution_coef: fph) -> Self {
        Self {
            origin,
            orientation,
            restitution_coef,
        }
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
                 velocity,
                 mass_density,
                 restitution_coef,
             }| {
                let collidable_id = collision_world.add_sphere_collidable(
                    CollidableKind::Dynamic,
                    Sphere::new(Position::origin(), sphere.radius()),
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
                            restitution_coef,
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

fn setup_plane_bodies(
    ecs_world: &mut ECSWorld,
    collision_world: &mut CollisionWorld,
    bodies: impl IntoIterator<Item = PlaneBody>,
) -> Vec<EntityID> {
    bodies
        .into_iter()
        .map(
            |PlaneBody {
                 origin,
                 orientation,
                 restitution_coef,
             }| {
                let collidable_id =
                    collision_world.add_plane_collidable(CollidableKind::Static, Plane::XZ_PLANE);

                let frame = ReferenceFrameComp::unscaled(origin, orientation);

                let entity_id = ecs_world
                    .create_entity((
                        &frame,
                        &UniformContactResponseComp(ContactResponseParameters {
                            restitution_coef,
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

fn for_entity_state(
    ecs_world: &ECSWorld,
    entity_id: EntityID,
    f: impl FnOnce(&Position, &Orientation, &Velocity, &AngularVelocity),
) {
    let entry = ecs_world.entity(entity_id);
    let frame = entry.component::<ReferenceFrameComp>();
    let frame = frame.access();
    let velocity = entry.component::<VelocityComp>();
    let velocity = velocity.access();
    f(
        &frame.position,
        &frame.orientation,
        &velocity.linear,
        &velocity.angular,
    );
}

fn for_entity_states(
    ecs_world: &ECSWorld,
    entity_ids: impl IntoIterator<Item = EntityID>,
    f: &impl Fn(usize, &Position, &Orientation, &Velocity, &AngularVelocity),
) {
    for (idx, entity_id) in entity_ids.into_iter().enumerate() {
        for_entity_state(ecs_world, entity_id, |p, o, v, a| f(idx, p, o, v, a));
    }
}

fn setup_bodies_and_run_constraints(
    ecs_world: &mut ECSWorld,
    constraint_manager: &mut ConstraintManager,
    spheres: impl IntoIterator<Item = SphereBody>,
    planes: impl IntoIterator<Item = PlaneBody>,
) -> (Vec<EntityID>, Vec<EntityID>) {
    let mut collision_world = CollisionWorld::new();
    let sphere_entities = setup_sphere_bodies(ecs_world, &mut collision_world, spheres);
    let plane_entities = setup_plane_bodies(ecs_world, &mut collision_world, planes);
    run_constraints(ecs_world, &collision_world, constraint_manager);
    (sphere_entities, plane_entities)
}

fn run_constraints(
    ecs_world: &ECSWorld,
    collision_world: &CollisionWorld,
    constraint_manager: &mut ConstraintManager,
) {
    constraint_manager.prepare_constraints(ecs_world, &VoxelObjectManager::new(), collision_world);
    constraint_manager.compute_and_apply_constrained_state(ecs_world);
}

#[test]
fn separated_bodies_unaffected_by_contact_constraints() {
    let spheres = [0.0, 2.1].map(|x| {
        SphereBody::new(
            Sphere::new(point![x, 0.0, 0.0], 1.0),
            Velocity::zeros(),
            1.0,
            1.0,
        )
    });

    let mut ecs_world = ECSWorld::default();
    let mut constraint_manager = ConstraintManager::new(ConstraintSolverConfig::default());

    let (entity_ids, _) = setup_bodies_and_run_constraints(
        &mut ecs_world,
        &mut constraint_manager,
        spheres.clone(),
        [],
    );

    assert_eq!(constraint_manager.solver().prepared_contact_count(), 0);
    assert_eq!(constraint_manager.solver().prepared_body_count(), 0);

    for_entity_states(
        &ecs_world,
        entity_ids,
        &|idx, position, _, velocity, angular_velocity| {
            assert_eq!(position, spheres[idx].center());
            assert_eq!(velocity, &spheres[idx].velocity);
            assert_abs_diff_eq!(angular_velocity.angular_speed(), Radians::zero());
        },
    );
}

fn test_binary_sphere_collision(
    sphere_a: SphereBody,
    sphere_b: SphereBody,
    expected_velocity_a: Velocity,
    expected_velocity_b: Velocity,
) {
    let mut ecs_world = ECSWorld::default();
    let mut constraint_manager = ConstraintManager::new(ConstraintSolverConfig {
        n_iterations: 1,
        n_positional_correction_iterations: 0,
        ..Default::default()
    });

    let (entity_ids, _) = setup_bodies_and_run_constraints(
        &mut ecs_world,
        &mut constraint_manager,
        [sphere_a.clone(), sphere_b.clone()],
        [],
    );

    assert_eq!(constraint_manager.solver().prepared_contact_count(), 1);
    assert_eq!(constraint_manager.solver().prepared_body_count(), 2);

    for_entity_state(
        &ecs_world,
        entity_ids[0],
        |position, orientation, velocity, angular_velocity| {
            assert_eq!(position, sphere_a.center());
            assert_eq!(orientation, &Orientation::identity());
            assert_abs_diff_eq!(velocity, &expected_velocity_a, epsilon = 1e-6);
            assert_abs_diff_eq!(
                angular_velocity.angular_speed(),
                Radians::zero(),
                epsilon = 1e-6
            );
        },
    );
    for_entity_state(
        &ecs_world,
        entity_ids[1],
        |position, orientation, velocity, angular_velocity| {
            assert_eq!(position, sphere_b.center());
            assert_eq!(orientation, &Orientation::identity());
            assert_abs_diff_eq!(velocity, &expected_velocity_b, epsilon = 1e-6);
            assert_abs_diff_eq!(
                angular_velocity.angular_speed(),
                Radians::zero(),
                epsilon = 1e-6
            );
        },
    );
}

#[test]
fn moving_sphere_colliding_head_on_with_same_mass_stationary_sphere() {
    let radius = 1.0;
    let speed = 0.5;
    let mass_density = 1.0;
    let restitution = 1.0;

    test_binary_sphere_collision(
        SphereBody::new(
            Sphere::new(Position::origin(), radius),
            vector![speed, 0.0, 0.0],
            mass_density,
            restitution,
        ),
        SphereBody::new(
            Sphere::new(point![2.0 * radius - 1e-6, 0.0, 0.0], radius),
            Velocity::zeros(),
            mass_density,
            restitution,
        ),
        // Sphere A will transfer all its velocity to B
        Velocity::zeros(),
        vector![speed, 0.0, 0.0],
    );
}

#[test]
fn moving_sphere_colliding_head_on_with_very_massive_stationary_sphere() {
    let radius = 1.0;
    let speed = 0.5;
    let restitution = 1.0;

    test_binary_sphere_collision(
        SphereBody::new(
            Sphere::new(Position::origin(), radius),
            vector![speed, 0.0, 0.0],
            1.0,
            restitution,
        ),
        SphereBody::new(
            Sphere::new(point![2.0 * radius - 1e-6, 0.0, 0.0], radius),
            Velocity::zeros(),
            1e9,
            restitution,
        ),
        // Sphere A will invert its velocity
        vector![-speed, 0.0, 0.0],
        Velocity::zeros(),
    );
}

#[test]
fn moving_sphere_colliding_head_on_with_inelastic_same_mass_stationary_sphere() {
    let radius = 1.0;
    let speed = 0.5;
    let mass_density = 1.0;
    let restitution = 0.0; // <- Completely inelastic

    test_binary_sphere_collision(
        SphereBody::new(
            Sphere::new(Position::origin(), radius),
            vector![speed, 0.0, 0.0],
            mass_density,
            restitution,
        ),
        SphereBody::new(
            Sphere::new(point![2.0 * radius - 1e-6, 0.0, 0.0], radius),
            Velocity::zeros(),
            mass_density,
            restitution,
        ),
        // Both spheres will continue with half the impact speed
        vector![0.5 * speed, 0.0, 0.0],
        vector![0.5 * speed, 0.0, 0.0],
    );
}

#[test]
fn grazing_sphere_collision() {
    let radius = 1.0;
    let speed = 0.5;
    let mass_density = 1.0;
    let restitution = 1.0;
    let offset = fph::sqrt(2.0) * radius; // Gives 90 degree deflection

    test_binary_sphere_collision(
        SphereBody::new(
            Sphere::new(point![1e-6, 0.0, 0.0], radius),
            vector![speed, 0.0, 0.0],
            mass_density,
            restitution,
        ),
        SphereBody::new(
            Sphere::new(point![offset, offset, 0.0], radius),
            vector![-speed, 0.0, 0.0],
            mass_density,
            restitution,
        ),
        vector![0.0, -speed, 0.0],
        vector![0.0, speed, 0.0],
    );
}

#[test]
fn sphere_colliding_with_static_plane() {
    let radius = 1.0;
    let speed_x = 0.5;
    let speed_y = 0.6;
    let restitution_coef = 1.0;

    let sphere = SphereBody::new(
        Sphere::new(point![0.0, radius - 1e-6, 0.0], radius),
        vector![speed_x, -speed_y, 0.0],
        1.0,
        restitution_coef,
    );
    let plane = PlaneBody::new(
        Position::origin(),
        Orientation::identity(),
        restitution_coef,
    );

    let mut ecs_world = ECSWorld::default();
    let mut constraint_manager = ConstraintManager::new(ConstraintSolverConfig {
        n_iterations: 1,
        n_positional_correction_iterations: 0,
        ..Default::default()
    });

    let (sphere_entity_ids, _) = setup_bodies_and_run_constraints(
        &mut ecs_world,
        &mut constraint_manager,
        [sphere.clone()],
        [plane],
    );

    assert_eq!(constraint_manager.solver().prepared_contact_count(), 1);
    assert_eq!(constraint_manager.solver().prepared_body_count(), 2);

    for_entity_state(
        &ecs_world,
        sphere_entity_ids[0],
        |position, orientation, velocity, angular_velocity| {
            assert_eq!(position, sphere.center());
            assert_eq!(orientation, &Orientation::identity());
            assert_abs_diff_eq!(velocity, &vector![speed_x, speed_y, 0.0], epsilon = 1e-6);
            assert_abs_diff_eq!(
                angular_velocity.angular_speed(),
                Radians::zero(),
                epsilon = 1e-6
            );
        },
    );
}

#[test]
fn position_correction_of_interpenetrating_spheres() {
    let radius = 1.0;
    let mass_density = 1.0;
    let restitution_coef = 1.0;
    let penetration = 0.2;

    let spheres = [
        SphereBody::new(
            Sphere::new(point![0.5 * penetration, 0.0, 0.0], radius),
            Velocity::zeros(),
            mass_density,
            restitution_coef,
        ),
        SphereBody::new(
            Sphere::new(point![2.0 * radius - 0.5 * penetration, 0.0, 0.0], radius),
            Velocity::zeros(),
            mass_density,
            restitution_coef,
        ),
    ];

    let mut ecs_world = ECSWorld::default();
    let mut constraint_manager = ConstraintManager::new(ConstraintSolverConfig {
        n_iterations: 0,
        n_positional_correction_iterations: 1,
        positional_correction_factor: 1.0,
        ..Default::default()
    });

    let (sphere_entity_ids, _) = setup_bodies_and_run_constraints(
        &mut ecs_world,
        &mut constraint_manager,
        spheres.clone(),
        [],
    );

    assert_eq!(constraint_manager.solver().prepared_contact_count(), 1);
    assert_eq!(constraint_manager.solver().prepared_body_count(), 2);

    for_entity_states(
        &ecs_world,
        sphere_entity_ids,
        &|idx, position, orientation, velocity, angular_velocity| {
            assert_abs_diff_eq!(
                position,
                &point![2.0 * radius * (idx as fph), 0.0, 0.0],
                epsilon = 1e-6
            );
            assert_eq!(orientation, &Orientation::identity());
            assert_eq!(velocity, &Velocity::zeros());
            assert_eq!(angular_velocity.angular_speed(), Radians::zero());
        },
    );
}
