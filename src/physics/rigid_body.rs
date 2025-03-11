//! Rigid body simulation.

pub mod components;
pub mod entity;
pub mod forces;
pub mod systems;

use crate::physics::{
    fph,
    inertia::InertialProperties,
    motion::{
        self, AngularMomentum, AngularVelocity, Force, Momentum, Orientation, Position, Torque,
        Velocity,
    },
};
use approx::AbsDiffEq;
use bytemuck::{Pod, Zeroable};

/// A rigid body.
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct RigidBody {
    inertial_properties: InertialProperties,
    position: Position,
    orientation: Orientation,
    momentum: Momentum,
    angular_momentum: AngularMomentum,
    total_force: Force,
    total_torque: Torque,
}

impl RigidBody {
    /// Creates a new rigid body with the given inertial properties, position
    /// (center of mass), orientation, scale factor, velocity and angular
    /// velocity.
    pub fn new(
        inertial_properties: InertialProperties,
        position: Position,
        orientation: Orientation,
        scaling: fph,
        velocity: &Velocity,
        angular_velocity: &AngularVelocity,
    ) -> Self {
        let momentum = Self::compute_momentum_from_velocity(&inertial_properties, velocity);
        let angular_momentum = Self::compute_angular_momentum_from_angular_velocity(
            &inertial_properties,
            &orientation,
            scaling,
            angular_velocity,
        );
        Self {
            inertial_properties,
            position,
            orientation,
            momentum,
            angular_momentum,
            total_force: Force::zeros(),
            total_torque: Torque::zeros(),
        }
    }

    /// Returns the inertial properties of the body.
    pub fn inertial_properties(&self) -> &InertialProperties {
        &self.inertial_properties
    }

    /// Returns the mass of the body.
    pub fn mass(&self) -> fph {
        self.inertial_properties.mass()
    }

    /// Returns the position (center of mass) of the body.
    pub fn position(&self) -> &Position {
        &self.position
    }

