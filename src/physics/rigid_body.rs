//! Rigid body simulation.

mod components;
mod forces;
mod systems;

pub use components::{RigidBodyComp, UniformRigidBodyComp};
pub use forces::{RigidBodyForceManager, UniformGravityComp};

use crate::physics::{
    self, fph, AngularMomentum, AngularVelocity, Force, InertialProperties, Momentum, Orientation,
    Position, Torque, Velocity,
};
use approx::AbsDiffEq;
use bytemuck::{Pod, Zeroable};

/// A rigid body.
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct RigidBody {
    inertial_properties: InertialProperties,
    center_of_mass: Position,
    momentum: Momentum,
    angular_momentum: AngularMomentum,
    total_force: Force,
    total_torque: Torque,
}

impl RigidBody {
    /// Creates a new rigid body based on the given inertial properties,
    /// position, orientation, velocity and angular velocity. The position is
    /// assumed to be the origin of the body's reference frame in world space,
    /// which could be different from the center of mass.
    pub fn new(
        inertial_properties: InertialProperties,
        position: &Position,
        orientation: &Orientation,
        velocity: &Velocity,
        angular_velocity: &AngularVelocity,
    ) -> Self {
        let center_of_mass =
            Self::compute_center_of_mass(&inertial_properties, position, orientation);

        Self::new_with_center_of_mass(
            inertial_properties,
            center_of_mass,
            orientation,
            velocity,
            angular_velocity,
        )
    }

    /// Creates a new rigid body with the given inertial properties, center of
    /// mass, velocity and angular velocity.
    pub fn new_with_center_of_mass(
        inertial_properties: InertialProperties,
        center_of_mass: Position,
        orientation: &Orientation,
        velocity: &Velocity,
        angular_velocity: &AngularVelocity,
    ) -> Self {
        let momentum = Self::compute_momentum(&inertial_properties, velocity);
        let angular_momentum =
            Self::compute_angular_momentum(&inertial_properties, orientation, angular_velocity);
        Self {
            inertial_properties,
            center_of_mass,
            momentum,
            angular_momentum,
            total_force: Force::zeros(),
            total_torque: Torque::zeros(),
        }
    }

    /// Returns the mass of the body.
    pub fn mass(&self) -> fph {
        self.inertial_properties.mass()
    }

    /// Returns the center of mass of the body (in world space).
    pub fn center_of_mass(&self) -> &Position {
        &self.center_of_mass
    }

    /// Returns the momentum of the body.
    pub fn momentum(&self) -> &Momentum {
        &self.momentum
    }

    /// Returns the angular momentum of the body.
    pub fn angular_momentum(&self) -> &AngularMomentum {
        &self.angular_momentum
    }

    /// Returns the current total force on the body.
    pub fn total_force(&self) -> &Force {
        &self.total_force
    }

    /// Returns the current total torque on the body around the center of mass.
    pub fn total_torque(&self) -> &Torque {
        &self.total_torque
    }

    /// Applies the given force at the body's center of mass.
    pub fn apply_force_at_center_of_mass(&mut self, force: &Force) {
        self.total_force += force;
    }

    /// Applies the given torque around the body's center of mass.
    pub fn apply_torque(&mut self, torque: &Torque) {
        self.total_torque += torque;
    }

    /// Applies the given force at the given position. This may result in a
    /// torque around the center of mass.
    pub fn apply_force(&mut self, force: &Force, position: &Position) {
        self.apply_force_at_center_of_mass(force);
        self.apply_torque(&(position - self.center_of_mass).cross(force));
    }

    /// Recomputes the body's center of mass according to the given position and
    /// orientation.
    pub fn synchronize_center_of_mass(&mut self, position: &Position, orientation: &Orientation) {
        self.center_of_mass =
            Self::compute_center_of_mass(&self.inertial_properties, position, orientation);
    }

    /// Recomputes the body's momentum according to the given velocity.
    pub fn synchronize_momentum(&mut self, velocity: &Velocity) {
        self.momentum = Self::compute_momentum(&self.inertial_properties, velocity);
    }

