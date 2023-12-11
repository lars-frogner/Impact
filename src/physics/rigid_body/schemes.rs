//! Schemes for evolving the motion of rigid bodies over time.

use super::RigidBodyAdvancedState;
use crate::{
    num::Float,
    physics::{self, fph, AngularVelocity, Force, RigidBody, Torque, Velocity},
};
use std::fmt;

/// Denotes a specific iterative scheme for evolving the motion of rigid bodies
/// over time.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SteppingScheme {
    EulerCromer,
    RK4,
}

/// Describes a substep of an iterative stepping scheme for rigid body motion.
pub trait SchemeSubstep {
    type SubstepIter: Iterator<Item = Self> + DoubleEndedIterator;

    /// Returns an iterator over all the substeps in the scheme given a full
    /// step duration.
    fn all_substeps(step_duration: fph) -> Self::SubstepIter;

    /// Whether this is the first substep in the scheme.
    fn is_first(&self) -> bool;

    /// Whether this is the last substep in the scheme.
    fn is_last(&self) -> bool;

    /// Returns the index of the substep (first step has index zero).
    fn idx(&self) -> usize;

    /// Returns the full step duration.
    fn full_step_duration(&self) -> fph;

    /// Returns the duration of this particular substep, which may be smaller
    /// than the full step duration.
    fn substep_duration(&self) -> fph;

    /// Returns the weight that should be used for the derivatives computed by
    /// this substep when the derivatives from all the substeps are averaged in
    /// the final step.
    fn derivative_weight(&self) -> fph;

    /// Computes the new simulation time after performing this substep, given
    /// the current simulation time.
    fn new_simulation_time(&self, current_simulation_time: fph) -> fph {
        current_simulation_time + self.substep_duration()
    }

    /// Given a velocity, angular velocity, force and torque, advances the
    /// current position, orientation, momentum, angular momentum, velocity and
    /// angular velocity of the given rigid body and returns the advanced
    /// quantities.
    fn advance_motion(
        &self,
        rigid_body: &RigidBody,
        scaling: fph,
        velocity: &Velocity,
        angular_velocity: &AngularVelocity,
        force: &Force,
        torque: &Torque,
    ) -> RigidBodyAdvancedState {
        let substep_duration = self.substep_duration();

        let advanced_position =
            RigidBody::advance_position(rigid_body.position(), velocity, substep_duration);

        let advanced_orientation = physics::advance_orientation(
            rigid_body.orientation(),
            angular_velocity,
            substep_duration,
        );

        let advanced_momentum =
            RigidBody::advance_momentum(rigid_body.momentum(), force, substep_duration);

        let advanced_velocity =
            RigidBody::compute_velocity(rigid_body.inertial_properties(), &advanced_momentum);

        let advanced_angular_momentum = RigidBody::advance_angular_momentum(
            rigid_body.angular_momentum(),
            torque,
            substep_duration,
        );

        let advanced_angular_velocity = RigidBody::compute_angular_velocity(
            rigid_body.inertial_properties(),
            &advanced_orientation,
            scaling,
            &advanced_angular_momentum,
        );

        RigidBodyAdvancedState {
            position: advanced_position,
            orientation: advanced_orientation,
            momentum: advanced_momentum,
            angular_momentum: advanced_angular_momentum,
            velocity: advanced_velocity,
            angular_velocity: advanced_angular_velocity,
        }
    }
}

/// A step in the Euler-Cromer scheme.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct EulerCromerStep {
    step_duration: fph,
}

/// A substep in the 4th order Runge-Kutta scheme.
#[derive(Clone, Debug, PartialEq)]
pub struct RungeKutta4Substep {
    idx: RK4SubstepIdx,
    step_duration: fph,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum RK4SubstepIdx {
    Zero = 0,
    One = 1,
    Two = 2,
    Three = 3,
}

impl fmt::Display for SteppingScheme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::EulerCromer => "Euler-Cromer",
                Self::RK4 => "RK4",
            }
        )
    }
}