    /// Returns the orientation of the body.
    pub fn orientation(&self) -> &Orientation {
        &self.orientation
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

    /// Computes the linear velocity of the body.
    pub fn compute_velocity(&self) -> Velocity {
        Self::compute_velocity_from_momentum(&self.inertial_properties, &self.momentum)
    }

    /// Computes the angular velocity of the body.
    pub fn compute_angular_velocity(&self, scaling: fph) -> AngularVelocity {
        Self::compute_angular_velocity_from_angular_momentum(
            &self.inertial_properties,
            &self.orientation,
            scaling,
            &self.angular_momentum,
        )
    }

    /// Computes the total kinetic energy (translational and rotational) of the
    /// body.
    pub fn compute_kinetic_energy(&self, scaling: fph) -> fph {
        self.compute_translational_kinetic_energy()
            + self.compute_rotational_kinetic_energy(scaling)
    }

    /// Computes the translational kinetic energy of the body.
    pub fn compute_translational_kinetic_energy(&self) -> fph {
        0.5 * self.mass() * self.compute_velocity().norm_squared()
    }

    /// Computes the rotational kinetic energy of the body.
    pub fn compute_rotational_kinetic_energy(&self, scaling: fph) -> fph {
        0.5 * self
            .compute_angular_velocity(scaling)
            .as_vector()
            .dot(&self.angular_momentum)
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
        self.apply_torque(&(position - self.position).cross(force));
    }

    /// Sets the given inertial properties for the body.
    pub fn update_inertial_properties(&mut self, inertial_properties: InertialProperties) {
        self.inertial_properties = inertial_properties;
    }

    /// Sets the given position for the body.
    pub fn update_position(&mut self, position: Position) {
        self.position = position;
    }

    /// Sets the given orientation for the body.
    pub fn update_orientation(&mut self, orientation: Orientation) {
        self.orientation = orientation;
    }

    /// Recomputes the body's momentum according to the given velocity.
    pub fn synchronize_momentum(&mut self, velocity: &Velocity) {
        self.momentum = Self::compute_momentum_from_velocity(&self.inertial_properties, velocity);
    }

    /// Recomputes the body's angular momentum according to the given
    /// orientation, scaling and angular velocity.
    pub fn synchronize_angular_momentum(
        &mut self,
        orientation: &Orientation,
        scaling: fph,
        angular_velocity: &AngularVelocity,
    ) {
        self.angular_momentum = Self::compute_angular_momentum_from_angular_velocity(
            &self.inertial_properties,
            orientation,
            scaling,
            angular_velocity,
        );
    }

    /// Advances the body's linear and angular momentum for the given duration
    /// based on total force and torque applied to the body since
    /// [`reset_force_and_torque`](Self::reset_force_and_torque) was called.
    pub fn advance_momenta(&mut self, step_duration: fph) {
        self.momentum = self.momentum() + self.total_force() * step_duration;

        self.angular_momentum = self.angular_momentum() + self.total_torque() * step_duration;
    }

    /// Advances the body's position and orientation for the given duration
    /// based on the given linear and angular velocity.
    pub fn advance_configuration_with(
        &mut self,
        step_duration: fph,
        advanced_velocity: &Velocity,
        advanced_angular_velocity: &AngularVelocity,
    ) {
        self.position = self.position() + advanced_velocity * step_duration;

        self.orientation = motion::advance_orientation(
            self.orientation(),
            advanced_angular_velocity,
            step_duration,
        );
    }

    /// Resets the total applied force and torque to zero.
    pub fn reset_force_and_torque(&mut self) {
        self.reset_total_force();
        self.reset_total_torque();
    }

    fn reset_total_force(&mut self) {
        self.total_force = Force::zeros();
    }

    fn reset_total_torque(&mut self) {
        self.total_torque = Torque::zeros();
    }

    fn compute_momentum_from_velocity(
        inertial_properties: &InertialProperties,
        velocity: &Velocity,
    ) -> Momentum {
        inertial_properties.mass() * velocity
    }

    fn compute_velocity_from_momentum(
        inertial_properties: &InertialProperties,
        momentum: &Momentum,
    ) -> Velocity {
        momentum / inertial_properties.mass()
    }

    fn compute_angular_velocity_from_angular_momentum(
        inertial_properties: &InertialProperties,
        orientation: &Orientation,
        scaling: fph,
        angular_momentum: &AngularMomentum,
    ) -> AngularVelocity {
        let inverse_world_space_inertia_tensor = inertial_properties
            .inertia_tensor()
            .inverse_rotated_matrix_with_scaled_extent(orientation, scaling);

        AngularVelocity::from_vector(inverse_world_space_inertia_tensor * angular_momentum)
    }

    fn compute_angular_momentum_from_angular_velocity(
        inertial_properties: &InertialProperties,
        orientation: &Orientation,
        scaling: fph,
        angular_velocity: &AngularVelocity,
    ) -> AngularMomentum {
        inertial_properties
            .inertia_tensor()
            .rotated_matrix_with_scaled_extent(orientation, scaling)
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
        ) && Position::abs_diff_eq(&self.position, &other.position, epsilon)
            && Force::abs_diff_eq(&self.total_force, &other.total_force, epsilon)
            && Torque::abs_diff_eq(&self.total_torque, &other.total_torque, epsilon)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{geometry::Degrees, num::Float};
    use approx::{abs_diff_eq, assert_abs_diff_eq, assert_abs_diff_ne};
    use nalgebra::{Vector3, point, vector};
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
        RigidBody::new(
            dummy_inertial_properties(),
            Position::origin(),
            Orientation::identity(),
            1.0,
            &Velocity::zeros(),
            &AngularVelocity::new(Vector3::y_axis(), Degrees(0.0)),
        )
    }

    fn dummy_rigid_body_with_position(position: Position) -> RigidBody {
        RigidBody::new(
            dummy_inertial_properties(),
            position,
            Orientation::identity(),
            1.0,
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
            let mut body = dummy_rigid_body_with_position(center_of_mass);
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
            let mut body = dummy_rigid_body_with_position(center_of_mass);
            body.apply_force(&force, &force_position);
            prop_assert!(abs_diff_eq!(
                body.total_torque(),
                &((force_position - center_of_mass).cross(&force))
            ));
        }
    }