    /// Recomputes the body's angular momentum according to the given
    /// orientation and angular velocity.
    pub fn synchronize_angular_momentum(
        &mut self,
        orientation: &Orientation,
        angular_velocity: &AngularVelocity,
    ) {
        self.angular_momentum = Self::compute_angular_momentum(
            &self.inertial_properties,
            orientation,
            angular_velocity,
        );
    }

    /// Advances the given motion-defining properties of the body for the given
    /// duration based on the current total force and torque. Updates the stored
    /// center of mass, momentum and angular momentum accordingly.
    ///
    /// If the given position, orientation, velocity or angular velocity have
    /// been modified after being returned from a previous call to this
    /// function, make sure to call [`synchronize_center_of_mass`],
    /// [`synchronize_momentum`] and/or [`synchronize_angular_momentum`]
    /// accordingly befor calling this function.
    ///
    /// This function resets the total force and torque.
    pub fn advance_motion(
        &mut self,
        position: &mut Position,
        orientation: &mut Orientation,
        velocity: &mut Velocity,
        angular_velocity: &mut AngularVelocity,
        duration: fph,
    ) {
        Self::advance_linear_motion(
            &self.inertial_properties,
            &self.total_force,
            &mut self.momentum,
            velocity,
            &mut self.center_of_mass,
            duration,
        );

        Self::advance_rotational_motion(
            &self.inertial_properties,
            &self.total_torque,
            &mut self.angular_momentum,
            angular_velocity,
            orientation,
            duration,
        );

        *position =
            Self::compute_position(&self.inertial_properties, &self.center_of_mass, orientation);

        self.reset_total_force();
        self.reset_total_torque();
    }

    fn reset_total_force(&mut self) {
        self.total_force = Force::zeros();
    }

    fn reset_total_torque(&mut self) {
        self.total_torque = Torque::zeros();
    }

    fn advance_linear_motion(
        inertial_properties: &InertialProperties,
        total_force: &Force,
        momentum: &mut Momentum,
        velocity: &mut Velocity,
        center_of_mass: &mut Position,
        duration: fph,
    ) {
        *momentum += total_force * duration;
        *velocity = Self::compute_velocity(inertial_properties, momentum);
        *center_of_mass += *velocity * duration;
    }

    fn advance_rotational_motion(
        inertial_properties: &InertialProperties,
        total_torque: &Torque,
        angular_momentum: &mut AngularMomentum,
        angular_velocity: &mut AngularVelocity,
        orientation: &mut Orientation,
        duration: fph,
    ) {
        *angular_momentum += total_torque * duration;
        *angular_velocity =
            Self::compute_angular_velocity(inertial_properties, orientation, angular_momentum);
        *orientation = physics::advance_orientation(orientation, angular_velocity, duration);
    }

    /// Computes the center of mass in world space.
    fn compute_center_of_mass(
        inertial_properties: &InertialProperties,
        position: &Position,
        orientation: &Orientation,
    ) -> Position {
        position + orientation.transform_vector(&inertial_properties.center_of_mass().coords)
    }

    /// Computes the body position (the origin of the body's reference frame in
    /// world space).
    fn compute_position(
        inertial_properties: &InertialProperties,
        center_of_mass: &Position,
        orientation: &Orientation,
    ) -> Position {
        center_of_mass - orientation.transform_vector(&inertial_properties.center_of_mass().coords)
    }

    fn compute_momentum(inertial_properties: &InertialProperties, velocity: &Velocity) -> Momentum {
        inertial_properties.mass() * velocity
    }

    fn compute_velocity(inertial_properties: &InertialProperties, momentum: &Momentum) -> Velocity {
        inertial_properties.inverse_mass() * momentum
    }

    fn compute_angular_velocity(
        inertial_properties: &InertialProperties,
        orientation: &Orientation,
        angular_momentum: &AngularMomentum,
    ) -> AngularVelocity {
        let inverse_world_space_inertia_tensor = inertial_properties
            .inertia_tensor()
            .inverse_rotated_matrix(orientation);

        AngularVelocity::new_from_vector(inverse_world_space_inertia_tensor * angular_momentum)
    }

