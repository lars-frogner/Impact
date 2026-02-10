//! Setup of driven motion for new entities.

use crate::{lock_order::OrderedRwLock, physics::PhysicsSimulator};
use anyhow::Result;
use impact_ecs::{
    setup,
    world::{EntityEntry, PrototypeEntities},
};
use impact_id::EntityID;
use impact_physics::{
    driven_motion::{
        circular::{CircularTrajectoryDriverID, HasCircularTrajectoryDriver},
        constant_acceleration::{
            ConstantAccelerationTrajectoryDriverID, HasConstantAccelerationTrajectoryDriver,
        },
        constant_rotation::{ConstantRotationDriverID, HasConstantRotationDriver},
        harmonic_oscillation::{
            HarmonicOscillatorTrajectoryDriverID, HasHarmonicOscillatorTrajectoryDriver,
        },
        orbit::{HasOrbitalTrajectoryDriver, OrbitalTrajectoryDriverID},
        setup::{
            self, CircularTrajectory, ConstantAccelerationTrajectory, ConstantRotation,
            HarmonicOscillatorTrajectory, OrbitalTrajectory,
        },
    },
    rigid_body::HasKinematicRigidBody,
};
use parking_lot::RwLock;

pub fn setup_driven_motion_for_new_entities(
    simulator: &RwLock<PhysicsSimulator>,
    entities: &mut PrototypeEntities,
) -> Result<()> {
    setup!(
        {
            let simulator = simulator.oread();
            let mut motion_driver_manager = simulator.motion_driver_manager().owrite();
        },
        entities,
        |entity_id: EntityID,
         trajectory: &CircularTrajectory|
         -> Result<HasCircularTrajectoryDriver> {
            setup::setup_circular_trajectory(&mut motion_driver_manager, entity_id, *trajectory)?;
            Ok(HasCircularTrajectoryDriver)
        },
        [HasKinematicRigidBody]
    )?;

    setup!(
        {
            let simulator = simulator.oread();
            let mut motion_driver_manager = simulator.motion_driver_manager().owrite();
        },
        entities,
        |entity_id: EntityID,
         trajectory: &ConstantAccelerationTrajectory|
         -> Result<HasConstantAccelerationTrajectoryDriver> {
            setup::setup_constant_acceleration_trajectory(
                &mut motion_driver_manager,
                entity_id,
                *trajectory,
            )?;
            Ok(HasConstantAccelerationTrajectoryDriver)
        },
        [HasKinematicRigidBody]
    )?;

    setup!(
        {
            let simulator = simulator.oread();
            let mut motion_driver_manager = simulator.motion_driver_manager().owrite();
        },
        entities,
        |entity_id: EntityID, rotation: &ConstantRotation| -> Result<HasConstantRotationDriver> {
            setup::setup_constant_rotation(&mut motion_driver_manager, entity_id, *rotation)?;
            Ok(HasConstantRotationDriver)
        },
        [HasKinematicRigidBody]
    )?;

    setup!(
        {
            let simulator = simulator.oread();
            let mut motion_driver_manager = simulator.motion_driver_manager().owrite();
        },
        entities,
        |entity_id: EntityID,
         trajectory: &HarmonicOscillatorTrajectory|
         -> Result<HasHarmonicOscillatorTrajectoryDriver> {
            setup::setup_harmonic_oscillator_trajectory(
                &mut motion_driver_manager,
                entity_id,
                *trajectory,
            )?;
            Ok(HasHarmonicOscillatorTrajectoryDriver)
        },
        [HasKinematicRigidBody]
    )?;

    setup!(
        {
            let simulator = simulator.oread();
            let mut motion_driver_manager = simulator.motion_driver_manager().owrite();
        },
        entities,
        |entity_id: EntityID,
         trajectory: &OrbitalTrajectory|
         -> Result<HasOrbitalTrajectoryDriver> {
            setup::setup_orbital_trajectory(&mut motion_driver_manager, entity_id, *trajectory)?;
            Ok(HasOrbitalTrajectoryDriver)
        },
        [HasKinematicRigidBody]
    )
}

pub fn remove_motion_drivers_for_entity(
    simulator: &RwLock<PhysicsSimulator>,
    entity_id: EntityID,
    entity: &EntityEntry<'_>,
) {
    if entity.has_component::<HasCircularTrajectoryDriver>() {
        let simulator = simulator.oread();
        let mut motion_driver_manager = simulator.motion_driver_manager().owrite();
        let driver_id = CircularTrajectoryDriverID::from_entity_id(entity_id);
        motion_driver_manager
            .circular_trajectories_mut()
            .remove_driver(driver_id);
    }
    if entity.has_component::<HasConstantAccelerationTrajectoryDriver>() {
        let simulator = simulator.oread();
        let mut motion_driver_manager = simulator.motion_driver_manager().owrite();
        let driver_id = ConstantAccelerationTrajectoryDriverID::from_entity_id(entity_id);
        motion_driver_manager
            .constant_acceleration_trajectories_mut()
            .remove_driver(driver_id);
    }
    if entity.has_component::<HasConstantRotationDriver>() {
        let simulator = simulator.oread();
        let mut motion_driver_manager = simulator.motion_driver_manager().owrite();
        let driver_id = ConstantRotationDriverID::from_entity_id(entity_id);
        motion_driver_manager
            .constant_rotations_mut()
            .remove_driver(driver_id);
    }
    if entity.has_component::<HasHarmonicOscillatorTrajectoryDriver>() {
        let simulator = simulator.oread();
        let mut motion_driver_manager = simulator.motion_driver_manager().owrite();
        let driver_id = HarmonicOscillatorTrajectoryDriverID::from_entity_id(entity_id);
        motion_driver_manager
            .harmonic_oscillator_trajectories_mut()
            .remove_driver(driver_id);
    }
    if entity.has_component::<HasOrbitalTrajectoryDriver>() {
        let simulator = simulator.oread();
        let mut motion_driver_manager = simulator.motion_driver_manager().owrite();
        let driver_id = OrbitalTrajectoryDriverID::from_entity_id(entity_id);
        motion_driver_manager
            .orbital_trajectories_mut()
            .remove_driver(driver_id);
    }
}
