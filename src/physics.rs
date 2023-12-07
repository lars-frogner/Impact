//! Simulation of physics.

mod collision;
mod events;
mod inertia;
mod motion;
mod rigid_body;
mod tasks;
mod time;

pub use inertia::{compute_convex_triangle_mesh_volume, InertiaTensor, InertialProperties};
pub use motion::{
    advance_orientation, Acceleration, AnalyticalMotionManager, AngularMomentum, AngularVelocity,
    AngularVelocityComp, CircularTrajectoryComp, ConstantAccelerationTrajectoryComp,
    ConstantRotationComp, Direction, Force, HarmonicOscillatorTrajectoryComp, LogsKineticEnergy,
    LogsMomentum, Momentum, OrbitalTrajectoryComp, Orientation, Position, SpatialConfigurationComp,
    Static, Torque, Velocity, VelocityComp,
};
pub use rigid_body::{
    EulerCromerStep, RigidBody, RigidBodyComp, RigidBodyForceManager, RungeKutta4Substep, Spring,
    SpringComp, SteppingScheme, UniformGravityComp, UniformRigidBodyComp,
};
pub use tasks::{AdvanceSimulation, PhysicsTag};

use impact_ecs::{
    query,
    world::{Entity, World as ECSWorld},
};
use num_traits::FromPrimitive;
use rigid_body::SchemeSubstep;
use std::{collections::LinkedList, sync::RwLock, time::Duration};

/// Floating point type used for physics simulation.
#[allow(non_camel_case_types)]
pub type fph = f64;

/// The manager of the physics simulation.
#[derive(Debug)]
pub struct PhysicsSimulator {
    config: SimulatorConfig,
    analytical_motion_manager: RwLock<AnalyticalMotionManager>,
    rigid_body_force_manager: RwLock<RigidBodyForceManager>,
    simulation_time: fph,
    time_step_duration: fph,
    simulation_speed_multiplier: fph,
}

/// Configuration parameters for the physics simulation.
#[derive(Clone, Debug)]
pub struct SimulatorConfig {
    /// The iterative scheme to use for advancing the motion of rigid bodies
    /// over time.
    pub stepping_scheme: SteppingScheme,
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
}

