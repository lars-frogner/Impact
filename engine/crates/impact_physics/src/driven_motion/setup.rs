//! Setup of driven motion.

use impact_id::EntityID;

pub use crate::driven_motion::{
    circular::CircularTrajectory, constant_acceleration::ConstantAccelerationTrajectory,
    constant_rotation::ConstantRotation, harmonic_oscillation::HarmonicOscillatorTrajectory,
    orbit::OrbitalTrajectory,
};

use crate::driven_motion::{
    MotionDriverManager,
    circular::{CircularTrajectoryDriver, CircularTrajectoryDriverID},
    constant_acceleration::{
        ConstantAccelerationTrajectoryDriver, ConstantAccelerationTrajectoryDriverID,
    },
    constant_rotation::{ConstantRotationDriver, ConstantRotationDriverID},
    harmonic_oscillation::{
        HarmonicOscillatorTrajectoryDriver, HarmonicOscillatorTrajectoryDriverID,
    },
    orbit::{OrbitalTrajectoryDriver, OrbitalTrajectoryDriverID},
};

pub fn setup_circular_trajectory(
    motion_driver_manager: &mut MotionDriverManager,
    entity_id: EntityID,
    trajectory: CircularTrajectory,
) -> CircularTrajectoryDriverID {
    motion_driver_manager
        .circular_trajectories_mut()
        .insert_driver(CircularTrajectoryDriver {
            entity_id,
            trajectory,
        })
}

pub fn setup_constant_acceleration_trajectory(
    motion_driver_manager: &mut MotionDriverManager,
    entity_id: EntityID,
    trajectory: ConstantAccelerationTrajectory,
) -> ConstantAccelerationTrajectoryDriverID {
    motion_driver_manager
        .constant_acceleration_trajectories_mut()
        .insert_driver(ConstantAccelerationTrajectoryDriver {
            entity_id,
            trajectory,
        })
}

pub fn setup_constant_rotation(
    motion_driver_manager: &mut MotionDriverManager,
    entity_id: EntityID,
    rotation: ConstantRotation,
) -> ConstantRotationDriverID {
    motion_driver_manager
        .constant_rotations_mut()
        .insert_driver(ConstantRotationDriver::new(entity_id, rotation))
}

pub fn setup_harmonic_oscillator_trajectory(
    motion_driver_manager: &mut MotionDriverManager,
    entity_id: EntityID,
    trajectory: HarmonicOscillatorTrajectory,
) -> HarmonicOscillatorTrajectoryDriverID {
    motion_driver_manager
        .harmonic_oscillator_trajectories_mut()
        .insert_driver(HarmonicOscillatorTrajectoryDriver::new(
            entity_id, trajectory,
        ))
}

pub fn setup_orbital_trajectory(
    motion_driver_manager: &mut MotionDriverManager,
    entity_id: EntityID,
    trajectory: OrbitalTrajectory,
) -> OrbitalTrajectoryDriverID {
    motion_driver_manager
        .orbital_trajectories_mut()
        .insert_driver(OrbitalTrajectoryDriver::new(entity_id, trajectory))
}
