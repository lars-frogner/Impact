//! Setup of driven motion.

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
use anyhow::Result;
use impact_id::EntityID;

pub fn setup_circular_trajectory(
    motion_driver_manager: &mut MotionDriverManager,
    entity_id: EntityID,
    trajectory: CircularTrajectory,
) -> Result<()> {
    let driver_id = CircularTrajectoryDriverID::from_entity_id(entity_id);
    motion_driver_manager
        .circular_trajectories_mut()
        .insert_driver(
            driver_id,
            CircularTrajectoryDriver {
                entity_id,
                trajectory,
            },
        )
}

pub fn setup_constant_acceleration_trajectory(
    motion_driver_manager: &mut MotionDriverManager,
    entity_id: EntityID,
    trajectory: ConstantAccelerationTrajectory,
) -> Result<()> {
    let driver_id = ConstantAccelerationTrajectoryDriverID::from_entity_id(entity_id);
    motion_driver_manager
        .constant_acceleration_trajectories_mut()
        .insert_driver(
            driver_id,
            ConstantAccelerationTrajectoryDriver {
                entity_id,
                trajectory,
            },
        )
}

pub fn setup_constant_rotation(
    motion_driver_manager: &mut MotionDriverManager,
    entity_id: EntityID,
    rotation: ConstantRotation,
) -> Result<()> {
    let driver_id = ConstantRotationDriverID::from_entity_id(entity_id);
    motion_driver_manager
        .constant_rotations_mut()
        .insert_driver(driver_id, ConstantRotationDriver::new(entity_id, rotation))
}

pub fn setup_harmonic_oscillator_trajectory(
    motion_driver_manager: &mut MotionDriverManager,
    entity_id: EntityID,
    trajectory: HarmonicOscillatorTrajectory,
) -> Result<()> {
    let driver_id = HarmonicOscillatorTrajectoryDriverID::from_entity_id(entity_id);
    motion_driver_manager
        .harmonic_oscillator_trajectories_mut()
        .insert_driver(
            driver_id,
            HarmonicOscillatorTrajectoryDriver::new(entity_id, trajectory),
        )
}

pub fn setup_orbital_trajectory(
    motion_driver_manager: &mut MotionDriverManager,
    entity_id: EntityID,
    trajectory: OrbitalTrajectory,
) -> Result<()> {
    let driver_id = OrbitalTrajectoryDriverID::from_entity_id(entity_id);
    motion_driver_manager
        .orbital_trajectories_mut()
        .insert_driver(
            driver_id,
            OrbitalTrajectoryDriver::new(entity_id, trajectory),
        )
}
