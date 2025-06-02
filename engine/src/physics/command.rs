//! Commands for operating the physics simulator.

use super::PhysicsSimulator;
use crate::{
    command::ToActiveState,
    physics::{fph, medium::UniformMedium},
};
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

impl PhysicsSimulator {
    pub fn set_simulation_substep_count(&mut self, to: ToSubstepCount) -> u32 {
        match to {
            ToSubstepCount::HigherBy(incr) => {
                self.config.n_substeps += incr;
            }
            ToSubstepCount::LowerBy(decr) => {
                self.config.n_substeps = self.config.n_substeps.saturating_sub(decr).max(1);
            }
            ToSubstepCount::Specific(n_substeps) => {
                self.config.n_substeps = n_substeps.max(1);
            }
        }
        self.config.n_substeps
    }

    pub fn set_simulation_speed(&mut self, to: ToSimulationSpeedMultiplier) -> fph {
        const MIN_ABS_MULTIPLIER: fph = 1e-9;

        let mut new_multiplier = match to {
            ToSimulationSpeedMultiplier::Higher => {
                self.simulation_speed_multiplier
                    * self.config.simulation_speed_multiplier_increment_factor
            }
            ToSimulationSpeedMultiplier::Lower => {
                self.simulation_speed_multiplier
                    / self.config.simulation_speed_multiplier_increment_factor
            }
            ToSimulationSpeedMultiplier::Specific(multiplier) => multiplier,
        };

        if new_multiplier.abs() < MIN_ABS_MULTIPLIER {
            new_multiplier = MIN_ABS_MULTIPLIER;
        }
        self.simulation_speed_multiplier = new_multiplier;

        new_multiplier
    }
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
