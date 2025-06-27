//! ECS systems related to analytical motion.

use crate::{
    control::{
        motion::components::MotionControlComp, orientation::components::OrientationControlComp,
    },
    physics::{
        fph,
        motion::{
            Position, Velocity,
            analytical::{
                circular::components::CircularTrajectoryComp,
                constant_acceleration::components::ConstantAccelerationTrajectoryComp,
                constant_rotation::components::ConstantRotationComp,
                harmonic_oscillation::components::HarmonicOscillatorTrajectoryComp,
                orbit::components::OrbitalTrajectoryComp,
            },
            components::{ReferenceFrameComp, Static, VelocityComp},
        },
        rigid_body::components::RigidBodyComp,
    },
};
use impact_ecs::{query, world::World as ECSWorld};
use impact_scene::components::SceneEntityFlagsComp;

/// Sets the positions, velocities, orientations and angular velocities of
/// all entities whose motions are controlled analytically to the values for
/// the given simulation time.
pub fn apply_analytical_motion(ecs_world: &ECSWorld, simulation_time: fph) {
    reset_positions_and_velocities(ecs_world);
    apply_constant_acceleration_trajectories(ecs_world, simulation_time);
    apply_harmonically_oscillating_trajectories(ecs_world, simulation_time);
    apply_circular_trajectories(ecs_world, simulation_time);
    apply_orbital_trajectories(ecs_world, simulation_time);
    apply_constant_rotations(ecs_world, simulation_time);
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
        ![
            Static,
            MotionControlComp,
            RigidBodyComp,
            SceneEntityFlagsComp
        ]
    );
    query!(
        ecs_world,
        |frame: &mut ReferenceFrameComp,
         velocity: &mut VelocityComp,
         trajectory: &ConstantAccelerationTrajectoryComp,
         flags: &SceneEntityFlagsComp| {
            if flags.is_disabled() {
                return;
            }
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
        ![
            Static,
            MotionControlComp,
            RigidBodyComp,
            SceneEntityFlagsComp
        ]
    );
    query!(
        ecs_world,
        |frame: &mut ReferenceFrameComp,
         velocity: &mut VelocityComp,
         trajectory: &HarmonicOscillatorTrajectoryComp,
         flags: &SceneEntityFlagsComp| {
            if flags.is_disabled() {
                return;
            }
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
        ![
            Static,
            MotionControlComp,
            RigidBodyComp,
            SceneEntityFlagsComp
        ]
    );
    query!(
        ecs_world,
        |frame: &mut ReferenceFrameComp,
         velocity: &mut VelocityComp,
         trajectory: &CircularTrajectoryComp,
         flags: &SceneEntityFlagsComp| {
            if flags.is_disabled() {
                return;
            }
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
        ![
            Static,
            MotionControlComp,
            RigidBodyComp,
            SceneEntityFlagsComp
        ]
    );
    query!(
        ecs_world,
        |frame: &mut ReferenceFrameComp,
         velocity: &mut VelocityComp,
         trajectory: &OrbitalTrajectoryComp,
         flags: &SceneEntityFlagsComp| {
            if flags.is_disabled() {
                return;
            }
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
        ![
            Static,
            OrientationControlComp,
            RigidBodyComp,
            SceneEntityFlagsComp
        ]
    );
    query!(
        ecs_world,
        |frame: &mut ReferenceFrameComp,
         rotation: &ConstantRotationComp,
         flags: &SceneEntityFlagsComp| {
            if flags.is_disabled() {
                return;
            }
            frame.orientation = rotation.compute_orientation(simulation_time);
        },
        ![Static, OrientationControlComp, RigidBodyComp]
    );
}
