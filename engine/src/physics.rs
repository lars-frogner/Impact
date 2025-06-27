//! Simulation of physics.

pub mod collision;
pub mod command;
pub mod constraint;
pub mod entity;
pub mod inertia;
pub mod material;
pub mod medium;
pub mod motion;
pub mod rigid_body;
pub mod tasks;

use anyhow::{Result, bail};
use collision::{CollidableGeometry, CollisionWorld};
use constraint::{ConstraintManager, solver::ConstraintSolverConfig};
use impact_ecs::world::{EntityID, World as ECSWorld};
use medium::UniformMedium;
use num_traits::FromPrimitive;
use rigid_body::forces::{RigidBodyForceConfig, RigidBodyForceManager};
use serde::{Deserialize, Serialize};
use std::{sync::RwLock, time::Duration};

/// Floating point type used for physics simulation.
#[allow(non_camel_case_types)]
pub type fph = f64;

/// The manager of the physics simulation.
#[derive(Debug)]
pub struct PhysicsSimulator<G: CollidableGeometry = collision::geometry::voxel::CollidableGeometry>
{
    config: SimulatorConfig,
    rigid_body_force_manager: RwLock<RigidBodyForceManager>,
    constraint_manager: RwLock<ConstraintManager>,
    collision_world: RwLock<CollisionWorld<G>>,
    medium: UniformMedium,
    simulation_time: fph,
    time_step_duration: fph,
    simulation_speed_multiplier: fph,
}

/// Configuration parameters for physics.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct PhysicsConfig {
    /// Configuration parameters for the physics simulation.
    pub simulator: SimulatorConfig,
    /// Configuration parameters for rigid body force generation.
    pub rigid_body_force: RigidBodyForceConfig,
    /// Configuration parameters for the constraint solver.
    pub constraint_solver: ConstraintSolverConfig,
    /// The uniform medium in which physics is simulated.
    pub medium: UniformMedium,
}

/// Configuration parameters for the physics simulation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SimulatorConfig {
    /// Whether physics simulation is enabled. Disabling the simulation will not
    /// prevent controlled entities from moving.
    pub enabled: bool,
    /// The number of substeps to perform each simulation step. Increase to
    /// improve accuracy.
    pub n_substeps: u32,
    /// The duration to use for the first time step.
    pub initial_time_step_duration: fph,
    /// If `true`, the time step duration will be updated regularly to match the
    /// frame duration. This gives "real-time" simulation.
    pub match_frame_duration: bool,
    /// The factor by which to increase or decrease the simulation speed
    /// multiplier when requested.
    pub simulation_speed_multiplier_increment_factor: fph,
}

impl<G: CollidableGeometry> PhysicsSimulator<G> {
    /// Creates a new physics simulator with the given configuration parameters.
    ///
    /// # Errors
    /// Returns an error if any of the configuration parameters are invalid.
    pub fn new(
        PhysicsConfig {
            simulator: config,
            rigid_body_force: rigid_body_force_config,
            constraint_solver: constraint_solver_config,
            medium,
        }: PhysicsConfig,
    ) -> Result<Self> {
        config.validate()?;

        let time_step_duration = config.initial_time_step_duration;

        Ok(Self {
            config,
            rigid_body_force_manager: RwLock::new(RigidBodyForceManager::new(
                rigid_body_force_config,
            )?),
            constraint_manager: RwLock::new(ConstraintManager::new(constraint_solver_config)),
            collision_world: RwLock::new(CollisionWorld::new()),
            medium,
            simulation_time: 0.0,
            time_step_duration,
            simulation_speed_multiplier: 1.0,
        })
    }

    /// Whether physics simulation is enabled.
    pub fn enabled(&self) -> bool {
        self.config.enabled
    }

    /// Whether physics simulation is enabled.
    pub fn enabled_mut(&mut self) -> &mut bool {
        &mut self.config.enabled
    }

    /// The current base duration used for each time step (without the
    /// simulation speed multiplier).
    pub fn time_step_duration(&self) -> fph {
        self.time_step_duration
    }

    /// The current base duration used for each time step (without the
    /// simulation speed multiplier).
    pub fn time_step_duration_mut(&mut self) -> &mut fph {
        &mut self.time_step_duration
    }

    /// The current multiplier for the simulation speed.
    pub fn simulation_speed_multiplier(&self) -> fph {
        self.simulation_speed_multiplier
    }

    /// The current multiplier for the simulation speed.
    pub fn simulation_speed_multiplier_mut(&mut self) -> &mut fph {
        &mut self.simulation_speed_multiplier
    }

    /// The actual duration used for each time step (including the
    /// simulation speed multiplier).
    pub fn scaled_time_step_duration(&self) -> fph {
        self.time_step_duration * self.simulation_speed_multiplier
    }

    /// The time that have elapsed within the simulation.
    pub fn current_simulation_time(&self) -> fph {
        self.simulation_time
    }

    /// The number of substeps performed each simulation step.
    pub fn n_substeps(&self) -> u32 {
        self.config.n_substeps
    }

    /// The number of substeps performed each simulation step.
    pub fn n_substeps_mut(&mut self) -> &mut u32 {
        &mut self.config.n_substeps
    }

    /// Whether the time step duration is updated regularly to match the frame
    /// duration.
    pub fn matches_frame_duration(&self) -> bool {
        self.config.match_frame_duration
    }

    /// Whether the time step duration is updated regularly to match the frame
    /// duration.
    pub fn matches_frame_duration_mut(&mut self) -> &mut bool {
        &mut self.config.match_frame_duration
    }

