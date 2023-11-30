//! Simulation of physics.

mod events;
mod inertia;
mod motion;
mod rigid_body;
mod tasks;
mod time;

pub use inertia::{InertiaTensor, InertialProperties};
pub use motion::{
    advance_orientation, AdvanceOrientations, AdvancePositions, AngularVelocity,
    AngularVelocityComp, DrivenAngularVelocityComp, Force, Orientation, OrientationComp, Position,
    PositionComp, Static, Torque, Velocity, VelocityComp,
};
pub use rigid_body::{
    RigidBody, RigidBodyComp, RigidBodyID, RigidBodyManager, UniformRigidBodyComp,
};
pub use tasks::PhysicsTag;

use std::sync::RwLock;

/// Floating point type used for physics simulation.
#[allow(non_camel_case_types)]
pub type fph = f64;

#[derive(Debug)]
pub struct PhysicsSimulator {
    config: SimulatorConfig,
    rigid_body_manager: RwLock<RigidBodyManager>,
}

#[derive(Clone, Debug)]
pub struct SimulatorConfig {
    time_step_duration: fph,
}

impl PhysicsSimulator {
    pub fn new(config: SimulatorConfig) -> Self {
        Self {
            config,
            rigid_body_manager: RwLock::new(RigidBodyManager::new()),
        }
    }

    pub fn time_step_duration(&self) -> fph {
        self.config.time_step_duration
    }

    /// Returns a reference to the [`RigidBodyManager`], guarded by a
    /// [`RwLock`].
    pub fn rigid_body_manager(&self) -> &RwLock<RigidBodyManager> {
        &self.rigid_body_manager
    }
}

impl Default for SimulatorConfig {
    fn default() -> Self {
        Self {
            time_step_duration: 1.0,
        }
    }
}
