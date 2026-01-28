//! Commands for operating the physics simulator.

use crate::{
    command::uils::{ModifiedActiveState, ToActiveState},
    engine::Engine,
    lock_order::{OrderedMutex, OrderedRwLock},
    physics::PhysicsSimulator,
};
use anyhow::{Context, Result, anyhow};
use impact_ecs::world::EntityID;
use impact_physics::{
    constraint::solver::ConstraintSolverConfig,
    force::alignment_torque::AlignmentDirection,
    quantities::{ForceC, ImpulseC, Motion, PositionC},
};
use roc_integration::roc;

#[roc(parents = "Command")]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Clone, Debug, PartialEq)]
pub enum PhysicsCommand {
    SetGravitationalConstant(f32),
    UpdateLocalForce {
        entity_id: EntityID,
        mode: LocalForceUpdateMode,
        force: ForceC,
    },
    SetAlignmentTorqueDirection {
        entity_id: EntityID,
        direction: AlignmentDirection,
    },
    ApplyImpulse {
        entity_id: EntityID,
        impulse: ImpulseC,
        relative_position: PositionC,
    },
    AddMassRetainingMotion {
        entity_id: EntityID,
        additional_mass: f32,
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

pub fn set_gravitational_constant(simulator: &PhysicsSimulator, to: f32) {
    let mut force_generator_manager = simulator.force_generator_manager().owrite();

    force_generator_manager
        .dynamic_gravity_manager_mut()
        .set_gravitational_constant(to);
}

pub fn update_local_force(
    engine: &Engine,
    entity_id: EntityID,
    mode: LocalForceUpdateMode,
    force: ForceC,
) -> Result<()> {
    let generator_id = engine
        .get_component_copy(entity_id)
        .context("Failed to get `LocalForceGeneratorID` component for local force update")?;

    let simulator = engine.simulator().oread();
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
    engine: &Engine,
    entity_id: EntityID,
    direction: AlignmentDirection,
) -> Result<()> {
    let generator_id = engine
        .get_component_copy(entity_id)
        .context("Failed to get `AlignmentTorqueGeneratorID` component for setting alignement torque direction")?;

    let simulator = engine.simulator().oread();
    let mut force_generator_manager = simulator.force_generator_manager().owrite();

    let alignment_torque = force_generator_manager
        .alignment_torques_mut()
        .get_generator_mut(&generator_id)
        .ok_or_else(|| anyhow!("No alignment torque with ID {}", u64::from(generator_id)))?;

    alignment_torque.alignment_direction = direction;

    Ok(())
}

pub fn apply_impulse(
    engine: &Engine,
    entity_id: EntityID,
    impulse: ImpulseC,
    relative_position: PositionC,
) -> Result<()> {
    let rigid_body_id = engine
        .get_component_copy(entity_id)
        .context("Failed to get `DynamicRigidBodyID` component for applying impulse")?;

    let (new_velocity, new_angular_velocity) = {
        let simulator = engine.simulator().oread();
        let mut rigid_body_manager = simulator.rigid_body_manager().owrite();

        let rigid_body = rigid_body_manager
            .get_dynamic_rigid_body_mut(rigid_body_id)
            .ok_or_else(|| anyhow!("No rigid body with ID {}", u64::from(rigid_body_id)))?;

        let impulse = impulse.aligned();
        let relative_position = relative_position.aligned();

        rigid_body.apply_impulse(&impulse, &relative_position);

        (
            rigid_body.compute_velocity(),
            rigid_body.compute_angular_velocity(),
        )
    };

    engine.with_component_mut(entity_id, |motion: &mut Motion| {
        motion.linear_velocity = new_velocity.compact();
        motion.angular_velocity = new_angular_velocity.compact();
        Ok(())
    })
}

pub fn add_mass_retaining_motion(
    engine: &Engine,
    entity_id: EntityID,
    additional_mass: f32,
) -> Result<()> {
    let rigid_body_id = engine
        .get_component_copy(entity_id)
        .context("Failed to get `DynamicRigidBodyID` component for adding mass")?;

    let simulator = engine.simulator().oread();
    let mut rigid_body_manager = simulator.rigid_body_manager().owrite();

    let rigid_body = rigid_body_manager
        .get_dynamic_rigid_body_mut(rigid_body_id)
        .ok_or_else(|| anyhow!("No rigid body with ID {}", u64::from(rigid_body_id)))?;

    let new_mass = rigid_body.mass() + additional_mass;
    rigid_body.set_mass_retaining_motion(new_mass);

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