    /// Returns a reference to the [`RigidBodyForceManager`], guarded by a
    /// [`RwLock`].
    pub fn rigid_body_force_manager(&self) -> &RwLock<RigidBodyForceManager> {
        &self.rigid_body_force_manager
    }

    /// Returns a reference to the [`ConstraintManager`], guarded by a
    /// [`RwLock`].
    pub fn constraint_manager(&self) -> &RwLock<ConstraintManager> {
        &self.constraint_manager
    }

    /// Returns a reference to the [`CollisionWorld`], guarded by a
    /// [`RwLock`].
    pub fn collision_world(&self) -> &RwLock<CollisionWorld<G>> {
        &self.collision_world
    }

    /// Sets the given medium for the physics simulation.
    pub fn set_medium(&mut self, medium: UniformMedium) {
        self.medium = medium;
    }

    /// If configured to do so, sets the time step duration to the given frame
    /// duration.
    pub fn update_time_step_duration(&mut self, frame_duration: &Duration) {
        if self.config.enabled && self.config.match_frame_duration {
            self.set_time_step_duration(frame_duration.as_secs_f64());
        }
    }

    /// Will use the given duration as the time step duration.
    pub fn set_time_step_duration(&mut self, time_step_duration: fph) {
        self.time_step_duration = time_step_duration;
    }

    /// Advances the physics simulation by one time step.
    pub fn advance_simulation(
        &mut self,
        ecs_world: &RwLock<ECSWorld>,
        collidable_context: &G::Context,
    ) {
        if !self.config.enabled {
            return;
        }
        impact_log::with_timing_info_logging!(
            "Simulation step with duration {:.2} ({:.1}x) and {} substeps",
            self.scaled_time_step_duration(),
            self.simulation_speed_multiplier,
            self.n_substeps(); {
            self.do_advance_simulation(ecs_world, collidable_context);
        });

        impact_log::info!("Simulation time: {:.1}", self.simulation_time);
    }

    fn do_advance_simulation(
        &mut self,
        ecs_world: &RwLock<ECSWorld>,
        collidable_context: &G::Context,
    ) {
        let mut entities_to_remove = Vec::new();

        let rigid_body_force_manager = self.rigid_body_force_manager.read().unwrap();
        let mut constraint_manager = self.constraint_manager.write().unwrap();
        let ecs_world_readonly = ecs_world.read().unwrap();

        let substep_duration = self.compute_substep_duration();
        for _ in 0..self.n_substeps() {
            Self::perform_step(
                &ecs_world_readonly,
                &rigid_body_force_manager,
                &mut constraint_manager,
                &self.collision_world,
                collidable_context,
                &self.medium,
                self.simulation_time,
                substep_duration,
                &mut entities_to_remove,
            );
            self.simulation_time += substep_duration;
        }

        rigid_body_force_manager.perform_post_simulation_step_actions(&ecs_world_readonly);

        drop(ecs_world_readonly);
        Self::remove_entities(ecs_world, &entities_to_remove);
    }

    fn compute_substep_duration(&self) -> fph {
        self.scaled_time_step_duration() / fph::from_u32(self.n_substeps()).unwrap()
    }

    fn perform_step(
        ecs_world: &ECSWorld,
        rigid_body_force_manager: &RigidBodyForceManager,
        constraint_manager: &mut ConstraintManager,
        collision_world: &RwLock<CollisionWorld<G>>,
        collidable_context: &G::Context,
        medium: &UniformMedium,
        current_simulation_time: fph,
        step_duration: fph,
        entities_to_remove: &mut Vec<EntityID>,
    ) {
        let new_simulation_time = current_simulation_time + step_duration;

        collision::systems::synchronize_collision_world(
            &mut collision_world.write().unwrap(),
            ecs_world,
        );

        motion::analytical::systems::apply_analytical_motion(ecs_world, new_simulation_time);

        if constraint_manager.solver().config().enabled {
            impact_log::with_timing_info_logging!("Preparing constraints"; {
                constraint_manager.prepare_constraints(ecs_world, &collision_world.read().unwrap(), collidable_context);
            });
        }

        rigid_body::systems::advance_rigid_body_velocities(ecs_world, step_duration);

        if constraint_manager.solver().config().enabled {
            impact_log::with_timing_info_logging!("Solving constraints"; {
                constraint_manager.compute_and_apply_constrained_state(ecs_world);
            });
        }

        rigid_body::systems::advance_rigid_body_configurations(ecs_world, step_duration);

        rigid_body_force_manager.apply_forces_and_torques(ecs_world, medium, entities_to_remove);
    }

    fn remove_entities(ecs_world: &RwLock<ECSWorld>, entities_to_remove: &Vec<EntityID>) {
        if !entities_to_remove.is_empty() {
            let mut ecs_world_write = ecs_world.write().unwrap();

            for entity_id in entities_to_remove {
                ecs_world_write.remove_entity(*entity_id).unwrap();
            }
        }
    }
}

impl SimulatorConfig {
    fn validate(&self) -> Result<()> {
        if self.n_substeps == 0 {
            bail!(
                "Invalid number of substeps for physics simulation: {}",
                self.n_substeps
            );
        }
        if self.initial_time_step_duration <= 0.0 {
            bail!(
                "Invalid initial time step duration for physics simulation: {}",
                self.initial_time_step_duration
            );
        }
        if self.simulation_speed_multiplier_increment_factor <= 1.0 {
            bail!(
                "Invalid simulation speed increment factor: {}",
                self.simulation_speed_multiplier_increment_factor
            );
        }
        Ok(())
    }
}

impl Default for SimulatorConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            n_substeps: 1,
            initial_time_step_duration: 0.001,
            match_frame_duration: true,
            simulation_speed_multiplier_increment_factor: 1.1,
        }
    }
}
