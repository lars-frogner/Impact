//! Commands for operating the physics simulator.

use crate::{command::ToActiveState, physics::PhysicsSimulator};
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

pub fn set_simulation_substep_count(simulator: &mut PhysicsSimulator, to: ToSubstepCount) -> u32 {
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