    fn compute_angular_momentum(
        inertial_properties: &InertialProperties,
        orientation: &Orientation,
        angular_velocity: &AngularVelocity,
    ) -> AngularMomentum {
        inertial_properties
            .inertia_tensor()
            .rotated_matrix(orientation)
            * angular_velocity.as_vector()
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
            && Force::abs_diff_eq(&self.total_force, &other.total_force, epsilon)
            && Torque::abs_diff_eq(&self.total_torque, &other.total_torque, epsilon)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{geometry::Degrees, num::Float, physics::InertiaTensor};
    use approx::{abs_diff_eq, assert_abs_diff_eq, assert_abs_diff_ne};
    use nalgebra::{point, vector, Vector3};
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
        fn force_strategy(max_force_coord: fph)(
            force_coord_x in -max_force_coord..max_force_coord,
            force_coord_y in -max_force_coord..max_force_coord,
            force_coord_z in -max_force_coord..max_force_coord,
        ) -> Force {
            vector![force_coord_x, force_coord_y, force_coord_z]
        }
    }

    prop_compose! {
        fn torque_strategy(max_torque_coord: fph)(
            torque_coord_x in -max_torque_coord..max_torque_coord,
            torque_coord_y in -max_torque_coord..max_torque_coord,
            torque_coord_z in -max_torque_coord..max_torque_coord,
        ) -> Force {
            vector![torque_coord_x, torque_coord_y, torque_coord_z]
        }
    }

    fn dummy_inertial_properties() -> InertialProperties {
        InertialProperties::of_uniform_box(1.0, 1.0, 1.0, 1.0)
    }

    fn dummy_rigid_body() -> RigidBody {
        RigidBody::new_with_center_of_mass(
            dummy_inertial_properties(),
            Position::origin(),
            &Orientation::identity(),
            &Velocity::zeros(),
            &AngularVelocity::new(Vector3::y_axis(), Degrees(0.0)),
        )
    }

