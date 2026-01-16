//! Constraint resolution tests.

use approx::assert_abs_diff_eq;
use impact_geometry::{PlaneC, ReferenceFrame, SphereC};
use impact_math::{
    angle::{Angle, Radians},
    point::Point3C,
    vector::{Vector3, Vector3C},
};
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
    quantities::{Motion, OrientationC, PositionC, Velocity, VelocityC},
    rigid_body::{self, DynamicRigidBodyID, KinematicRigidBodyID, RigidBodyManager},
};

#[derive(Clone, Debug)]
struct SphereBody {
    sphere: SphereC,
    velocity: VelocityC,
    mass_density: f32,
    restitution_coef: f32,
}

#[derive(Clone, Debug)]
struct PlaneBody {
    origin: PositionC,
    orientation: OrientationC,
    restitution_coef: f32,
}

impl SphereBody {
    fn new(sphere: SphereC, velocity: VelocityC, mass_density: f32, restitution_coef: f32) -> Self {
        Self {
            sphere,
            velocity,
            mass_density,
            restitution_coef,
        }
    }

    fn center(&self) -> &PositionC {
        self.sphere.center()
    }
}

impl PlaneBody {
    fn new(origin: PositionC, orientation: OrientationC, restitution_coef: f32) -> Self {
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
                    SphereC::new(PositionC::origin(), sphere.radius()),
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
                    PlaneC::XZ_PLANE,
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
            SphereC::new(Point3C::new(x, 0.0, 0.0), 1.0),
            VelocityC::zeros(),
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
        assert_eq!(body.compute_velocity(), sphere.velocity.aligned());
        assert_abs_diff_eq!(
            body.compute_angular_velocity().angular_speed(),
            Radians::zero()
        );
    }
}

fn test_binary_sphere_collision(
    sphere_a: SphereBody,
    sphere_b: SphereBody,
    expected_velocity_a: VelocityC,
    expected_velocity_b: VelocityC,
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
    assert_eq!(body_a.orientation(), &OrientationC::identity());
    assert_abs_diff_eq!(
        body_a.compute_velocity(),
        expected_velocity_a.aligned(),
        epsilon = 1e-6
    );
    assert_abs_diff_eq!(
        body_a.compute_angular_velocity().angular_speed(),
        Radians::zero(),
        epsilon = 1e-6
    );

    let body_b = rigid_body_manager.dynamic_rigid_body(sphere_body_ids[1]);
    assert_eq!(body_b.position(), sphere_b.center());
    assert_eq!(body_b.orientation(), &OrientationC::identity());
    assert_abs_diff_eq!(
        body_b.compute_velocity(),
        expected_velocity_b.aligned(),
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
            SphereC::new(PositionC::origin(), radius),
            Vector3C::new(speed, 0.0, 0.0),
            mass_density,
            restitution,
        ),
        SphereBody::new(
            SphereC::new(Point3C::new(2.0 * radius - 1e-6, 0.0, 0.0), radius),
            VelocityC::zeros(),
            mass_density,
            restitution,
        ),
        // Sphere A will transfer all its velocity to B
        VelocityC::zeros(),
        Vector3C::new(speed, 0.0, 0.0),
    );
}

#[test]
fn moving_sphere_colliding_head_on_with_very_massive_stationary_sphere() {
    let radius = 1.0;
    let speed = 0.5;
    let restitution = 1.0;

    test_binary_sphere_collision(
        SphereBody::new(
            SphereC::new(PositionC::origin(), radius),
            Vector3C::new(speed, 0.0, 0.0),
            1.0,
            restitution,
        ),
        SphereBody::new(
            SphereC::new(Point3C::new(2.0 * radius - 1e-6, 0.0, 0.0), radius),
            VelocityC::zeros(),
            1e9,
            restitution,
        ),
        // Sphere A will invert its velocity
        Vector3C::new(-speed, 0.0, 0.0),
        VelocityC::zeros(),
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
            SphereC::new(PositionC::origin(), radius),
            Vector3C::new(speed, 0.0, 0.0),
            mass_density,
            restitution,
        ),
        SphereBody::new(
            SphereC::new(Point3C::new(2.0 * radius - 1e-6, 0.0, 0.0), radius),
            VelocityC::zeros(),
            mass_density,
            restitution,
        ),
        // Both spheres will continue with half the impact speed
        Vector3C::new(0.5 * speed, 0.0, 0.0),
        Vector3C::new(0.5 * speed, 0.0, 0.0),
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
            SphereC::new(Point3C::new(1e-6, 0.0, 0.0), radius),
            Vector3C::new(speed, 0.0, 0.0),
            mass_density,
            restitution,
        ),
        SphereBody::new(
            SphereC::new(Point3C::new(offset, offset, 0.0), radius),
            Vector3C::new(-speed, 0.0, 0.0),
            mass_density,
            restitution,
        ),
        Vector3C::new(0.0, -speed, 0.0),
        Vector3C::new(0.0, speed, 0.0),
    );
}

#[test]
fn sphere_colliding_with_static_plane() {
    let radius = 1.0;
    let speed_x = 0.5;
    let speed_y = 0.6;
    let restitution_coef = 1.0;

    let sphere = SphereBody::new(
        SphereC::new(Point3C::new(0.0, radius - 1e-6, 0.0), radius),
        Vector3C::new(speed_x, -speed_y, 0.0),
        1.0,
        restitution_coef,
    );
    let plane = PlaneBody::new(
        PositionC::origin(),
        OrientationC::identity(),
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
    assert_eq!(body.orientation(), &OrientationC::identity());
    assert_abs_diff_eq!(
        body.compute_velocity(),
        Vector3::new(speed_x, speed_y, 0.0),
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
            SphereC::new(Point3C::new(0.5 * penetration, 0.0, 0.0), radius),
            VelocityC::zeros(),
            mass_density,
            restitution_coef,
        ),
        SphereBody::new(
            SphereC::new(
                Point3C::new(2.0 * radius - 0.5 * penetration, 0.0, 0.0),
                radius,
            ),
            VelocityC::zeros(),
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
            &Point3C::new(2.0 * radius * (idx as f32), 0.0, 0.0),
            epsilon = 1e-6
        );
        assert_eq!(body.orientation(), &OrientationC::identity());
        assert_eq!(body.compute_velocity(), Velocity::zeros());
        assert_abs_diff_eq!(
            body.compute_angular_velocity().angular_speed(),
            Radians::zero()
        );
    }
}
