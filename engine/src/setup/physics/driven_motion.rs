//! Setup of driven motion for new entities.

use crate::{lock_order::OrderedRwLock, physics::PhysicsSimulator};
use impact_ecs::{
    setup,
    world::{EntityEntry, PrototypeEntities},
};
use impact_id::EntityID;
use impact_physics::{
    driven_motion::{
        circular::CircularTrajectoryDriverID,
        constant_acceleration::ConstantAccelerationTrajectoryDriverID,
        constant_rotation::ConstantRotationDriverID,
        harmonic_oscillation::HarmonicOscillatorTrajectoryDriverID,
        orbit::OrbitalTrajectoryDriverID,
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
) {
    setup!(
        {
            let simulator = simulator.oread();
            let mut motion_driver_manager = simulator.motion_driver_manager().owrite();
        },
        entities,
        |entity_id: EntityID, trajectory: &CircularTrajectory| -> CircularTrajectoryDriverID {
            setup::setup_circular_trajectory(&mut motion_driver_manager, entity_id, *trajectory)
        },
        [HasKinematicRigidBody]
    );

    setup!(
        {
            let simulator = simulator.oread();
            let mut motion_driver_manager = simulator.motion_driver_manager().owrite();
        },
        entities,
        |entity_id: EntityID,
         trajectory: &ConstantAccelerationTrajectory|
         -> ConstantAccelerationTrajectoryDriverID {
            setup::setup_constant_acceleration_trajectory(
                &mut motion_driver_manager,
                entity_id,
                *trajectory,
            )
        },
        [HasKinematicRigidBody]
    );

    setup!(
        {
            let simulator = simulator.oread();
            let mut motion_driver_manager = simulator.motion_driver_manager().owrite();
        },
        entities,
        |entity_id: EntityID, rotation: &ConstantRotation| -> ConstantRotationDriverID {
            setup::setup_constant_rotation(&mut motion_driver_manager, entity_id, *rotation)
        },
        [HasKinematicRigidBody]
    );

    setup!(
        {
            let simulator = simulator.oread();
            let mut motion_driver_manager = simulator.motion_driver_manager().owrite();
        },
        entities,
        |entity_id: EntityID,
         trajectory: &HarmonicOscillatorTrajectory|
         -> HarmonicOscillatorTrajectoryDriverID {
            setup::setup_harmonic_oscillator_trajectory(
                &mut motion_driver_manager,
                entity_id,
                *trajectory,
            )
        },
        [HasKinematicRigidBody]
    );

    setup!(
        {
            let simulator = simulator.oread();
            let mut motion_driver_manager = simulator.motion_driver_manager().owrite();
        },
        entities,
        |entity_id: EntityID, trajectory: &OrbitalTrajectory| -> OrbitalTrajectoryDriverID {
            setup::setup_orbital_trajectory(&mut motion_driver_manager, entity_id, *trajectory)
        },
        [HasKinematicRigidBody]
    );
}

pub fn remove_motion_drivers_for_entity(
    simulator: &RwLock<PhysicsSimulator>,
    entity: &EntityEntry<'_>,
) {
    if let Some(driver_id) = entity.get_component::<CircularTrajectoryDriverID>() {
        let simulator = simulator.oread();
        let mut motion_driver_manager = simulator.motion_driver_manager().owrite();
        motion_driver_manager
            .circular_trajectories_mut()
            .remove_driver(*driver_id.access());
    }
    if let Some(driver_id) = entity.get_component::<ConstantAccelerationTrajectoryDriverID>() {
        let simulator = simulator.oread();
        let mut motion_driver_manager = simulator.motion_driver_manager().owrite();
        motion_driver_manager
            .constant_acceleration_trajectories_mut()
            .remove_driver(*driver_id.access());
    }
    if let Some(driver_id) = entity.get_component::<ConstantRotationDriverID>() {
        let simulator = simulator.oread();
        let mut motion_driver_manager = simulator.motion_driver_manager().owrite();
        motion_driver_manager
            .constant_rotations_mut()
            .remove_driver(*driver_id.access());
    }
    if let Some(driver_id) = entity.get_component::<HarmonicOscillatorTrajectoryDriverID>() {
        let simulator = simulator.oread();
        let mut motion_driver_manager = simulator.motion_driver_manager().owrite();
        motion_driver_manager
            .harmonic_oscillator_trajectories_mut()
            .remove_driver(*driver_id.access());
    }
    if let Some(driver_id) = entity.get_component::<OrbitalTrajectoryDriverID>() {
        let simulator = simulator.oread();
        let mut motion_driver_manager = simulator.motion_driver_manager().owrite();
        motion_driver_manager
            .orbital_trajectories_mut()
            .remove_driver(*driver_id.access());
    }
}