    fn dummy_rigid_body_with_center_of_mass(center_of_mass: Position) -> RigidBody {
        RigidBody::new_with_center_of_mass(
            dummy_inertial_properties(),
            center_of_mass,
            &Orientation::identity(),
            &Velocity::zeros(),
            &AngularVelocity::new(Vector3::y_axis(), Degrees(0.0)),
        )
    }

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
                &position,
                &orientation,
                &Velocity::zeros(),
                &AngularVelocity::new(Vector3::y_axis(), Degrees(0.0)),
            );
            prop_assert!(abs_diff_eq!(
                body.center_of_mass(),
                &(position + orientation.transform_vector(&model_space_center_of_mass.coords))
            ));
        }
    }

    proptest! {
        #[test]
        fn should_add_forces_applied_at_center_of_mass(
            force_1 in force_strategy(1e3),
            force_2 in force_strategy(1e3),
        ) {
            let mut body = dummy_rigid_body();
            body.apply_force_at_center_of_mass(&force_1);
            body.apply_force_at_center_of_mass(&force_2);
            prop_assert!(abs_diff_eq!(body.total_force(), &(force_1 + force_2)));
        }
    }

    proptest! {
        #[test]
        fn should_add_forces_applied_anywhere(
            center_of_mass in position_strategy(1e3),
            force_1 in force_strategy(1e3),
            force_2 in force_strategy(1e3),
            force_position_1 in position_strategy(1e3),
            force_position_2 in position_strategy(1e3),
        ) {
            let mut body = dummy_rigid_body_with_center_of_mass(center_of_mass);
            body.apply_force(&force_1, &force_position_1);
            body.apply_force(&force_2, &force_position_2);
            prop_assert!(abs_diff_eq!(body.total_force(), &(force_1 + force_2)));
        }
    }

    proptest! {
        #[test]
        fn should_add_torques_applied_around_center_of_mass(
            torque_1 in torque_strategy(1e3),
            torque_2 in torque_strategy(1e3),
        ) {
            let mut body = dummy_rigid_body();
            body.apply_torque(&torque_1);
            body.apply_torque(&torque_2);
            prop_assert!(abs_diff_eq!(body.total_torque(), &(torque_1 + torque_2)));
        }
    }

    proptest! {
        #[test]
        fn should_get_torque_from_applying_force_outside_center_of_mass(
            center_of_mass in position_strategy(1e3),
            force in force_strategy(1e3),
            force_position in position_strategy(1e3),
        ) {
            let mut body = dummy_rigid_body_with_center_of_mass(center_of_mass);
            body.apply_force(&force, &force_position);
            prop_assert!(abs_diff_eq!(
                body.total_torque(),
                &((force_position - center_of_mass).cross(&force))
            ));
        }
    }

    #[test]
    fn should_reset_force_and_torque_after_advancing_motion() {
        let mut body = dummy_rigid_body();

        body.apply_force(&Force::x(), &point![0.0, 1.0, 0.0]);

        let mut position = Position::origin();
        let mut orientation = Orientation::identity();
        let mut velocity = Velocity::zeros();
        let mut angular_velocity = AngularVelocity::zero();

        body.advance_motion(
            &mut position,
            &mut orientation,
            &mut velocity,
            &mut angular_velocity,
            1.0,
        );

        assert_abs_diff_eq!(body.total_force(), &Force::zeros());
        assert_abs_diff_eq!(body.total_torque(), &Torque::zeros());
    }

    #[test]
    fn should_retain_motion_when_advancing_for_zero_time() {
        let original_position = Position::origin();
        let original_orientation = Orientation::identity();
        let original_velocity = Velocity::z();
        let original_angular_velocity = AngularVelocity::new_from_vector(Vector3::x());

        let mut body = RigidBody::new_with_center_of_mass(
            dummy_inertial_properties(),
            original_position.clone(),
            &original_orientation,
            &original_velocity,
            &original_angular_velocity,
        );

        body.apply_force(&Force::x(), &point![0.0, 1.0, 0.0]);

        let mut position = original_position.clone();
        let mut orientation = original_orientation.clone();
        let mut velocity = original_velocity.clone();
        let mut angular_velocity = original_angular_velocity.clone();

        body.advance_motion(
            &mut position,
            &mut orientation,
            &mut velocity,
            &mut angular_velocity,
            0.0,
        );

        assert_abs_diff_eq!(position, original_position);
        assert_abs_diff_eq!(orientation, original_orientation);
        assert_abs_diff_eq!(velocity, original_velocity);
        assert_abs_diff_eq!(angular_velocity, original_angular_velocity);
    }

    #[test]
    fn should_retain_motion_with_zero_force_and_velocity() {
        let original_position = Position::origin();
        let original_orientation = Orientation::identity();
        let original_velocity = Velocity::zeros();
        let original_angular_velocity = AngularVelocity::zero();

        let mut body = RigidBody::new_with_center_of_mass(
            dummy_inertial_properties(),
            original_position.clone(),
            &original_orientation,
            &original_velocity,
            &original_angular_velocity,
        );

        let mut position = original_position.clone();
        let mut orientation = original_orientation.clone();
        let mut velocity = original_velocity.clone();
        let mut angular_velocity = original_angular_velocity.clone();

        body.advance_motion(
            &mut position,
            &mut orientation,
            &mut velocity,
            &mut angular_velocity,
            1.0,
        );

        assert_abs_diff_eq!(position, original_position);
        assert_abs_diff_eq!(orientation, original_orientation);
        assert_abs_diff_eq!(velocity, original_velocity);
        assert_abs_diff_eq!(angular_velocity, original_angular_velocity);
    }

    #[test]
    fn should_change_motion_with_nonzero_force_and_velocity() {
        let original_position = Position::origin();
        let original_orientation = Orientation::identity();
        let original_velocity = Velocity::z();
        let original_angular_velocity = AngularVelocity::new_from_vector(Vector3::x());

        let mut body = RigidBody::new_with_center_of_mass(
            dummy_inertial_properties(),
            original_position.clone(),
            &original_orientation,
            &original_velocity,
            &original_angular_velocity,
        );

        body.apply_force(&Force::x(), &point![0.0, 1.0, 0.0]);

        let mut position = original_position.clone();
        let mut orientation = original_orientation.clone();
        let mut velocity = original_velocity.clone();
        let mut angular_velocity = original_angular_velocity.clone();

        body.advance_motion(
            &mut position,
            &mut orientation,
            &mut velocity,
            &mut angular_velocity,
            1.0,
        );

        assert_abs_diff_ne!(position, original_position);
        assert_abs_diff_ne!(orientation, original_orientation);
        assert_abs_diff_ne!(velocity, original_velocity);
        assert_abs_diff_ne!(angular_velocity, original_angular_velocity);
    }
}