impl PhysicsSimulator {
    /// Creates a new physics simulater with the given configuration parameters.
    pub fn new(config: SimulatorConfig) -> Self {
        let time_step_duration = config.initial_time_step_duration;
        Self {
            config,
            analytical_motion_manager: RwLock::new(AnalyticalMotionManager::new()),
            rigid_body_force_manager: RwLock::new(RigidBodyForceManager::new()),
            simulation_time: 0.0,
            time_step_duration,
            simulation_speed_multiplier: 1.0,
        }
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
    pub fn stepping_scheme(&self) -> SteppingScheme {
        self.config.stepping_scheme
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

    /// Returns a reference to the [`AnalyticalMotionManager`], guarded by a
    /// [`RwLock`].
    pub fn analytical_motion_manager(&self) -> &RwLock<AnalyticalMotionManager> {
        &self.analytical_motion_manager
    }

    /// Returns a reference to the [`RigidBodyForceManager`], guarded by a
    /// [`RwLock`].
    pub fn rigid_body_force_manager(&self) -> &RwLock<RigidBodyForceManager> {
        &self.rigid_body_force_manager
    }

    /// If configured to do so, sets the time step duration to the given frame
    /// duration.
    pub fn update_time_step_duration(&mut self, frame_duration: &Duration) {
        if self.config.match_frame_duration {
            self.set_time_step_duration(frame_duration.as_secs_f64())
        }
    }

    /// Will use the given duration as the time step duration.
    pub fn set_time_step_duration(&mut self, time_step_duration: fph) {
        self.time_step_duration = time_step_duration;
    }

    /// Will use the given stepping scheme for advancing rigid body motion.
    pub fn set_stepping_scheme(&mut self, stepping_scheme: SteppingScheme) {
        self.config.stepping_scheme = stepping_scheme;
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
        self.analytical_motion_manager
            .read()
            .unwrap()
            .apply_analytical_motion(&ecs_world.read().unwrap(), self.simulation_time);

        self.apply_forces_and_torques(ecs_world);
    }

    /// Advances the physics simulation by one time step.
    pub fn advance_simulation(&mut self, ecs_world: &RwLock<ECSWorld>) {
        with_timing_info_logging!(
            "Simulation step ({}) with duration {:.2} ({:.1}x) and {} substeps",
            self.stepping_scheme(),
            self.scaled_time_step_duration(),
            self.simulation_speed_multiplier,
            self.n_substeps(); {

            match self.stepping_scheme() {
                SteppingScheme::EulerCromer => {
                    self.advance_simulation_with_scheme::<EulerCromerStep>(ecs_world);
                }
                SteppingScheme::RK4 => {
                    self.advance_simulation_with_scheme::<RungeKutta4Substep>(ecs_world);
                }
            }
        });

        log::info!("Simulation time: {:.1}", self.simulation_time);
    }

    fn advance_simulation_with_scheme<S: SchemeSubstep>(&mut self, ecs_world: &RwLock<ECSWorld>) {
        let mut entities_to_remove = LinkedList::new();

        let analytical_motion_manager = self.analytical_motion_manager.read().unwrap();
        let rigid_body_force_manager = self.rigid_body_force_manager.read().unwrap();
        let ecs_world_readonly = ecs_world.read().unwrap();

        let substep_duration = self.compute_substep_duration();
        for _ in 0..self.n_substeps() {
            Self::perform_step::<S>(
                &ecs_world_readonly,
                &analytical_motion_manager,
                &rigid_body_force_manager,
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

    fn perform_step<S: SchemeSubstep>(
        ecs_world: &ECSWorld,
        analytical_motion_manager: &AnalyticalMotionManager,
        rigid_body_force_manager: &RigidBodyForceManager,
        current_simulation_time: fph,
        step_duration: fph,
        entities_to_remove: &mut LinkedList<Entity>,
    ) {
        for scheme_substep in S::all_substeps(step_duration) {
            let new_simulation_time = scheme_substep.new_simulation_time(current_simulation_time);

            analytical_motion_manager.apply_analytical_motion(ecs_world, new_simulation_time);

            Self::advance_rigid_body_motion(ecs_world, &scheme_substep);

            rigid_body_force_manager.apply_forces_and_torques(ecs_world, entities_to_remove);
        }
    }

    fn advance_rigid_body_motion<S: SchemeSubstep>(ecs_world: &ECSWorld, scheme_substep: &S) {
        query!(
            ecs_world,
            |rigid_body: &mut RigidBodyComp,
             spatial: &mut SpatialConfigurationComp,
             velocity: &mut VelocityComp,
             angular_velocity: &mut AngularVelocityComp| {
                rigid_body.0.advance_motion(
                    scheme_substep,
                    &mut spatial.position,
                    &mut spatial.orientation,
                    &mut velocity.0,
                    &mut angular_velocity.0,
                );
            },
            ![Static]
        );
    }

    fn apply_forces_and_torques(&self, ecs_world: &RwLock<ECSWorld>) {
        let mut entities_to_remove = LinkedList::new();

        self.rigid_body_force_manager
            .read()
            .unwrap()
            .apply_forces_and_torques(&ecs_world.read().unwrap(), &mut entities_to_remove);

        Self::remove_entities(ecs_world, &entities_to_remove);
    }

    fn remove_entities(ecs_world: &RwLock<ECSWorld>, entities_to_remove: &LinkedList<Entity>) {
        if !entities_to_remove.is_empty() {
            let mut ecs_world_write = ecs_world.write().unwrap();

            for entity in entities_to_remove {
                ecs_world_write.remove_entity(&entity).unwrap();
            }
        }
    }
}

impl Default for SimulatorConfig {
    fn default() -> Self {
        Self {
            stepping_scheme: SteppingScheme::RK4,
            n_substeps: 1,
            initial_time_step_duration: 0.015,
            match_frame_duration: true,
            simulation_speed_multiplier_increment_factor: 1.1,
        }
    }
}