    #[test]
    fn should_retain_momenta_when_advancing_for_zero_time() {
        let velocity = Velocity::z();
        let angular_velocity = AngularVelocity::from_vector(Vector3::x());

        let mut body = RigidBody::new(
            dummy_inertial_properties(),
            Position::origin(),
            Orientation::identity(),
            1.0,
            &velocity,
            &angular_velocity,
        );

        body.apply_force(&Force::x(), &point![0.0, 1.0, 0.0]);

        body.advance_momenta(0.0);

        assert_abs_diff_eq!(body.compute_velocity(), velocity);
        assert_abs_diff_eq!(body.compute_angular_velocity(1.0), angular_velocity);
    }

    #[test]
    fn should_retain_configuration_when_advancing_for_zero_time() {
        let position = Position::origin();
        let orientation = Orientation::identity();
        let velocity = Velocity::z();
        let angular_velocity = AngularVelocity::from_vector(Vector3::x());

        let mut body = RigidBody::new(
            dummy_inertial_properties(),
            position,
            orientation,
            1.0,
            &velocity,
            &angular_velocity,
        );

        body.advance_configuration_with(0.0, &velocity, &angular_velocity);

        assert_abs_diff_eq!(body.position(), &position);
        assert_abs_diff_eq!(body.orientation(), &orientation);
    }

    #[test]
    fn should_retain_momenta_with_zero_force() {
        let velocity = Velocity::zeros();
        let angular_velocity = AngularVelocity::zero();

        let mut body = RigidBody::new(
            dummy_inertial_properties(),
            Position::origin(),
            Orientation::identity(),
            1.0,
            &velocity,
            &angular_velocity,
        );

        body.advance_momenta(1.0);

        assert_abs_diff_eq!(body.compute_velocity(), velocity);
        assert_abs_diff_eq!(body.compute_angular_velocity(1.0), angular_velocity);
    }

    #[test]
    fn should_retain_configuration_with_zero_velocity() {
        let position = Position::origin();
        let orientation = Orientation::identity();
        let velocity = Velocity::zeros();
        let angular_velocity = AngularVelocity::zero();

        let mut body = RigidBody::new(
            dummy_inertial_properties(),
            position,
            orientation,
            1.0,
            &velocity,
            &angular_velocity,
        );

        body.advance_configuration_with(1.0, &velocity, &angular_velocity);

        assert_abs_diff_eq!(body.position(), &position);
        assert_abs_diff_eq!(body.orientation(), &orientation);
    }

    #[test]
    fn should_change_momenta_with_nonzero_force() {
        let position = Position::origin();
        let orientation = Orientation::identity();
        let velocity = Velocity::z();
        let angular_velocity = AngularVelocity::from_vector(Vector3::x());

        let mut body = RigidBody::new(
            dummy_inertial_properties(),
            position,
            orientation,
            1.0,
            &velocity,
            &angular_velocity,
        );

        body.apply_force(&Force::x(), &point![0.0, 1.0, 0.0]);

        body.advance_momenta(1.0);

        assert_abs_diff_ne!(body.compute_velocity(), velocity);
        assert_abs_diff_ne!(body.compute_angular_velocity(1.0), angular_velocity);
    }

    #[test]
    fn should_change_configuration_with_nonzero_velocity() {
        let position = Position::origin();
        let orientation = Orientation::identity();
        let velocity = Velocity::z();
        let angular_velocity = AngularVelocity::from_vector(Vector3::x());

        let mut body = RigidBody::new(
            dummy_inertial_properties(),
            position,
            orientation,
            1.0,
            &velocity,
            &angular_velocity,
        );

        body.advance_configuration_with(1.0, &velocity, &angular_velocity);

        assert_abs_diff_ne!(body.position(), &position);
        assert_abs_diff_ne!(body.orientation(), &orientation);
    }
}
