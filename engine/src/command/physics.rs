//! Commands for operating the physics simulator.

use crate::{
    command::uils::{ModifiedActiveState, ToActiveState},
    engine::Engine,
    lock_order::{OrderedMutex, OrderedRwLock},
    physics::PhysicsSimulator,
};
use anyhow::{Result, anyhow};
use impact_physics::{
    constraint::solver::ConstraintSolverConfig,
    force::{
        alignment_torque::{AlignmentDirection, AlignmentTorqueGeneratorID},
        local_force::LocalForceGeneratorID,
    },
    quantities::ForceP,
};
use roc_integration::roc;

#[roc(parents = "Command")]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Clone, Debug)]
pub enum PhysicsCommand {
    UpdateLocalForce {
        generator_id: LocalForceGeneratorID,
        mode: LocalForceUpdateMode,
        force: ForceP,
    },
    SetAlignmentTorqueDirection {
        generator_id: AlignmentTorqueGeneratorID,
        direction: AlignmentDirection,
    },
}

#[roc(parents = "Command")]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LocalForceUpdateMode {
    Set,
    Add,
}

#[derive(Clone, Debug)]
pub enum PhysicsAdminCommand {
    SetSimulation(ToActiveState),
    SetSimulationSubstepCount(ToSubstepCount),
    SetSimulationSpeed(ToSimulationSpeedMultiplier),
    SetTimeStepDuration(f32),
    SetMatchFrameDuration(ToActiveState),
    SetConstraintSolverConfig(ConstraintSolverConfig),
}

#[derive(Clone, Copy, Debug)]
pub enum ToSubstepCount {
    HigherBy(u32),
    LowerBy(u32),
    Specific(u32),
}

#[derive(Clone, Debug)]
pub enum ToSimulationSpeedMultiplier {
    Higher,
    Lower,
    Specific(f32),
}

impl PartialEq for PhysicsCommand {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                Self::UpdateLocalForce {
                    generator_id: self_id,
                    mode: self_mode,
                    force: self_force,
                },
                Self::UpdateLocalForce {
                    generator_id: other_id,
                    mode: other_mode,
                    force: other_force,
                },
            ) => {
                self_id == other_id
                    && self_mode == other_mode
                    && bytemuck::bytes_of(self_force) == bytemuck::bytes_of(other_force)
            }
            (
                Self::SetAlignmentTorqueDirection {
                    generator_id: self_id,
                    direction: self_direction,
                },
                Self::SetAlignmentTorqueDirection {
                    generator_id: other_id,
                    direction: other_direction,
                },
            ) => {
                if self_id != other_id {
                    return false;
                }
                match (self_direction, other_direction) {
                    (
                        AlignmentDirection::Fixed(self_direction),
                        AlignmentDirection::Fixed(other_direction),
                    ) => bytemuck::bytes_of(self_direction) == bytemuck::bytes_of(other_direction),
                    (AlignmentDirection::GravityForce, AlignmentDirection::GravityForce) => true,
                    _ => false,
                }
            }
            _ => false,
        }
    }
}

impl Eq for PhysicsCommand {}

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

pub fn update_local_force(
    simulator: &PhysicsSimulator,
    generator_id: LocalForceGeneratorID,
    mode: LocalForceUpdateMode,
    force: ForceP,
) -> Result<()> {
    let mut force_generator_manager = simulator.force_generator_manager().owrite();

    let local_force = force_generator_manager
        .local_forces_mut()
        .get_generator_mut(&generator_id)
        .ok_or_else(|| anyhow!("No local force with ID {}", u64::from(generator_id)))?;

    match mode {
        LocalForceUpdateMode::Set => {
            local_force.force = force;
        }
        LocalForceUpdateMode::Add => {
            local_force.force += force;
        }
    }

    Ok(())
}

pub fn set_alignment_torque_direction(
    simulator: &PhysicsSimulator,
    generator_id: AlignmentTorqueGeneratorID,
    direction: AlignmentDirection,
) -> Result<()> {
    let mut force_generator_manager = simulator.force_generator_manager().owrite();

    let alignment_torque = force_generator_manager
        .alignment_torques_mut()
        .get_generator_mut(&generator_id)
        .ok_or_else(|| anyhow!("No alignment torque with ID {}", u64::from(generator_id)))?;

    alignment_torque.alignment_direction = direction;

    Ok(())
}

pub fn set_simulation(simulator: &mut PhysicsSimulator, to: ToActiveState) -> ModifiedActiveState {
    log::info!("Setting simulation to {to:?}");
    to.set(simulator.enabled_mut())
}

pub fn set_simulation_substep_count(simulator: &mut PhysicsSimulator, to: ToSubstepCount) -> u32 {
    log::info!("Setting simulation substep count to {to:?}");
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
) -> f32 {
    log::info!("Setting simulation speed to {to:?}");
    const INCREMENT_FACTOR: f32 = 1.1;
    const MIN_ABS_MULTIPLIER: f32 = 1e-9;

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

pub fn set_simulation_speed_and_compensate_controller_movement_speed(
    engine: &Engine,
    to: ToSimulationSpeedMultiplier,
) -> f32 {
    let mut simulator = engine.simulator().owrite();
    let old_multiplier = simulator.simulation_speed_multiplier();
    let new_multiplier = set_simulation_speed(&mut simulator, to);
    drop(simulator);

    if new_multiplier != old_multiplier {
        // Adjust movement speed to compensate for the change in simulation speed
        if let Some(motion_controller) = engine.motion_controller() {
            let mut motion_controller = motion_controller.olock();
            let new_movement_speed =
                motion_controller.movement_speed() * (old_multiplier / new_multiplier);
            motion_controller.set_movement_speed(new_movement_speed);
        }
    }

    new_multiplier
}

pub fn set_time_step_duration(simulator: &mut PhysicsSimulator, duration: f32) -> f32 {
    log::info!("Setting time step duration to {duration:?}");
    *simulator.time_step_duration_mut() = duration;
    duration
}

pub fn set_match_frame_duration(
    simulator: &mut PhysicsSimulator,
    to: ToActiveState,
) -> ModifiedActiveState {
    log::info!("Setting match frame duration to {to:?}");
    to.set(simulator.matches_frame_duration_mut())
}

pub fn set_constraint_solver_config(
    simulator: &mut PhysicsSimulator,
    config: ConstraintSolverConfig,
) {
    log::info!("Setting constraint solver config to {config:?}");
    let mut constraint_manager = simulator.constraint_manager().owrite();
    *constraint_manager.solver_mut().config_mut() = config;
}
