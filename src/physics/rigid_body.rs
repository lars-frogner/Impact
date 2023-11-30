//! Rigid body simulation.

mod components;

pub use components::{RigidBodyComp, UniformRigidBodyComp};

use crate::physics::{
    fph, AngularVelocity, Force, InertialProperties, Orientation, Position, Torque, Velocity,
};
use approx::AbsDiffEq;
use bytemuck::{Pod, Zeroable};
use impact_utils::KeyIndexMapper;

/// Identifier for a [`RigidBody`].
#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Zeroable, Pod)]
pub struct RigidBodyID(u64);

/// Manager of all [`RigidBody`]s.
#[derive(Debug)]
pub struct RigidBodyManager {
    bodies: Vec<RigidBody>,
    key_map: KeyIndexMapper<RigidBodyID>,
    rigid_body_id_counter: u64,
}

/// A rigid body.
#[derive(Clone, Debug, PartialEq)]
pub struct RigidBody {
    inertial_properties: InertialProperties,
    center_of_mass: Position,
    orientation: Orientation,
    velocity: Velocity,
    angular_velocity: AngularVelocity,
    total_force: Force,
    total_torque: Torque,
}

impl RigidBodyManager {
    /// Creates a new rigid body manager.
    pub fn new() -> Self {
        Self {
            bodies: Vec::new(),
            key_map: KeyIndexMapper::new(),
            rigid_body_id_counter: 0,
        }
    }

    /// Returns the number of managed rigid bodies.
    pub fn n_bodies(&self) -> usize {
        self.bodies.len()
    }

    /// Returns a reference to the [`RigidBody`] with the given ID.
    ///
    /// # Panics
    /// If no rigid body with the given ID exists.
    pub fn rigid_body(&self, rigid_body_id: RigidBodyID) -> &RigidBody {
        let idx = self.key_map.idx(rigid_body_id);
        &self.bodies[idx]
    }

    /// Returns a reference to the [`RigidBody`] with the given ID, or [`None`]
    /// if rigid body with the given ID exists.
    pub fn get_rigid_body(&self, rigid_body_id: RigidBodyID) -> Option<&RigidBody> {
        self.key_map.get(rigid_body_id).map(|idx| &self.bodies[idx])
    }

    /// Includes the given [`RigidBody`] in the manager.
    ///
    /// # Returns
    /// An identifier that can be used to access the rigid body.
    pub fn include_rigid_body(&mut self, body: RigidBody) -> RigidBodyID {
        let id = self.generate_new_rigid_body_id();
        self.key_map.push_key(id);
        self.bodies.push(body);
        id
    }

    /// Removes the [`RigidBody`] with the given ID from the manager.
    ///
    /// # Panics
    /// If no rigid body with the given ID exists.
    pub fn remove_rigid_body(&mut self, rigid_body_id: RigidBodyID) {
        let idx = self.key_map.swap_remove_key(rigid_body_id);
        self.bodies.swap_remove(idx);
    }

    fn generate_new_rigid_body_id(&mut self) -> RigidBodyID {
        let id = RigidBodyID(self.rigid_body_id_counter);
        self.rigid_body_id_counter += 1;
        id
    }
}

impl RigidBody {
    /// Creates a new rigid body with the given inertial properties, position,
    /// orientation, velocity and angular velocity. The position is assumed to
    /// be the origin of the body's reference frame in world space, which could
    /// be different from the center of mass.
    pub fn new(
        inertial_properties: InertialProperties,
        position: Position,
        orientation: Orientation,
        velocity: Velocity,
        angular_velocity: AngularVelocity,
    ) -> Self {
        // Compute center of mass in world space
        let center_of_mass =
            position + orientation.transform_vector(&inertial_properties.center_of_mass().coords);

        Self::new_with_center_of_mass(
            inertial_properties,
            center_of_mass,
            orientation,
            velocity,
            angular_velocity,
        )
    }

    /// Creates a new rigid body with the given inertial properties, center of
    /// mass, orientation, velocity and angular velocity.
    pub fn new_with_center_of_mass(
        inertial_properties: InertialProperties,
        center_of_mass: Position,
        orientation: Orientation,
        velocity: Velocity,
        angular_velocity: AngularVelocity,
    ) -> Self {
        Self {
            inertial_properties,
            center_of_mass,
            orientation,
            velocity,
            angular_velocity,
            total_force: Force::zeros(),
            total_torque: Torque::zeros(),
        }
    }

