//! Simulation of physics.

mod motion;
mod tasks;
mod time;

pub use motion::{AdvancePositions, Position, PositionComp, Velocity, VelocityComp};
pub use tasks::PhysicsTag;

/// Floating point type used for physics simulation.
#[allow(non_camel_case_types)]
pub type fph = f64;

#[derive(Debug)]
pub struct PhysicsSimulator {
    config: SimulatorConfig,
}

#[derive(Clone, Debug)]
pub struct SimulatorConfig {
    time_step_duration: fph,
}

impl PhysicsSimulator {
    pub fn new(config: SimulatorConfig) -> Self {
        Self { config }
    }

    pub fn time_step_duration(&self) -> fph {
        self.config.time_step_duration
    }
}

impl Default for SimulatorConfig {
    fn default() -> Self {
        Self {
            time_step_duration: 1.0,
        }
    }
}
