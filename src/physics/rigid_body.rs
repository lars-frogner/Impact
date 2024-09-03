//! Rigid body simulation.

pub mod components;
pub mod entity;
pub mod forces;
pub mod schemes;
pub mod systems;

use crate::physics::{
    fph,
    inertia::InertialProperties,
    motion::{
        AngularMomentum, AngularVelocity, Force, Momentum, Orientation, Position, Torque, Velocity,
    },
};
use approx::AbsDiffEq;
use bytemuck::{Pod, Zeroable};
use nalgebra::Vector3;
use schemes::SchemeSubstep;

/// The maximum number of intermediate states from the substeps of a stepping
/// scheme that can be stored in a rigid body.
const MAX_INTERMEDIATE_STATES: usize = 4;

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
    intermediate_states: RigidBodyIntermediateStates,
}

#[derive(Clone, Debug)]
pub struct RigidBodyAdvancedState {
    position: Position,
    orientation: Orientation,
    momentum: Momentum,
    angular_momentum: AngularMomentum,
    velocity: Velocity,
    angular_velocity: AngularVelocity,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq, Zeroable, Pod)]
struct RigidBodyDynamicState {
    velocity: Velocity,
    angular_velocity: Vector3<fph>,
    force: Force,
    torque: Torque,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
struct RigidBodyIntermediateStates {
    states: [RigidBodyDynamicState; MAX_INTERMEDIATE_STATES],
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
        let momentum = Self::compute_momentum(&inertial_properties, velocity);
        let angular_momentum = Self::compute_angular_momentum(
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
            intermediate_states: RigidBodyIntermediateStates::new(),
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
    ///
    /// # Warning
    /// This position will be out of sync with the position in with the entity's
    /// [`ReferenceFrameComp`](crate::physics::ReferenceFrameComp)
    /// between the first and last substep in the stepping scheme, as it is only
    /// updated with the final orientation after the last substep.
    pub fn position(&self) -> &Position {
        &self.position
    }

    /// Returns the orientation of the body.
    ///
    /// # Warning
    /// This orientation will be out of sync with the orientation in with the
    /// entity's
    /// [`ReferenceFrameComp`](crate::physics::ReferenceFrameComp)
    /// between the first and last substep in the stepping scheme, as it is only
    /// updated with the final orientation after the last substep.
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

    /// Computes the total kinetic energy (translational and rotational) of the
    /// body.
    pub fn compute_kinetic_energy(&self, scaling: fph) -> fph {
        self.compute_translational_kinetic_energy()
            + self.compute_rotational_kinetic_energy(scaling)
    }

    /// Computes the translational kinetic energy of the body.
    pub fn compute_translational_kinetic_energy(&self) -> fph {
        let velocity = Self::compute_velocity(&self.inertial_properties, &self.momentum);

        0.5 * self.mass() * velocity.norm_squared()
    }

    /// Computes the rotational kinetic energy of the body.
    pub fn compute_rotational_kinetic_energy(&self, scaling: fph) -> fph {
        let angular_velocity = Self::compute_angular_velocity(
            &self.inertial_properties,
            &self.orientation,
            scaling,
            &self.angular_momentum,
        );

        0.5 * angular_velocity.as_vector().dot(&self.angular_momentum)
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

    /// Recomputes the body's momentum according to the given velocity.
    pub fn synchronize_momentum(&mut self, velocity: &Velocity) {
        self.momentum = Self::compute_momentum(&self.inertial_properties, velocity);
    }

    /// Recomputes the body's angular momentum according to the given
    /// orientation, scaling and angular velocity.
    pub fn synchronize_angular_momentum(
        &mut self,
        orientation: &Orientation,
        scaling: fph,
        angular_velocity: &AngularVelocity,
    ) {
        self.angular_momentum = Self::compute_angular_momentum(
            &self.inertial_properties,
            orientation,
            scaling,
            angular_velocity,
        );
    }

    /// Executes the given substep of a stepping scheme. Updates the given
    /// position, orientation, velocity and angular velocity with the results of
    /// the substep. If the substep is the final one in a full step, the stored
    /// state of the rigid body is updated to the final result.
    ///
    /// If the given orientation, scaling, velocity or angular velocity have
    /// been modified after the previous call to this function, make sure to
    /// call [`synchronize_momentum`](Self::synchronize_momentum) and/or
    /// [`synchronize_angular_momentum`](Self::synchronize_angular_momentum)
    /// accordingly before calling this function.
    ///
    /// This function resets the total force and torque.
    pub fn advance_motion<S: SchemeSubstep>(
        &mut self,
        substep: &S,
        position: &mut Position,
        orientation: &mut Orientation,
        scaling: fph,
        velocity: &mut Velocity,
        angular_velocity: &mut AngularVelocity,
    ) {
        // In the first substep, we update the current position and orientation
        // in case they have been changed after the previous full step. To avoid
        // unnecessary calculations, we assume that momentum and angular
        // momentum have already been updated through the appropriate methods.
        // In later substeps, the supplied state is updated with the we values
        // resulting from the substep, but we do not update the state stored in
        // the rigid body until the final substep is complete.
        if substep.is_first() {
            self.position = *position;
            self.orientation = *orientation;
        }

        if substep.is_last() {
            self.perform_final_motion_advancement_substep(
                substep,
                position,
                orientation,
                scaling,
                velocity,
                angular_velocity,
            );
        } else {
            // Multi-step schemes will need access to the dynamic state
            // (derivatives) produced by previous substeps, so we record the
            // current state here before it is changed
            self.intermediate_states.store_state(
                substep,
                RigidBodyDynamicState {
                    velocity: *velocity,
                    angular_velocity: angular_velocity.as_vector(),
                    force: self.total_force,
                    torque: self.total_torque,
                },
            );

            self.perform_intermediate_motion_advancement_substep(
                substep,
                position,
                orientation,
                scaling,
                velocity,
                angular_velocity,
            );
        }

        self.reset_total_force();
        self.reset_total_torque();
    }

    fn perform_intermediate_motion_advancement_substep<S: SchemeSubstep>(
        &self,
        substep: &S,
        position: &mut Position,
        orientation: &mut Orientation,
        scaling: fph,
        velocity: &mut Velocity,
        angular_velocity: &mut AngularVelocity,
    ) {
        let RigidBodyAdvancedState {
            position: advanced_position,
            orientation: advanced_orientation,
            momentum: _,
            angular_momentum: _,
            velocity: advanced_velocity,
            angular_velocity: advanced_angular_velocity,
        } = substep.advance_motion(
            self,
            scaling,
            velocity,
            angular_velocity,
            &self.total_force,
            &self.total_torque,
        );

        *position = advanced_position;
        *orientation = advanced_orientation;
        *velocity = advanced_velocity;
        *angular_velocity = advanced_angular_velocity;
    }

    fn perform_final_motion_advancement_substep<S: SchemeSubstep>(
        &mut self,
        last_substep: &S,
        position: &mut Position,
        orientation: &mut Orientation,
        scaling: fph,
        velocity: &mut Velocity,
        angular_velocity: &mut AngularVelocity,
    ) {
        // In the final substep, the derivatives calculated by the previous
        // substeps are averaged (with weights) to obtain the derivatives for
        // performing the final step
        let last_weight = last_substep.derivative_weight();
        let mut average_velocity = *velocity * last_weight;
        let mut average_angular_velocity = angular_velocity.as_vector() * last_weight;
        let mut average_force = self.total_force * last_weight;
        let mut average_torque = self.total_torque * last_weight;

        for substep in S::all_substeps(last_substep.full_step_duration())
            .rev()
            .skip(1)
        {
            let weight = substep.derivative_weight();
            let state = self.intermediate_states.state(&substep);
            average_velocity += state.velocity * weight;
            average_angular_velocity += state.angular_velocity * weight;
            average_force += state.force * weight;
            average_torque += state.torque * weight;
        }

        let average_angular_velocity = AngularVelocity::from_vector(average_angular_velocity);

        let RigidBodyAdvancedState {
            position: advanced_position,
            orientation: advanced_orientation,
            momentum: advanced_momentum,
            angular_momentum: advanced_angular_momentum,
            velocity: advanced_velocity,
            angular_velocity: advanced_angular_velocity,
        } = last_substep.advance_motion(
            self,
            scaling,
            &average_velocity,
            &average_angular_velocity,
            &average_force,
            &average_torque,
        );

        *position = advanced_position;
        *orientation = advanced_orientation;
        *velocity = advanced_velocity;
        *angular_velocity = advanced_angular_velocity;

        // We record the final state as the new state of the rigid body
        self.position = *position;
        self.orientation = *orientation;
        self.momentum = advanced_momentum;
        self.angular_momentum = advanced_angular_momentum;
    }

    fn reset_total_force(&mut self) {
        self.total_force = Force::zeros();
    }

    fn reset_total_torque(&mut self) {
        self.total_torque = Torque::zeros();
    }

    fn advance_position(position: &Position, velocity: &Velocity, duration: fph) -> Position {
        *position + velocity * duration
    }

    fn advance_momentum(momentum: &Momentum, force: &Force, duration: fph) -> Momentum {
        *momentum + force * duration
    }

    fn advance_angular_momentum(
        angular_momentum: &AngularMomentum,
        torque: &Torque,
        duration: fph,
    ) -> AngularMomentum {
        *angular_momentum + torque * duration
    }

    fn compute_momentum(inertial_properties: &InertialProperties, velocity: &Velocity) -> Momentum {
        inertial_properties.mass() * velocity
    }

    fn compute_velocity(inertial_properties: &InertialProperties, momentum: &Momentum) -> Velocity {
        momentum / inertial_properties.mass()
    }

    fn compute_angular_velocity(
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

    fn compute_angular_momentum(
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

impl RigidBodyIntermediateStates {
    fn new() -> Self {
        Self {
            states: [RigidBodyDynamicState::default(); MAX_INTERMEDIATE_STATES],
        }
    }

    fn store_state<S: SchemeSubstep>(&mut self, substep: &S, state: RigidBodyDynamicState) {
        self.states[substep.idx()] = state;
    }

    fn state<S: SchemeSubstep>(&self, substep: &S) -> &RigidBodyDynamicState {
        &self.states[substep.idx()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{geometry::Degrees, num::Float};
    use approx::{abs_diff_eq, assert_abs_diff_eq, assert_abs_diff_ne};
    use nalgebra::{point, vector, Vector3};
    use proptest::prelude::*;
    use schemes::EulerCromerStep;

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
    fn should_reset_force_and_torque_after_advancing_motion() {
        let mut body = dummy_rigid_body();

        body.apply_force(&Force::x(), &point![0.0, 1.0, 0.0]);

        let mut position = Position::origin();
        let mut orientation = Orientation::identity();
        let mut velocity = Velocity::zeros();
        let mut angular_velocity = AngularVelocity::zero();

        body.advance_motion(
            &EulerCromerStep::new(1.0),
            &mut position,
            &mut orientation,
            1.0,
            &mut velocity,
            &mut angular_velocity,
        );

        assert_abs_diff_eq!(body.total_force(), &Force::zeros());
        assert_abs_diff_eq!(body.total_torque(), &Torque::zeros());
    }

    #[test]
    fn should_retain_motion_when_advancing_for_zero_time() {
        let original_position = Position::origin();
        let original_orientation = Orientation::identity();
        let original_velocity = Velocity::z();
        let original_angular_velocity = AngularVelocity::from_vector(Vector3::x());

        let mut body = RigidBody::new(
            dummy_inertial_properties(),
            original_position,
            original_orientation,
            1.0,
            &original_velocity,
            &original_angular_velocity,
        );

        body.apply_force(&Force::x(), &point![0.0, 1.0, 0.0]);

        let mut position = original_position;
        let mut orientation = original_orientation;
        let mut velocity = original_velocity;
        let mut angular_velocity = original_angular_velocity;

        body.advance_motion(
            &EulerCromerStep::new(0.0),
            &mut position,
            &mut orientation,
            1.0,
            &mut velocity,
            &mut angular_velocity,
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

        let mut body = RigidBody::new(
            dummy_inertial_properties(),
            original_position,
            original_orientation,
            1.0,
            &original_velocity,
            &original_angular_velocity,
        );

        let mut position = original_position;
        let mut orientation = original_orientation;
        let mut velocity = original_velocity;
        let mut angular_velocity = original_angular_velocity;

        body.advance_motion(
            &EulerCromerStep::new(1.0),
            &mut position,
            &mut orientation,
            1.0,
            &mut velocity,
            &mut angular_velocity,
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
        let original_angular_velocity = AngularVelocity::from_vector(Vector3::x());

        let mut body = RigidBody::new(
            dummy_inertial_properties(),
            original_position,
            original_orientation,
            1.0,
            &original_velocity,
            &original_angular_velocity,
        );

        body.apply_force(&Force::x(), &point![0.0, 1.0, 0.0]);

        let mut position = original_position;
        let mut orientation = original_orientation;
        let mut velocity = original_velocity;
        let mut angular_velocity = original_angular_velocity;

        body.advance_motion(
            &EulerCromerStep::new(1.0),
            &mut position,
            &mut orientation,
            1.0,
            &mut velocity,
            &mut angular_velocity,
        );

        assert_abs_diff_ne!(position, original_position);
        assert_abs_diff_ne!(orientation, original_orientation);
        assert_abs_diff_ne!(velocity, original_velocity);
        assert_abs_diff_ne!(angular_velocity, original_angular_velocity);
    }
}
