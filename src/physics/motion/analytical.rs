//! Motion defined by analytical functions.

mod circular;
mod constant_acceleration;
mod constant_rotation;
mod harmonic_oscillation;
mod orbit;

pub use circular::CircularTrajectoryComp;
pub use constant_acceleration::ConstantAccelerationTrajectoryComp;
pub use constant_rotation::ConstantRotationComp;
pub use harmonic_oscillation::HarmonicOscillatorTrajectoryComp;
pub use orbit::OrbitalTrajectoryComp;

use crate::{
    component::ComponentRegistry,
    control::components::{MotionControlComp, OrientationControlComp},
    physics::{fph, Position, ReferenceFrameComp, RigidBodyComp, Static, Velocity, VelocityComp},
};
use anyhow::Result;
use impact_ecs::{query, world::World as ECSWorld};

/// Manager of all systems controlling entity motion analytically.
#[derive(Debug)]
pub struct AnalyticalMotionManager;

impl AnalyticalMotionManager {
    /// Creates a new analytical motion manager.
    pub fn new() -> Self {
        Self
    }

    /// Sets the positions, velocities, orientations and angular velocities of
    /// all entities whose motions are controlled analytically to the values for
    /// the given simulation time.
    pub fn apply_analytical_motion(&self, ecs_world: &ECSWorld, simulation_time: fph) {
        Self::reset_positions_and_velocities(ecs_world);
        Self::apply_constant_acceleration_trajectories(ecs_world, simulation_time);
        Self::apply_harmonically_oscillating_trajectories(ecs_world, simulation_time);
        Self::apply_circular_trajectories(ecs_world, simulation_time);
        Self::apply_orbital_trajectories(ecs_world, simulation_time);
        Self::apply_constant_rotations(ecs_world, simulation_time);
    }

    fn reset_positions_and_velocities(ecs_world: &ECSWorld) {
        query!(
            ecs_world,
            |frame: &mut ReferenceFrameComp, velocity: &mut VelocityComp| {
                frame.position = Position::origin();
                velocity.linear = Velocity::zeros();
            },
            [ConstantAccelerationTrajectoryComp],
            ![Static, MotionControlComp, RigidBodyComp]
        );
        query!(
            ecs_world,
            |frame: &mut ReferenceFrameComp, velocity: &mut VelocityComp| {
                frame.position = Position::origin();
                velocity.linear = Velocity::zeros();
            },
            [HarmonicOscillatorTrajectoryComp],
            ![Static, MotionControlComp, RigidBodyComp]
        );
        query!(
            ecs_world,
            |frame: &mut ReferenceFrameComp, velocity: &mut VelocityComp| {
                frame.position = Position::origin();
                velocity.linear = Velocity::zeros();
            },
            [CircularTrajectoryComp],
            ![Static, MotionControlComp, RigidBodyComp]
        );
        query!(
            ecs_world,
            |frame: &mut ReferenceFrameComp, velocity: &mut VelocityComp| {
                frame.position = Position::origin();
                velocity.linear = Velocity::zeros();
            },
            [OrbitalTrajectoryComp],
            ![Static, MotionControlComp, RigidBodyComp]
        );
    }

    fn apply_constant_acceleration_trajectories(ecs_world: &ECSWorld, simulation_time: fph) {
        query!(
            ecs_world,
            |frame: &mut ReferenceFrameComp,
             velocity: &mut VelocityComp,
             trajectory: &ConstantAccelerationTrajectoryComp| {
                let (trajectory_position, trajectory_velocity) =
                    trajectory.compute_position_and_velocity(simulation_time);
                frame.position += trajectory_position.coords;
                velocity.linear += trajectory_velocity;
            },
            ![Static, MotionControlComp, RigidBodyComp]
        );
    }

    fn apply_harmonically_oscillating_trajectories(ecs_world: &ECSWorld, simulation_time: fph) {
        query!(
            ecs_world,
            |frame: &mut ReferenceFrameComp,
             velocity: &mut VelocityComp,
             trajectory: &HarmonicOscillatorTrajectoryComp| {
                let (trajectory_position, trajectory_velocity) =
                    trajectory.compute_position_and_velocity(simulation_time);
                frame.position += trajectory_position.coords;
                velocity.linear += trajectory_velocity;
            },
            ![Static, MotionControlComp, RigidBodyComp]
        );
    }

    fn apply_circular_trajectories(ecs_world: &ECSWorld, simulation_time: fph) {
        query!(
            ecs_world,
            |frame: &mut ReferenceFrameComp,
             velocity: &mut VelocityComp,
             trajectory: &CircularTrajectoryComp| {
                let (trajectory_position, trajectory_velocity) =
                    trajectory.compute_position_and_velocity(simulation_time);
                frame.position += trajectory_position.coords;
                velocity.linear += trajectory_velocity;
            },
            ![Static, MotionControlComp, RigidBodyComp]
        );
    }

    fn apply_orbital_trajectories(ecs_world: &ECSWorld, simulation_time: fph) {
        query!(
            ecs_world,
            |frame: &mut ReferenceFrameComp,
             velocity: &mut VelocityComp,
             trajectory: &OrbitalTrajectoryComp| {
                let (trajectory_position, trajectory_velocity) =
                    trajectory.compute_position_and_velocity(simulation_time);
                frame.position += trajectory_position.coords;
                velocity.linear += trajectory_velocity;
            },
            ![Static, MotionControlComp, RigidBodyComp]
        );
    }

    fn apply_constant_rotations(ecs_world: &ECSWorld, simulation_time: fph) {
        query!(
            ecs_world,
            |frame: &mut ReferenceFrameComp, rotation: &ConstantRotationComp| {
                frame.orientation = rotation.compute_orientation(simulation_time);
            },
            ![Static, OrientationControlComp, RigidBodyComp]
        );
    }
}

impl Default for AnalyticalMotionManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Registers all analytical motion
/// [`Component`](impact_ecs::component::Component)s.
pub fn register_analytical_motion_components(registry: &mut ComponentRegistry) -> Result<()> {
    register_component!(registry, ConstantAccelerationTrajectoryComp)?;
    register_component!(registry, HarmonicOscillatorTrajectoryComp)?;
    register_component!(registry, CircularTrajectoryComp)?;
    register_component!(registry, OrbitalTrajectoryComp)?;
    register_component!(registry, ConstantRotationComp)
}
