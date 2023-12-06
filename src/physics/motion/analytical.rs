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
    control::{MotionControlComp, OrientationControlComp},
    physics::{
        fph, Position, RigidBodyComp, SpatialConfigurationComp, Static, Velocity, VelocityComp,
    },
};
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
            |spatial: &mut SpatialConfigurationComp, velocity: &mut VelocityComp| {
                spatial.position = Position::origin();
                velocity.0 = Velocity::zeros();
            },
            [ConstantAccelerationTrajectoryComp],
            ![Static, MotionControlComp, RigidBodyComp]
        );
        query!(
            ecs_world,
            |spatial: &mut SpatialConfigurationComp, velocity: &mut VelocityComp| {
                spatial.position = Position::origin();
                velocity.0 = Velocity::zeros();
            },
            [HarmonicOscillatorTrajectoryComp],
            ![Static, MotionControlComp, RigidBodyComp]
        );
        query!(
            ecs_world,
            |spatial: &mut SpatialConfigurationComp, velocity: &mut VelocityComp| {
                spatial.position = Position::origin();
                velocity.0 = Velocity::zeros();
            },
            [CircularTrajectoryComp],
            ![Static, MotionControlComp, RigidBodyComp]
        );
        query!(
            ecs_world,
            |spatial: &mut SpatialConfigurationComp, velocity: &mut VelocityComp| {
                spatial.position = Position::origin();
                velocity.0 = Velocity::zeros();
            },
            [OrbitalTrajectoryComp],
            ![Static, MotionControlComp, RigidBodyComp]
        );
    }

    fn apply_constant_acceleration_trajectories(ecs_world: &ECSWorld, simulation_time: fph) {
        query!(
            ecs_world,
            |spatial: &mut SpatialConfigurationComp,
             velocity: &mut VelocityComp,
             trajectory: &ConstantAccelerationTrajectoryComp| {
                let (trajectory_position, trajectory_velocity) =
                    trajectory.compute_position_and_velocity(simulation_time);
                spatial.position += trajectory_position.coords;
                velocity.0 += trajectory_velocity;
            },
            ![Static, MotionControlComp, RigidBodyComp]
        );
    }

    fn apply_harmonically_oscillating_trajectories(ecs_world: &ECSWorld, simulation_time: fph) {
        query!(
            ecs_world,
            |spatial: &mut SpatialConfigurationComp,
             velocity: &mut VelocityComp,
             trajectory: &HarmonicOscillatorTrajectoryComp| {
                let (trajectory_position, trajectory_velocity) =
                    trajectory.compute_position_and_velocity(simulation_time);
                spatial.position += trajectory_position.coords;
                velocity.0 += trajectory_velocity;
            },
            ![Static, MotionControlComp, RigidBodyComp]
        );
    }

    fn apply_circular_trajectories(ecs_world: &ECSWorld, simulation_time: fph) {
        query!(
            ecs_world,
            |spatial: &mut SpatialConfigurationComp,
             velocity: &mut VelocityComp,
             trajectory: &CircularTrajectoryComp| {
                let (trajectory_position, trajectory_velocity) =
                    trajectory.compute_position_and_velocity(simulation_time);
                spatial.position += trajectory_position.coords;
                velocity.0 += trajectory_velocity;
            },
            ![Static, MotionControlComp, RigidBodyComp]
        );
    }

    fn apply_orbital_trajectories(ecs_world: &ECSWorld, simulation_time: fph) {
        query!(
            ecs_world,
            |spatial: &mut SpatialConfigurationComp,
             velocity: &mut VelocityComp,
             trajectory: &OrbitalTrajectoryComp| {
                let (trajectory_position, trajectory_velocity) =
                    trajectory.compute_position_and_velocity(simulation_time);
                spatial.position += trajectory_position.coords;
                velocity.0 += trajectory_velocity;
            },
            ![Static, MotionControlComp, RigidBodyComp]
        );
    }

    fn apply_constant_rotations(ecs_world: &ECSWorld, simulation_time: fph) {
        query!(
            ecs_world,
            |spatial: &mut SpatialConfigurationComp, rotation: &ConstantRotationComp| {
                spatial.orientation = rotation.compute_orientation(simulation_time);
            },
            ![Static, OrientationControlComp, RigidBodyComp]
        );
    }
}