    /// Returns the center of mass of the body (in world space).
    pub fn center_of_mass(&self) -> &Position {
        &self.center_of_mass
    }

    /// Returns the current total force on the body.
    pub fn total_force(&self) -> &Force {
        &self.total_force
    }

    /// Returns the current total torque on the body around the center of mass.
    pub fn total_torque(&self) -> &Torque {
        &self.total_torque
    }
}

impl AbsDiffEq for RigidBody {
    type Epsilon = <fph as AbsDiffEq>::Epsilon;

    fn default_epsilon() -> Self::Epsilon {
        fph::default_epsilon()
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        InertialProperties::abs_diff_eq(
            &self.inertial_properties,
            &other.inertial_properties,
            epsilon,
        ) && Position::abs_diff_eq(&self.center_of_mass, &other.center_of_mass, epsilon)
            && Orientation::abs_diff_eq(&self.orientation, &other.orientation, epsilon)
            && Velocity::abs_diff_eq(&self.velocity, &other.velocity, epsilon)
            && AngularVelocity::abs_diff_eq(
                &self.angular_velocity,
                &other.angular_velocity,
                epsilon,
            )
            && Force::abs_diff_eq(&self.total_force, &other.total_force, epsilon)
            && Torque::abs_diff_eq(&self.total_torque, &other.total_torque, epsilon)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        geometry::{Degrees, Radians},
        num::Float,
    };
    use nalgebra::{point, vector, UnitVector3, Vector3};
    use proptest::prelude::*;

    prop_compose! {
        fn position_strategy(max_position_coord: fph)(
            position_coord_x in -max_position_coord..max_position_coord,
            position_coord_y in -max_position_coord..max_position_coord,
            position_coord_z in -max_position_coord..max_position_coord,
        ) -> Position {
            point![position_coord_x, position_coord_y, position_coord_z]
        }
    }

    prop_compose! {
        fn orientation_strategy()(
            rotation_roll in 0.0..fph::TWO_PI,
            rotation_pitch in -fph::FRAC_PI_2..fph::FRAC_PI_2,
            rotation_yaw in 0.0..fph::TWO_PI,
        ) -> Orientation {
            Orientation::from_euler_angles(rotation_roll, rotation_pitch, rotation_yaw)
        }
    }

    prop_compose! {
        fn velocity_strategy(max_velocity_coord: fph)(
            velocity_coord_x in -max_velocity_coord..max_velocity_coord,
            velocity_coord_y in -max_velocity_coord..max_velocity_coord,
            velocity_coord_z in -max_velocity_coord..max_velocity_coord,
        ) -> Velocity {
            vector![velocity_coord_x, velocity_coord_y, velocity_coord_z]
        }
    }

    prop_compose! {
        fn angular_velocity_strategy(max_angular_speed: fph)(
            angular_speed in 0.0..max_angular_speed,
            direction_phi in 0.0..fph::TWO_PI,
            direction_theta in -fph::FRAC_PI_2..fph::FRAC_PI_2,
        ) -> AngularVelocity {
            AngularVelocity::new(
                UnitVector3::new_normalize(
                    vector![
                        fph::cos(direction_phi) * fph::sin(direction_theta),
                        fph::sin(direction_phi) * fph::sin(direction_theta),
                        fph::cos(direction_theta)
                    ]
                ), Radians(angular_speed)
            )
        }
    }

    fn dummy_rigid_body() -> RigidBody {
        RigidBody::new_with_center_of_mass(
            InertialProperties::of_uniform_box(1.0, 1.0, 1.0, 1.0),
            Position::origin(),
            Orientation::identity(),
            Velocity::zeros(),
            AngularVelocity::new(Vector3::y_axis(), Degrees(0.0)),
        )
    }

    fn dummy_rigid_body_2() -> RigidBody {
        RigidBody::new_with_center_of_mass(
            InertialProperties::of_uniform_sphere(1.0),
            Position::origin(),
            Orientation::identity(),
            Velocity::zeros(),
            AngularVelocity::new(Vector3::y_axis(), Degrees(0.0)),
        )
    }

    mod test_manager {
        use super::*;
        use approx::assert_abs_diff_eq;

