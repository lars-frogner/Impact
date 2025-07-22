//! Commands for operating the physics simulator.

use crate::{
    command::uils::{ModifiedActiveState, ToActiveState},
    engine::Engine,
    physics::PhysicsSimulator,
};
use impact_physics::{fph, medium::UniformMedium};
use roc_integration::roc;

#[roc(parents = "Command")]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PhysicsCommand {
    SetSimulation(ToActiveState),
    SetSimulationSubstepCount(ToSubstepCount),
    SetSimulationSpeed(ToSimulationSpeedMultiplier),
    SetMedium(UniformMedium),
}

#[roc(parents = "Command")]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToSubstepCount {
    HigherBy(u32),
    LowerBy(u32),
    Specific(u32),
}

#[roc(parents = "Command")]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Clone, Debug)]
pub enum ToSimulationSpeedMultiplier {
    Higher,
    Lower,
    Specific(fph),
}

impl PartialEq for ToSimulationSpeedMultiplier {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Higher, Self::Higher) | (Self::Lower, Self::Lower) => true,
            (Self::Specific(a), Self::Specific(b)) => a.to_bits() == b.to_bits(),
            _ => false,
        }
    }
}

impl Eq for ToSimulationSpeedMultiplier {}

pub fn set_simulation(simulator: &mut PhysicsSimulator, to: ToActiveState) -> ModifiedActiveState {
    impact_log::info!("Setting simulation to {to:?}");
    to.set(simulator.enabled_mut())
}

pub fn set_simulation_substep_count(simulator: &mut PhysicsSimulator, to: ToSubstepCount) -> u32 {
    impact_log::info!("Setting simulation substep count to {to:?}");
    let n_substeps = simulator.n_substeps_mut();
    match to {
        ToSubstepCount::HigherBy(incr) => {
            *n_substeps += incr;
        }
        ToSubstepCount::LowerBy(decr) => {
            *n_substeps = n_substeps.saturating_sub(decr).max(1);
        }
        ToSubstepCount::Specific(n) => {
            *n_substeps = n.max(1);
        }
    }
    *n_substeps
}

pub fn set_simulation_speed(
    simulator: &mut PhysicsSimulator,
    to: ToSimulationSpeedMultiplier,
) -> fph {
    impact_log::info!("Setting simulation speed to {to:?}");
    const INCREMENT_FACTOR: fph = 1.1;
    const MIN_ABS_MULTIPLIER: fph = 1e-9;

    let mut new_multiplier = match to {
        ToSimulationSpeedMultiplier::Higher => {
            simulator.simulation_speed_multiplier() * INCREMENT_FACTOR
        }
        ToSimulationSpeedMultiplier::Lower => {
            simulator.simulation_speed_multiplier() / INCREMENT_FACTOR
        }
        ToSimulationSpeedMultiplier::Specific(multiplier) => multiplier,
    };

    if new_multiplier.abs() < MIN_ABS_MULTIPLIER {
        new_multiplier = MIN_ABS_MULTIPLIER;
    }
    *simulator.simulation_speed_multiplier_mut() = new_multiplier;

    new_multiplier
}

pub fn set_medium(simulator: &mut PhysicsSimulator, to: UniformMedium) {
    impact_log::info!("Setting medium to {to:?}");
    simulator.set_medium(to);
}

pub fn set_simulation_speed_and_compensate_controller_movement_speed(
    engine: &Engine,
    to: ToSimulationSpeedMultiplier,
) -> f64 {
    let mut simulator = engine.simulator().write();
    let old_multiplier = simulator.simulation_speed_multiplier();
    let new_multiplier = set_simulation_speed(&mut simulator, to);
    drop(simulator);

    if new_multiplier != old_multiplier {
        // Adjust movement speed to compensate for the change in simulation speed
        if let Some(motion_controller) = engine.motion_controller() {
            let mut motion_controller = motion_controller.lock();
            let new_movement_speed =
                motion_controller.movement_speed() * (old_multiplier / new_multiplier);
            motion_controller.set_movement_speed(new_movement_speed);
        }
    }

    new_multiplier
}
