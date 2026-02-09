//! Constraint resolution tests.

use approx::assert_abs_diff_eq;
use impact_geometry::{PlaneC, ReferenceFrame, SphereC};
use impact_id::{EntityID, EntityIDManager};
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
    rigid_body::{self, DynamicRigidBodyID, RigidBodyManager, RigidBodyType},
};

#[derive(Clone, Debug)]
struct SphereBody {
    entity_id: EntityID,
    sphere: SphereC,
    velocity: VelocityC,
    mass_density: f32,
    restitution_coef: f32,
}

#[derive(Clone, Debug)]
struct PlaneBody {
    entity_id: EntityID,
    origin: PositionC,
    orientation: OrientationC,
    restitution_coef: f32,
}

impl SphereBody {
    fn new(
        entity_id: EntityID,
        sphere: SphereC,
        velocity: VelocityC,
        mass_density: f32,
        restitution_coef: f32,
    ) -> Self {
        Self {
            entity_id,
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
    fn new(
        entity_id: EntityID,
        origin: PositionC,
        orientation: OrientationC,
        restitution_coef: f32,
    ) -> Self {
        Self {
            entity_id,
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
) {
    for SphereBody {
        entity_id,
        sphere,
        velocity,
        mass_density,
        restitution_coef,
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
                restitution_coef,
                ..Default::default()
            },
        );

        collision::setup::setup_spherical_collidable(
            collision_world,
            entity_id,
            RigidBodyType::Dynamic,
            &collidable,
            LocalCollidable::Sphere,
        )
        .unwrap();
    }
}

fn setup_plane_bodies(
    rigid_body_manager: &mut RigidBodyManager,
    collision_world: &mut CollisionWorld,
    bodies: impl IntoIterator<Item = PlaneBody>,
) {
    for PlaneBody {
        entity_id,
        origin,
        orientation,
        restitution_coef,
    } in bodies
    {
        let frame = ReferenceFrame::new(origin, orientation);
        let motion = Motion::stationary();

        rigid_body::setup::setup_kinematic_rigid_body(rigid_body_manager, entity_id, frame, motion)
            .unwrap();

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
            entity_id,
            RigidBodyType::Kinematic,
            &collidable,
            LocalCollidable::Plane,
        )
        .unwrap();
    }
}

fn setup_bodies_and_run_constraints(
    rigid_body_manager: &mut RigidBodyManager,
    constraint_manager: &mut ConstraintManager,
    spheres: impl IntoIterator<Item = SphereBody>,
    planes: impl IntoIterator<Item = PlaneBody>,
) {
    let anchor_manager = AnchorManager::new();
    let mut collision_world = CollisionWorld::new();

    setup_sphere_bodies(rigid_body_manager, &mut collision_world, spheres);
    setup_plane_bodies(rigid_body_manager, &mut collision_world, planes);

    collision_world.synchronize_collidables_with_rigid_bodies(rigid_body_manager);

    constraint_manager.prepare_constraints(
        rigid_body_manager,
        &anchor_manager,
        &collision_world,
        &(),
    );
    constraint_manager.compute_and_apply_constrained_state(rigid_body_manager);
}

#[test]
fn separated_bodies_unaffected_by_contact_constraints() {
    let mut entity_id_manager = EntityIDManager::new();

    let spheres = [0.0, 2.1].map(|x| {
        SphereBody::new(
            entity_id_manager.provide_id(),
            SphereC::new(Point3C::new(x, 0.0, 0.0), 1.0),
            VelocityC::zeros(),
            1.0,
            1.0,
        )
    });

    let mut rigid_body_manager = RigidBodyManager::new();
    let mut constraint_manager = ConstraintManager::new(ConstraintSolverConfig::default());

    setup_bodies_and_run_constraints(
        &mut rigid_body_manager,
        &mut constraint_manager,
        spheres.clone(),
        [],
    );

    assert_eq!(constraint_manager.solver().prepared_contact_count(), 0);
    assert_eq!(constraint_manager.solver().prepared_body_count(), 0);

    for sphere in spheres {
        let id = DynamicRigidBodyID::from_entity_id(sphere.entity_id);
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

    setup_bodies_and_run_constraints(
        &mut rigid_body_manager,
        &mut constraint_manager,
        [sphere_a.clone(), sphere_b.clone()],
        [],
    );

    assert_eq!(constraint_manager.solver().prepared_contact_count(), 1);
    assert_eq!(constraint_manager.solver().prepared_body_count(), 2);

    let body_a_id = DynamicRigidBodyID::from_entity_id(sphere_a.entity_id);
    let body_a = rigid_body_manager.dynamic_rigid_body(body_a_id);
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

    let body_b_id = DynamicRigidBodyID::from_entity_id(sphere_b.entity_id);
    let body_b = rigid_body_manager.dynamic_rigid_body(body_b_id);
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

    let mut entity_id_manager = EntityIDManager::new();

    test_binary_sphere_collision(
        SphereBody::new(
            entity_id_manager.provide_id(),
            SphereC::new(PositionC::origin(), radius),
            Vector3C::new(speed, 0.0, 0.0),
            mass_density,
            restitution,
        ),
        SphereBody::new(
            entity_id_manager.provide_id(),
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

    let mut entity_id_manager = EntityIDManager::new();

    test_binary_sphere_collision(
        SphereBody::new(
            entity_id_manager.provide_id(),
            SphereC::new(PositionC::origin(), radius),
            Vector3C::new(speed, 0.0, 0.0),
            1.0,
            restitution,
        ),
        SphereBody::new(
            entity_id_manager.provide_id(),
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

    let mut entity_id_manager = EntityIDManager::new();

    test_binary_sphere_collision(
        SphereBody::new(
            entity_id_manager.provide_id(),
            SphereC::new(PositionC::origin(), radius),
            Vector3C::new(speed, 0.0, 0.0),
            mass_density,
            restitution,
        ),
        SphereBody::new(
            entity_id_manager.provide_id(),
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

    let mut entity_id_manager = EntityIDManager::new();

    test_binary_sphere_collision(
        SphereBody::new(
            entity_id_manager.provide_id(),
            SphereC::new(Point3C::new(1e-6, 0.0, 0.0), radius),
            Vector3C::new(speed, 0.0, 0.0),
            mass_density,
            restitution,
        ),
        SphereBody::new(
            entity_id_manager.provide_id(),
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

    let mut entity_id_manager = EntityIDManager::new();

    let sphere = SphereBody::new(
        entity_id_manager.provide_id(),
        SphereC::new(Point3C::new(0.0, radius - 1e-6, 0.0), radius),
        Vector3C::new(speed_x, -speed_y, 0.0),
        1.0,
        restitution_coef,
    );
    let plane = PlaneBody::new(
        entity_id_manager.provide_id(),
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

    setup_bodies_and_run_constraints(
        &mut rigid_body_manager,
        &mut constraint_manager,
        [sphere.clone()],
        [plane],
    );

    assert_eq!(constraint_manager.solver().prepared_contact_count(), 1);
    assert_eq!(constraint_manager.solver().prepared_body_count(), 2);

    let body_id = DynamicRigidBodyID::from_entity_id(sphere.entity_id);
    let body = rigid_body_manager.dynamic_rigid_body(body_id);
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

    let mut entity_id_manager = EntityIDManager::new();

    let spheres = [
        SphereBody::new(
            entity_id_manager.provide_id(),
            SphereC::new(Point3C::new(0.5 * penetration, 0.0, 0.0), radius),
            VelocityC::zeros(),
            mass_density,
            restitution_coef,
        ),
        SphereBody::new(
            entity_id_manager.provide_id(),
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

    setup_bodies_and_run_constraints(
        &mut rigid_body_manager,
        &mut constraint_manager,
        spheres.clone(),
        [],
    );

    assert_eq!(constraint_manager.solver().prepared_contact_count(), 1);
    assert_eq!(constraint_manager.solver().prepared_body_count(), 2);

    for (idx, sphere) in spheres.into_iter().enumerate() {
        let id = DynamicRigidBodyID::from_entity_id(sphere.entity_id);
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
