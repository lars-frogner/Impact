//! Simulation of physics.

#[macro_use]
mod macros;

pub mod collision;
pub mod constraint;
pub mod driven_motion;
pub mod force;
pub mod inertia;
pub mod material;
pub mod medium;
pub mod quantities;
pub mod rigid_body;
pub mod setup;

#[cfg(feature = "ecs")]
pub mod systems;

use anyhow::{Result, bail};
use collision::{Collidable, CollisionWorld};
use constraint::{ConstraintManager, solver::ConstraintSolverConfig};
use driven_motion::MotionDriverManager;
use force::{ForceGenerationConfig, ForceGeneratorManager};
use medium::UniformMedium;
use num_traits::FromPrimitive;
use parking_lot::RwLock;
use rigid_body::RigidBodyManager;
use std::time::Duration;

/// Floating point type used for physics simulation.
#[allow(non_camel_case_types)]
pub type fph = f64;

/// The manager of the physics simulation.
#[derive(Debug)]
pub struct PhysicsSimulator<C: Collidable = collision::collidable::basic::Collidable> {
    config: SimulatorConfig,
    rigid_body_manager: RwLock<RigidBodyManager>,
    force_generator_manager: RwLock<ForceGeneratorManager>,
    motion_driver_manager: RwLock<MotionDriverManager>,
    constraint_manager: RwLock<ConstraintManager>,
    collision_world: RwLock<CollisionWorld<C>>,
    medium: UniformMedium,
    simulation_time: fph,
    time_step_duration: fph,
    simulation_speed_multiplier: fph,
}

/// Configuration parameters for physics.
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(default)
)]
#[derive(Clone, Debug, Default)]
pub struct PhysicsConfig {
    /// Configuration parameters for the physics simulation.
    pub simulator: SimulatorConfig,
    /// Configuration parameters for rigid body force generation.
    pub rigid_body_force: ForceGenerationConfig,
    /// Configuration parameters for the constraint solver.
    pub constraint_solver: ConstraintSolverConfig,
    /// The uniform medium in which physics is simulated.
    pub medium: UniformMedium,
}

/// Configuration parameters for the physics simulation.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
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
}

impl<C: Collidable> PhysicsSimulator<C> {
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
            rigid_body_manager: RwLock::new(RigidBodyManager::new()),
            force_generator_manager: RwLock::new(ForceGeneratorManager::new(
                rigid_body_force_config,
            )?),
            motion_driver_manager: RwLock::new(MotionDriverManager::new()),
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

    /// Returns a reference to the [`RigidBodyManager`], guarded by a
    /// [`RwLock`].
    pub fn rigid_body_manager(&self) -> &RwLock<RigidBodyManager> {
        &self.rigid_body_manager
    }

    /// Returns a reference to the [`ForceGeneratorManager`], guarded by a
    /// [`RwLock`].
    pub fn force_generator_manager(&self) -> &RwLock<ForceGeneratorManager> {
        &self.force_generator_manager
    }

    /// Returns a reference to the [`MotionDriverManager`], guarded by a
    /// [`RwLock`].
    pub fn motion_driver_manager(&self) -> &RwLock<MotionDriverManager> {
        &self.motion_driver_manager
    }

    /// Returns a reference to the [`ConstraintManager`], guarded by a
    /// [`RwLock`].
    pub fn constraint_manager(&self) -> &RwLock<ConstraintManager> {
        &self.constraint_manager
    }

    /// Returns a reference to the [`CollisionWorld`], guarded by a
    /// [`RwLock`].
    pub fn collision_world(&self) -> &RwLock<CollisionWorld<C>> {
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
    pub fn advance_simulation(&mut self, collidable_context: &C::Context) {
        if !self.config.enabled {
            return;
        }
        impact_log::with_timing_info_logging!(
            "Simulation step with duration {:.2} ({:.1}x) and {} substeps",
            self.scaled_time_step_duration(),
            self.simulation_speed_multiplier,
            self.n_substeps(); {
            self.do_advance_simulation(collidable_context);
        });

        impact_log::info!("Simulation time: {:.1}", self.simulation_time);
    }

    /// Resets the simulator to the initial empty state and sets the simulation
    /// time to zero.
    pub fn reset(&mut self) {
        self.rigid_body_manager.write().clear();
        self.force_generator_manager.write().clear();
        self.motion_driver_manager.write().clear();
        self.constraint_manager.write().clear();
        self.collision_world.write().clear();
        self.simulation_time = 0.0;
    }

    fn do_advance_simulation(&mut self, collidable_context: &C::Context) {
        let mut rigid_body_manager = self.rigid_body_manager.write();
        let force_generator_manager = self.force_generator_manager.read();
        let motion_driver_manager = self.motion_driver_manager.read();
        let mut constraint_manager = self.constraint_manager.write();
        let mut collision_world = self.collision_world.write();

        let substep_duration = self.compute_substep_duration();
        for _ in 0..self.n_substeps() {
            Self::perform_step(
                &mut rigid_body_manager,
                &force_generator_manager,
                &motion_driver_manager,
                &mut constraint_manager,
                &mut collision_world,
                collidable_context,
                &self.medium,
                self.simulation_time,
                substep_duration,
            );
            self.simulation_time += substep_duration;
        }
    }

    fn compute_substep_duration(&self) -> fph {
        self.scaled_time_step_duration() / fph::from_u32(self.n_substeps()).unwrap()
    }

    fn perform_step(
        rigid_body_manager: &mut RigidBodyManager,
        force_generator_manager: &ForceGeneratorManager,
        motion_driver_manager: &MotionDriverManager,
        constraint_manager: &mut ConstraintManager,
        collision_world: &mut CollisionWorld<C>,
        collidable_context: &C::Context,
        medium: &UniformMedium,
        current_simulation_time: fph,
        step_duration: fph,
    ) {
        let new_simulation_time = current_simulation_time + step_duration;

        collision_world.synchronize_collidables_with_rigid_bodies(rigid_body_manager);

        if constraint_manager.solver().config().enabled {
            impact_log::with_timing_info_logging!("Preparing constraints"; {
                constraint_manager.prepare_constraints(
                    rigid_body_manager,
                    collision_world,
                    collidable_context,
                );
            });
        }

        rigid_body_manager.advance_dynamic_rigid_body_momenta(step_duration);

        if constraint_manager.solver().config().enabled {
            impact_log::with_timing_info_logging!("Solving constraints"; {
                constraint_manager.compute_and_apply_constrained_state(rigid_body_manager);
            });
        }

        rigid_body_manager.advance_dynamic_rigid_body_configurations(step_duration);

        // We really only want to advance non-driven kinematic bodies, but since
        // the bodies with a motion driver will have their state overwritten
        // when we call `apply_motion` anyway, we advance all kinematic bodies
        // here for simplicity
        rigid_body_manager.advance_kinematic_rigid_body_configurations(step_duration);

        motion_driver_manager.apply_motion(rigid_body_manager, new_simulation_time);

        force_generator_manager.apply_forces_and_torques(medium, rigid_body_manager);
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
        }
    }
}
