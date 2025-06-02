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
use roc_integration::roc;

/// A rigid body. It holds its [`InertialProperties`], the total [`Force`] and
/// [`Torque`] it is subjected to as well as its [`Momentum`] and
/// [`AngularMomentum`]. To avoid replication of data, the body does not store
/// or manage its position, orientation, velocity and angular velocity. The
/// reason it stores its linear and angular momentum is that these are the
/// conserved quantities in free motion and thus should be the primary
/// variables in the simulation, with linear and angular velocity being derived
/// from them. This means that the body's linear or angular momentum has to be
/// updated whenever something manually modifies the linear or angular
/// velocity, respectively.
#[roc(parents = "Physics")]
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
pub struct RigidBody {
    inertial_properties: InertialProperties,
    momentum: Momentum,
    angular_momentum: AngularMomentum,
    total_force: Force,
    total_torque: Torque,
}

impl RigidBody {
    /// Creates a new rigid body with the given inertial properties. This is
    /// used together with the other properties to initialize the linear and
    /// angular momentum.
    pub fn new(
        inertial_properties: InertialProperties,
        orientation: Orientation,
        scaling: fph,
        velocity: &Velocity,
        angular_velocity: &AngularVelocity,
    ) -> Self {
        let momentum = velocity * inertial_properties.mass();
        let angular_momentum = motion::compute_angular_momentum(
            &inertial_properties,
            &orientation,
            scaling,
            angular_velocity,
        );
        Self {
            inertial_properties,
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

    /// Returns the linear momentum of the body.
    pub fn momentum(&self) -> &Momentum {
        &self.momentum
    }

    /// Returns the angular momentum of the body.
    pub fn angular_momentum(&self) -> &AngularMomentum {
        &self.angular_momentum
    }

    /// Computes the velocity of the body based on its momentum.
    pub fn compute_velocity(&self) -> Velocity {
        self.momentum / self.mass()
    }

    /// Computes the angular velocity of the body based on its angular
    /// momentum.
    pub fn compute_angular_velocity(
        &self,
        orientation: &Orientation,
        scaling: fph,
    ) -> AngularVelocity {
        motion::compute_angular_velocity(
            self.inertial_properties(),
            orientation,
            scaling,
            self.angular_momentum(),
        )
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
    pub fn apply_force(
        &mut self,
        center_of_mass: &Position,
        force: &Force,
        force_position: &Position,
    ) {
        self.apply_force_at_center_of_mass(force);
        self.apply_torque(&(force_position - center_of_mass).cross(force));
    }

    /// Sets the given inertial properties for the body.
    pub fn update_inertial_properties(&mut self, inertial_properties: InertialProperties) {
        self.inertial_properties = inertial_properties;
    }

    /// Recomputes the body's linear momentum according to the given
    /// velocity.
    pub fn synchronize_momentum(&mut self, velocity: &Velocity) {
        self.momentum = velocity * self.mass();
    }

    /// Recomputes the body's angular momentum according to the given
    /// orientation, scaling and angular velocity.
    pub fn synchronize_angular_momentum(
        &mut self,
        orientation: &Orientation,
        scaling: fph,
        angular_velocity: &AngularVelocity,
    ) {
        self.angular_momentum = motion::compute_angular_momentum(
            self.inertial_properties(),
            orientation,
            scaling,
            angular_velocity,
        );
    }

    /// Advances the linear momentum of the body based on the total force
    /// applied to the body since
    /// [`reset_total_force`](Self::reset_total_force) was called.
    pub fn advance_momentum(&mut self, step_duration: fph) {
        self.momentum += self.total_force() * step_duration;
    }

    /// Advances the angular momentum of the body based on the total torque
    /// applied to the body since
    /// [`reset_total_torque`](Self::reset_total_torque) was called.
    pub fn advance_angular_momentum(&mut self, step_duration: fph) {
        self.angular_momentum += self.total_torque() * step_duration;
    }

    /// Resets the total applied force and torque to zero.
    pub fn reset_force_and_torque(&mut self) {
        self.reset_total_force();
        self.reset_total_torque();
    }

    /// Resets the total applied force to zero.
    pub fn reset_total_force(&mut self) {
        self.total_force = Force::zeros();
    }

    /// Resets the total applied torque to zero.
    pub fn reset_total_torque(&mut self) {
        self.total_torque = Torque::zeros();
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
        ) && Force::abs_diff_eq(&self.total_force, &other.total_force, epsilon)
            && Torque::abs_diff_eq(&self.total_torque, &other.total_torque, epsilon)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Radians;
    use approx::{abs_diff_eq, assert_abs_diff_eq, assert_abs_diff_ne};
    use impact_math::Float;
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
            Orientation::identity(),
            1.0,
            &Velocity::zeros(),
            &AngularVelocity::new(Vector3::y_axis(), Radians(0.0)),
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
            let mut body = dummy_rigid_body();
            body.apply_force(&center_of_mass, &force_1, &force_position_1);
            body.apply_force(&center_of_mass, &force_2, &force_position_2);
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
            let mut body = dummy_rigid_body();
            body.apply_force(&center_of_mass, &force, &force_position);
            prop_assert!(abs_diff_eq!(
                body.total_torque(),
                &((force_position - center_of_mass).cross(&force))
            ));
        }
    }

    #[test]
    fn should_retain_velocities_when_advancing_for_zero_time() {
        let velocity = Velocity::z();
        let angular_velocity = AngularVelocity::from_vector(Vector3::x());

        let mut body = RigidBody::new(
            dummy_inertial_properties(),
            Orientation::identity(),
            1.0,
            &velocity,
            &angular_velocity,
        );

        body.apply_force(&Position::origin(), &Force::x(), &point![0.0, 1.0, 0.0]);

        body.advance_momentum(0.0);
        assert_abs_diff_eq!(body.compute_velocity(), velocity);

        body.advance_angular_momentum(0.0);
        assert_abs_diff_eq!(
            body.compute_angular_velocity(&Orientation::identity(), 1.0),
            angular_velocity
        );
    }

    #[test]
    fn should_retain_velocities_with_zero_force() {
        let velocity = Velocity::zeros();
        let angular_velocity = AngularVelocity::zero();

        let mut body = RigidBody::new(
            dummy_inertial_properties(),
            Orientation::identity(),
            1.0,
            &velocity,
            &angular_velocity,
        );

        body.advance_momentum(1.0);
        assert_abs_diff_eq!(body.compute_velocity(), velocity);

        body.advance_angular_momentum(1.0);
        assert_abs_diff_eq!(
            body.compute_angular_velocity(&Orientation::identity(), 1.0),
            angular_velocity
        );
    }

    #[test]
    fn should_change_velocities_with_nonzero_force_and_torque() {
        let position = Position::origin();
        let orientation = Orientation::identity();
        let velocity = Velocity::z();
        let angular_velocity = AngularVelocity::from_vector(Vector3::x());

        let mut body = RigidBody::new(
            dummy_inertial_properties(),
            orientation,
            1.0,
            &velocity,
            &angular_velocity,
        );

        body.apply_force(&position, &Force::x(), &point![0.0, 1.0, 0.0]);

        body.advance_momentum(1.0);
        assert_abs_diff_ne!(body.compute_velocity(), velocity);

        body.advance_angular_momentum(1.0);
        assert_abs_diff_ne!(
            body.compute_angular_velocity(&orientation, 1.0),
            angular_velocity
        );
    }
}
