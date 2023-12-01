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
    advance_orientation, AdvanceOrientations, AdvancePositions, AngularMomentum, AngularVelocity,
    AngularVelocityComp, Direction, DrivenAngularVelocityComp, Force, Momentum, Orientation,
    OrientationComp, Position, PositionComp, Static, Torque, Velocity, VelocityComp,
};
pub use rigid_body::{
    RigidBody, RigidBodyComp, RigidBodyForceManager, Spring, SpringComp, UniformGravityComp,
    UniformRigidBodyComp,
};
pub use tasks::{AdvanceSimulation, PhysicsTag};

use impact_ecs::{query, world::World as ECSWorld};
use std::{collections::LinkedList, sync::RwLock};

/// Floating point type used for physics simulation.
#[allow(non_camel_case_types)]
pub type fph = f64;

#[derive(Debug)]
pub struct PhysicsSimulator {
    config: SimulatorConfig,
    rigid_body_force_manager: RwLock<RigidBodyForceManager>,
}

#[derive(Clone, Debug)]
pub struct SimulatorConfig {
    time_step_duration: fph,
}

impl PhysicsSimulator {
    pub fn new(config: SimulatorConfig) -> Self {
        Self {
            config,
            rigid_body_force_manager: RwLock::new(RigidBodyForceManager::new()),
        }
    }

    pub fn time_step_duration(&self) -> fph {
        self.config.time_step_duration
    }

    /// Returns a reference to the [`RigidBodyForceManager`], guarded by a
    /// [`RwLock`].
    pub fn rigid_body_force_manager(&self) -> &RwLock<RigidBodyForceManager> {
        &self.rigid_body_force_manager
    }

    /// Advances the physics simulation by one time step.
    pub fn advance_simulation(&self, ecs_world: &RwLock<ECSWorld>) {
        let mut entities_to_remove = LinkedList::new();

        let rigid_body_force_manager = self.rigid_body_force_manager.read().unwrap();
        let ecs_world_readonly = ecs_world.read().unwrap();

        rigid_body_force_manager
            .apply_forces_and_torques(&ecs_world_readonly, &mut entities_to_remove);

        Self::advance_rigid_body_motion(&ecs_world_readonly, self.time_step_duration());

        rigid_body_force_manager.perform_post_simulation_step_actions(&ecs_world_readonly);

        if !entities_to_remove.is_empty() {
            drop(ecs_world_readonly);
            let mut ecs_world_write = ecs_world.write().unwrap();

            for entity in entities_to_remove {
                ecs_world_write.remove_entity(&entity).unwrap();
            }
        }
    }

    fn advance_rigid_body_motion(ecs_world: &ECSWorld, duration: fph) {
        query!(
            ecs_world,
            |position: &mut PositionComp,
             orientation: &mut OrientationComp,
             velocity: &mut VelocityComp,
             angular_velocity: &mut AngularVelocityComp,
             rigid_body: &mut RigidBodyComp| {
                rigid_body.0.advance_motion(
                    &mut position.0,
                    &mut orientation.0,
                    &mut velocity.0,
                    &mut angular_velocity.0,
                    duration,
                );
            }
        );
    }
}

impl Default for SimulatorConfig {
    fn default() -> Self {
        Self {
            time_step_duration: 1.0,
        }
    }
}
