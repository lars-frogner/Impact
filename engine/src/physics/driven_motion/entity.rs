//! Management of driven motion for entities.

use impact_ecs::{archetype::ArchetypeComponentStorage, setup};
use impact_physics::{
    driven_motion::{
        MotionDriverManager,
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
    rigid_body::KinematicRigidBodyID,
};
use std::sync::RwLock;

pub fn setup_driven_motion_for_new_entity(
    motion_driver_manager: &RwLock<MotionDriverManager>,
    components: &mut ArchetypeComponentStorage,
) {
    setup!(
        {
            let mut motion_driver_manager = motion_driver_manager.write().unwrap();
        },
        components,
        |rigid_body_id: &KinematicRigidBodyID,
         trajectory: &CircularTrajectory|
         -> CircularTrajectoryDriverID {
            setup::setup_circular_trajectory(
                &mut motion_driver_manager,
                *rigid_body_id,
                *trajectory,
            )
        }
    );

    setup!(
        {
            let mut motion_driver_manager = motion_driver_manager.write().unwrap();
        },
        components,
        |rigid_body_id: &KinematicRigidBodyID,
         trajectory: &ConstantAccelerationTrajectory|
         -> ConstantAccelerationTrajectoryDriverID {
            setup::setup_constant_acceleration_trajectory(
                &mut motion_driver_manager,
                *rigid_body_id,
                *trajectory,
            )
        }
    );

    setup!(
        {
            let mut motion_driver_manager = motion_driver_manager.write().unwrap();
        },
        components,
        |rigid_body_id: &KinematicRigidBodyID,
         rotation: &ConstantRotation|
         -> ConstantRotationDriverID {
            setup::setup_constant_rotation(&mut motion_driver_manager, *rigid_body_id, *rotation)
        }
    );

    setup!(
        {
            let mut motion_driver_manager = motion_driver_manager.write().unwrap();
        },
        components,
        |rigid_body_id: &KinematicRigidBodyID,
         trajectory: &HarmonicOscillatorTrajectory|
         -> HarmonicOscillatorTrajectoryDriverID {
            setup::setup_harmonic_oscillator_trajectory(
                &mut motion_driver_manager,
                *rigid_body_id,
                *trajectory,
            )
        }
    );

    setup!(
        {
            let mut motion_driver_manager = motion_driver_manager.write().unwrap();
        },
        components,
        |rigid_body_id: &KinematicRigidBodyID,
         trajectory: &OrbitalTrajectory|
         -> OrbitalTrajectoryDriverID {
            setup::setup_orbital_trajectory(&mut motion_driver_manager, *rigid_body_id, *trajectory)
        }
    );
}
