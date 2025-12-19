//! Constraint resolution tests.

use approx::assert_abs_diff_eq;
use impact_geometry::{Plane, ReferenceFrame, Sphere};
use impact_math::angle::{Angle, Radians};
use impact_physics::{
    anchor::AnchorManager,
    collision::{
        self, CollidableKind,
        collidable::basic::{CollisionWorld, LocalCollidable},
        setup::{PlanarCollidable, SphericalCollidable},
    },
    constraint::{ConstraintManager, solver::ConstraintSolverConfig},
    inertia::InertialProperties,
    material::ContactResponseParameters,
    quantities::{Motion, Orientation, Position, Velocity},
    rigid_body::{self, DynamicRigidBodyID, KinematicRigidBodyID, RigidBodyManager},
};
use nalgebra::{point, vector};

#[derive(Clone, Debug)]
struct SphereBody {
    sphere: Sphere,
    velocity: Velocity,
    mass_density: f32,
    restitution_coef: f32,
}

#[derive(Clone, Debug)]
struct PlaneBody {
    origin: Position,
    orientation: Orientation,
    restitution_coef: f32,
}

impl SphereBody {
    fn new(sphere: Sphere, velocity: Velocity, mass_density: f32, restitution_coef: f32) -> Self {
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
    fn new(origin: Position, orientation: Orientation, restitution_coef: f32) -> Self {
        Self {
            origin,
            orientation,
            restitution_coef,
        }
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
                 velocity,
                 mass_density,
                 restitution_coef,
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
                        restitution_coef,
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

fn setup_plane_bodies(
    rigid_body_manager: &mut RigidBodyManager,
    collision_world: &mut CollisionWorld,
    bodies: impl IntoIterator<Item = PlaneBody>,
) -> Vec<KinematicRigidBodyID> {
    bodies
        .into_iter()
        .map(
            |PlaneBody {
                 origin,
                 orientation,
                 restitution_coef,
             }| {
                let frame = ReferenceFrame::new(origin, orientation);
                let motion = Motion::stationary();

                let rigid_body_id = rigid_body::setup::setup_kinematic_rigid_body(
                    rigid_body_manager,
                    frame,
                    motion,
                );

                let collidable = PlanarCollidable::new(
                    CollidableKind::Static,
                    Plane::XZ_PLANE,
                    ContactResponseParameters {
                        restitution_coef,
                        ..Default::default()
                    },
                );

                collision::setup::setup_planar_collidable(
                    collision_world,
                    rigid_body_id.into(),
                    &collidable,
                    LocalCollidable::Plane,
                );

                rigid_body_id
            },
        )
        .collect()
}

fn setup_bodies_and_run_constraints(
    rigid_body_manager: &mut RigidBodyManager,
    constraint_manager: &mut ConstraintManager,
    spheres: impl IntoIterator<Item = SphereBody>,
    planes: impl IntoIterator<Item = PlaneBody>,
) -> (Vec<DynamicRigidBodyID>, Vec<KinematicRigidBodyID>) {
    let anchor_manager = AnchorManager::new();
    let mut collision_world = CollisionWorld::new();

    let sphere_entities = setup_sphere_bodies(rigid_body_manager, &mut collision_world, spheres);
    let plane_entities = setup_plane_bodies(rigid_body_manager, &mut collision_world, planes);

    collision_world.synchronize_collidables_with_rigid_bodies(rigid_body_manager);

    constraint_manager.prepare_constraints(
        rigid_body_manager,
        &anchor_manager,
        &collision_world,
        &(),
    );
    constraint_manager.compute_and_apply_constrained_state(rigid_body_manager);

    (sphere_entities, plane_entities)
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

    let mut rigid_body_manager = RigidBodyManager::new();
    let mut constraint_manager = ConstraintManager::new(ConstraintSolverConfig::default());

    let (sphere_body_ids, _) = setup_bodies_and_run_constraints(
        &mut rigid_body_manager,
        &mut constraint_manager,
        spheres.clone(),
        [],
    );

    assert_eq!(constraint_manager.solver().prepared_contact_count(), 0);
    assert_eq!(constraint_manager.solver().prepared_body_count(), 0);

    for (id, sphere) in sphere_body_ids.into_iter().zip(spheres) {
        let body = rigid_body_manager.dynamic_rigid_body(id);
        assert_eq!(body.position(), sphere.center());
        assert_eq!(body.compute_velocity(), sphere.velocity);
        assert_abs_diff_eq!(
            body.compute_angular_velocity().angular_speed(),
            Radians::zero()
        );
    }
}

fn test_binary_sphere_collision(
    sphere_a: SphereBody,
    sphere_b: SphereBody,
    expected_velocity_a: Velocity,
    expected_velocity_b: Velocity,
) {
    let mut rigid_body_manager = RigidBodyManager::new();
    let mut constraint_manager = ConstraintManager::new(ConstraintSolverConfig {
        n_iterations: 1,
        n_positional_correction_iterations: 0,
        ..Default::default()
    });

    let (sphere_body_ids, _) = setup_bodies_and_run_constraints(
        &mut rigid_body_manager,
        &mut constraint_manager,
        [sphere_a.clone(), sphere_b.clone()],
        [],
    );

    assert_eq!(constraint_manager.solver().prepared_contact_count(), 1);
    assert_eq!(constraint_manager.solver().prepared_body_count(), 2);

    let body_a = rigid_body_manager.dynamic_rigid_body(sphere_body_ids[0]);
    assert_eq!(body_a.position(), sphere_a.center());
    assert_eq!(body_a.orientation(), &Orientation::identity());
    assert_abs_diff_eq!(
        body_a.compute_velocity(),
        expected_velocity_a,
        epsilon = 1e-6
    );
    assert_abs_diff_eq!(
        body_a.compute_angular_velocity().angular_speed(),
        Radians::zero(),
        epsilon = 1e-6
    );

    let body_b = rigid_body_manager.dynamic_rigid_body(sphere_body_ids[1]);
    assert_eq!(body_b.position(), sphere_b.center());
    assert_eq!(body_b.orientation(), &Orientation::identity());
    assert_abs_diff_eq!(
        body_b.compute_velocity(),
        expected_velocity_b,
        epsilon = 1e-6
    );
    assert_abs_diff_eq!(
        body_b.compute_angular_velocity().angular_speed(),
        Radians::zero(),
        epsilon = 1e-6
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
    let offset = f32::sqrt(2.0) * radius; // Gives 90 degree deflection

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

    let mut rigid_body_manager = RigidBodyManager::new();
    let mut constraint_manager = ConstraintManager::new(ConstraintSolverConfig {
        n_iterations: 1,
        n_positional_correction_iterations: 0,
        ..Default::default()
    });

    let (sphere_body_ids, _) = setup_bodies_and_run_constraints(
        &mut rigid_body_manager,
        &mut constraint_manager,
        [sphere.clone()],
        [plane],
    );

    assert_eq!(constraint_manager.solver().prepared_contact_count(), 1);
    assert_eq!(constraint_manager.solver().prepared_body_count(), 2);

    let body = rigid_body_manager.dynamic_rigid_body(sphere_body_ids[0]);
    assert_eq!(body.position(), sphere.center());
    assert_eq!(body.orientation(), &Orientation::identity());
    assert_abs_diff_eq!(
        body.compute_velocity(),
        vector![speed_x, speed_y, 0.0],
        epsilon = 1e-6
    );
    assert_abs_diff_eq!(
        body.compute_angular_velocity().angular_speed(),
        Radians::zero(),
        epsilon = 1e-6
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

    let mut rigid_body_manager = RigidBodyManager::new();
    let mut constraint_manager = ConstraintManager::new(ConstraintSolverConfig {
        n_iterations: 0,
        n_positional_correction_iterations: 1,
        positional_correction_factor: 1.0,
        ..Default::default()
    });

    let (sphere_body_ids, _) = setup_bodies_and_run_constraints(
        &mut rigid_body_manager,
        &mut constraint_manager,
        spheres.clone(),
        [],
    );

    assert_eq!(constraint_manager.solver().prepared_contact_count(), 1);
    assert_eq!(constraint_manager.solver().prepared_body_count(), 2);

    for (idx, id) in sphere_body_ids.into_iter().enumerate() {
        let body = rigid_body_manager.dynamic_rigid_body(id);
        assert_abs_diff_eq!(
            body.position(),
            &point![2.0 * radius * (idx as f32), 0.0, 0.0],
            epsilon = 1e-6
        );
        assert_eq!(body.orientation(), &Orientation::identity());
        assert_eq!(body.compute_velocity(), Velocity::zeros());
        assert_abs_diff_eq!(
            body.compute_angular_velocity().angular_speed(),
            Radians::zero()
        );
    }
}