        #[test]
        fn should_get_zero_bodies_in_new_manager() {
            let manager = RigidBodyManager::new();
            assert_eq!(manager.n_bodies(), 0);
        }

        #[test]
        fn should_get_one_body_after_including_one() {
            let mut manager = RigidBodyManager::new();
            manager.include_rigid_body(dummy_rigid_body());
            assert_eq!(manager.n_bodies(), 1);
        }

        #[test]
        fn should_get_different_ids_for_two_included_bodies() {
            let mut manager = RigidBodyManager::new();
            let id_1 = manager.include_rigid_body(dummy_rigid_body());
            let id_2 = manager.include_rigid_body(dummy_rigid_body());
            assert_ne!(id_1, id_2);
        }

        #[test]
        fn should_get_same_body_as_included() {
            let body_1 = dummy_rigid_body();
            let body_2 = dummy_rigid_body_2();
            let mut manager = RigidBodyManager::new();
            let id_1 = manager.include_rigid_body(body_1.clone());
            let id_2 = manager.include_rigid_body(body_2.clone());
            assert_abs_diff_eq!(manager.rigid_body(id_1), &body_1);
            assert_abs_diff_eq!(manager.rigid_body(id_2), &body_2);
            assert_abs_diff_eq!(manager.get_rigid_body(id_1).unwrap(), &body_1);
            assert_abs_diff_eq!(manager.get_rigid_body(id_2).unwrap(), &body_2);
        }

        #[test]
        fn should_get_zero_bodies_after_removing_all() {
            let mut manager = RigidBodyManager::new();
            let id = manager.include_rigid_body(dummy_rigid_body());
            manager.remove_rigid_body(id);
            assert_eq!(manager.n_bodies(), 0);
        }

        #[test]
        fn should_remove_correct_body() {
            let body_1 = dummy_rigid_body();
            let body_2 = dummy_rigid_body_2();
            let mut manager = RigidBodyManager::new();
            let id_1 = manager.include_rigid_body(body_1.clone());
            let id_2 = manager.include_rigid_body(body_2);
            let id_3 = manager.include_rigid_body(body_1.clone());
            manager.remove_rigid_body(id_2);
            assert_eq!(manager.n_bodies(), 2);
            assert_abs_diff_eq!(manager.rigid_body(id_1), &body_1);
            assert_abs_diff_eq!(manager.rigid_body(id_3), &body_1);
        }

        #[test]
        #[should_panic]
        fn should_panic_when_accessing_removed_body() {
            let mut manager = RigidBodyManager::new();
            let id = manager.include_rigid_body(dummy_rigid_body());
            manager.remove_rigid_body(id);
            manager.rigid_body(id);
        }

        #[test]
        fn should_get_none_when_getting_removed_body() {
            let mut manager = RigidBodyManager::new();
            let id = manager.include_rigid_body(dummy_rigid_body());
            manager.remove_rigid_body(id);
            assert!(manager.get_rigid_body(id).is_none());
        }

        #[test]
        #[should_panic]
        fn should_panic_when_removing_same_body_twice() {
            let mut manager = RigidBodyManager::new();
            let id = manager.include_rigid_body(dummy_rigid_body());
            manager.remove_rigid_body(id);
            manager.remove_rigid_body(id);
        }
    }

    mod test_rigid_body {
        use super::*;
        use crate::physics::InertiaTensor;
        use approx::assert_abs_diff_eq;

        #[test]
        fn should_get_zero_force_and_torque_for_new_body() {
            let body = dummy_rigid_body();
            assert_abs_diff_eq!(body.total_force(), &Force::zeros());
            assert_abs_diff_eq!(body.total_torque(), &Torque::zeros());
        }

        proptest! {
            #[test]
            fn should_get_correct_center_of_mass_from_position(
                model_space_center_of_mass in position_strategy(1e3),
                position in position_strategy(1e3),
                orientation in orientation_strategy(),
            ) {
                let body = RigidBody::new(
                    InertialProperties::new(1.0, model_space_center_of_mass, InertiaTensor::identity()),
                    position,
                    orientation,
                    Velocity::zeros(),
                    AngularVelocity::new(Vector3::y_axis(), Degrees(0.0)),
                );
                assert_abs_diff_eq!(
                    body.center_of_mass(),
                    &(position + orientation.transform_vector(&model_space_center_of_mass.coords))
                );
            }
        }
    }
}