impl EulerCromerStep {
    /// Creates a new Euler-Cromer step with the given duration.
    pub fn new(step_duration: fph) -> Self {
        Self { step_duration }
    }
}

impl SchemeSubstep for EulerCromerStep {
    type SubstepIter = std::array::IntoIter<Self, 1>;

    fn all_substeps(step_duration: fph) -> Self::SubstepIter {
        [Self::new(step_duration)].into_iter()
    }

    fn is_first(&self) -> bool {
        true
    }

    fn is_last(&self) -> bool {
        true
    }

    fn idx(&self) -> usize {
        0
    }

    fn full_step_duration(&self) -> fph {
        self.step_duration
    }

    fn substep_duration(&self) -> fph {
        self.step_duration
    }

    fn derivative_weight(&self) -> fph {
        1.0
    }

    fn advance_motion(
        &self,
        rigid_body: &RigidBody,
        scaling: fph,
        _velocity: &Velocity,
        _angular_velocity: &AngularVelocity,
        force: &Force,
        torque: &Torque,
    ) -> RigidBodyAdvancedState {
        let advanced_momentum =
            RigidBody::advance_momentum(rigid_body.momentum(), force, self.step_duration);

        let advanced_velocity =
            RigidBody::compute_velocity(rigid_body.inertial_properties(), &advanced_momentum);

        let advanced_angular_momentum = RigidBody::advance_angular_momentum(
            rigid_body.angular_momentum(),
            torque,
            self.step_duration,
        );

        let advanced_angular_velocity = RigidBody::compute_angular_velocity(
            rigid_body.inertial_properties(),
            rigid_body.orientation(),
            scaling,
            &advanced_angular_momentum,
        );

        let advanced_position = RigidBody::advance_position(
            rigid_body.position(),
            &advanced_velocity,
            self.step_duration,
        );

        let advanced_orientation = physics::advance_orientation(
            rigid_body.orientation(),
            &advanced_angular_velocity,
            self.step_duration,
        );

        RigidBodyAdvancedState {
            position: advanced_position,
            orientation: advanced_orientation,
            momentum: advanced_momentum,
            angular_momentum: advanced_angular_momentum,
            velocity: advanced_velocity,
            angular_velocity: advanced_angular_velocity,
        }
    }
}

impl RungeKutta4Substep {
    const N_SUBSTEPS: usize = 4;

    const STEP_DURATION_WEIGHTS: [fph; Self::N_SUBSTEPS] = [0.5, 0.5, 1.0, 1.0];

    const INTERMEDIATE_STATE_WEIGHTS: [fph; Self::N_SUBSTEPS] = [
        fph::ONE_SIXTH,
        2.0 * fph::ONE_SIXTH,
        2.0 * fph::ONE_SIXTH,
        fph::ONE_SIXTH,
    ];

    fn new(idx: RK4SubstepIdx, step_duration: fph) -> Self {
        Self { idx, step_duration }
    }
}

impl SchemeSubstep for RungeKutta4Substep {
    type SubstepIter = std::array::IntoIter<Self, { Self::N_SUBSTEPS }>;

    fn all_substeps(step_duration: fph) -> Self::SubstepIter {
        [
            Self::new(RK4SubstepIdx::Zero, step_duration),
            Self::new(RK4SubstepIdx::One, step_duration),
            Self::new(RK4SubstepIdx::Two, step_duration),
            Self::new(RK4SubstepIdx::Three, step_duration),
        ]
        .into_iter()
    }

    fn is_first(&self) -> bool {
        self.idx == RK4SubstepIdx::Zero
    }

    fn is_last(&self) -> bool {
        self.idx == RK4SubstepIdx::Three
    }

    fn idx(&self) -> usize {
        self.idx as usize
    }

    fn full_step_duration(&self) -> fph {
        self.step_duration
    }

    fn substep_duration(&self) -> fph {
        self.step_duration * Self::STEP_DURATION_WEIGHTS[self.idx()]
    }

    fn derivative_weight(&self) -> fph {
        Self::INTERMEDIATE_STATE_WEIGHTS[self.idx()]
    }
}
