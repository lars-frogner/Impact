//! Simulation of physics.

pub mod collision;
pub mod components;
pub mod constraint;
pub mod entity;
pub mod inertia;
pub mod medium;
pub mod motion;
pub mod rigid_body;
pub mod tasks;

use anyhow::{Result, bail};
use collision::CollisionWorld;
use constraint::ConstraintManager;
use impact_ecs::world::{Entity, World as ECSWorld};
use medium::UniformMedium;
use num_traits::FromPrimitive;
use rigid_body::forces::{RigidBodyForceConfig, RigidBodyForceManager};
use std::{sync::RwLock, time::Duration};

/// Floating point type used for physics simulation.
#[allow(non_camel_case_types)]
pub type fph = f64;

/// The manager of the physics simulation.
#[derive(Debug)]
pub struct PhysicsSimulator {
    config: SimulatorConfig,
    rigid_body_force_manager: RwLock<RigidBodyForceManager>,
    constraint_manager: RwLock<ConstraintManager>,
    collision_world: RwLock<CollisionWorld>,
    medium: UniformMedium,
    simulation_time: fph,
    time_step_duration: fph,
    simulation_speed_multiplier: fph,
}

/// Configuration parameters for the physics simulation.
#[derive(Clone, Debug)]
pub struct SimulatorConfig {
    /// The number of substeps to perform each simulation step. Increase to
    /// improve accuracy.
    pub n_substeps: u32,
    /// The duration to use for the first time step.
    pub initial_time_step_duration: fph,
    /// If `true`, the time step duration will be updated regularly to match the
    /// frame duration. This gives "real-time" simulation.
    pub match_frame_duration: bool,
    /// The factor by which to increase or decrease the simulation speed
    /// multiplyer when calling
    /// [`increment_simulation_speed_multiplier`](PhysicsSimulator::increment_simulation_speed_multiplier)
    /// or
    /// [`decrement_simulation_speed_multiplier`](PhysicsSimulator::decrement_simulation_speed_multiplier).
    pub simulation_speed_multiplier_increment_factor: fph,
    /// Configuration parameters for rigid body force generation. If [`None`],
    /// default parameters are used.
    pub rigid_body_force_config: Option<RigidBodyForceConfig>,
}

impl PhysicsSimulator {
    /// Creates a new physics simulator with the given configuration parameters
    /// and uniform physical medium.
    ///
    /// # Errors
    /// Returns an error if any of the configuration parameters are invalid.
    pub fn new(mut config: SimulatorConfig, medium: UniformMedium) -> Result<Self> {
        config.validate()?;

        let rigid_body_force_config = config.rigid_body_force_config.take().unwrap_or_default();

        let time_step_duration = config.initial_time_step_duration;

        Ok(Self {
            config,
            rigid_body_force_manager: RwLock::new(RigidBodyForceManager::new(
                rigid_body_force_config,
            )?),
            constraint_manager: RwLock::new(ConstraintManager::new()),
            collision_world: RwLock::new(CollisionWorld::new()),
            medium,
            simulation_time: 0.0,
            time_step_duration,
            simulation_speed_multiplier: 1.0,
        })
    }

    /// Returns the current base duration used for each time step (without the
    /// simulation speed multiplier).
    pub fn time_step_duration(&self) -> fph {
        self.time_step_duration
    }

    /// Returns the actual duration used for each time step (including the
    /// simulation speed multiplier).
    pub fn scaled_time_step_duration(&self) -> fph {
        self.time_step_duration * self.simulation_speed_multiplier
    }

    /// Returns the time that have elapsed within the simulation.
    pub fn current_simulation_time(&self) -> fph {
        self.simulation_time
    }

    /// Returns the number of substeps performed each simulation step.
    pub fn n_substeps(&self) -> u32 {
        self.config.n_substeps
    }

    /// Returns the factor by which to increase or decrease the simulation speed
    /// multiplyer when calling
    /// [`increment_simulation_speed_multiplier`](PhysicsSimulator::increment_simulation_speed_multiplier)
    /// or
    /// [`decrement_simulation_speed_multiplier`](PhysicsSimulator::decrement_simulation_speed_multiplier).
    pub fn simulation_speed_multiplier_increment_factor(&self) -> fph {
        self.config.simulation_speed_multiplier_increment_factor
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
    pub fn collision_world(&self) -> &RwLock<CollisionWorld> {
        &self.collision_world
    }

    /// Sets the given medium for the physics simulation.
    pub fn set_medium(&mut self, medium: UniformMedium) {
        self.medium = medium;
    }

    /// If configured to do so, sets the time step duration to the given frame
    /// duration.
    pub fn update_time_step_duration(&mut self, frame_duration: &Duration) {
        if self.config.match_frame_duration {
            self.set_time_step_duration(frame_duration.as_secs_f64());
        }
    }

    /// Will use the given duration as the time step duration.
    pub fn set_time_step_duration(&mut self, time_step_duration: fph) {
        self.time_step_duration = time_step_duration;
    }

    /// Will execute the given number of substeps each simulation step.
    pub fn set_n_substeps(&mut self, n_substeps: u32) {
        self.config.n_substeps = n_substeps;
    }

    /// Increment the number of substeps by one.
    pub fn increment_n_substeps(&mut self) {
        self.config.n_substeps += 1;
    }

    /// Decrement the number of substeps by one, to a minimum of unity.
    pub fn decrement_n_substeps(&mut self) {
        if self.config.n_substeps > 1 {
            self.config.n_substeps -= 1;
        }
    }

    /// Will use the given multiplier to scale the simulation time step
    /// duration.
    pub fn set_simulation_speed_multiplier(&mut self, simulation_speed_multiplier: fph) {
        self.simulation_speed_multiplier = simulation_speed_multiplier;
    }

    /// Increases the simulation speed multiplier by the
    /// `simulation_speed_multiplier_increment_factor` specified in the
    /// configuration.
    pub fn increment_simulation_speed_multiplier(&mut self) {
        self.simulation_speed_multiplier *=
            self.config.simulation_speed_multiplier_increment_factor;
    }

    /// Decreases the simulation speed multiplier by the
    /// `simulation_speed_multiplier_increment_factor` specified in the
    /// configuration.
    pub fn decrement_simulation_speed_multiplier(&mut self) {
        self.simulation_speed_multiplier /=
            self.config.simulation_speed_multiplier_increment_factor;
    }

    /// Performs any setup required before starting the game loop.
    pub fn perform_setup_for_game_loop(&self, ecs_world: &RwLock<ECSWorld>) {
        motion::analytical::systems::apply_analytical_motion(
            &ecs_world.read().unwrap(),
            self.simulation_time,
        );

        self.apply_forces_and_torques(ecs_world);
    }

    /// Advances the physics simulation by one time step.
    pub fn advance_simulation(&mut self, ecs_world: &RwLock<ECSWorld>) {
        with_timing_info_logging!(
        "Simulation step with duration {:.2} ({:.1}x) and {} substeps",
        self.scaled_time_step_duration(),
        self.simulation_speed_multiplier,
        self.n_substeps(); {
            self.do_advance_simulation(ecs_world);
        });

        log::info!("Simulation time: {:.1}", self.simulation_time);
    }

    fn do_advance_simulation(&mut self, ecs_world: &RwLock<ECSWorld>) {
        let mut entities_to_remove = Vec::new();

        let rigid_body_force_manager = self.rigid_body_force_manager.read().unwrap();
        let constraint_manager = self.constraint_manager.read().unwrap();
        let ecs_world_readonly = ecs_world.read().unwrap();

        let substep_duration = self.compute_substep_duration();
        for _ in 0..self.n_substeps() {
            Self::perform_step(
                &ecs_world_readonly,
                &rigid_body_force_manager,
                &constraint_manager,
                &self.collision_world,
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
        constraint_manager: &ConstraintManager,
        collision_world: &RwLock<CollisionWorld>,
        medium: &UniformMedium,
        current_simulation_time: fph,
        step_duration: fph,
        entities_to_remove: &mut Vec<Entity>,
    ) {
        let new_simulation_time = current_simulation_time + step_duration;

        collision::systems::synchronize_collision_world(
            &mut collision_world.write().unwrap(),
            ecs_world,
        );

        motion::analytical::systems::apply_analytical_motion(ecs_world, new_simulation_time);

        rigid_body::systems::advance_rigid_body_velocities(ecs_world, step_duration);

        constraint_manager.prepare_constraints(ecs_world, &collision_world.read().unwrap());
        constraint_manager.compute_and_apply_constrained_velocities(ecs_world);

        rigid_body::systems::advance_rigid_body_configurations(ecs_world, step_duration);

        rigid_body_force_manager.apply_forces_and_torques(ecs_world, medium, entities_to_remove);
    }

    fn apply_forces_and_torques(&self, ecs_world: &RwLock<ECSWorld>) {
        let mut entities_to_remove = Vec::new();

        self.rigid_body_force_manager
            .read()
            .unwrap()
            .apply_forces_and_torques(
                &ecs_world.read().unwrap(),
                &self.medium,
                &mut entities_to_remove,
            );

        Self::remove_entities(ecs_world, &entities_to_remove);
    }

    fn remove_entities(ecs_world: &RwLock<ECSWorld>, entities_to_remove: &Vec<Entity>) {
        if !entities_to_remove.is_empty() {
            let mut ecs_world_write = ecs_world.write().unwrap();

            for entity in entities_to_remove {
                ecs_world_write.remove_entity(entity).unwrap();
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
            n_substeps: 1,
            initial_time_step_duration: 0.015,
            match_frame_duration: true,
            simulation_speed_multiplier_increment_factor: 1.1,
            rigid_body_force_config: None,
        }
    }
}
