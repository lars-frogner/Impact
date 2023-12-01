//! Calculation of forces and torques on rigid bodies.

mod spring;
mod uniform_gravity;

pub use spring::{Spring, SpringComp};
pub use uniform_gravity::UniformGravityComp;

use impact_ecs::world::{Entity, World as ECSWorld};
use std::collections::LinkedList;

/// Manager of all systems resulting in forces and torques on rigid bodies.
#[derive(Debug)]
pub struct RigidBodyForceManager;

impl RigidBodyForceManager {
    /// Creates a new force manager.
    pub fn new() -> Self {
        Self
    }

    /// Applies all forces of torques on entities with rigid bodies.
    pub fn apply_forces_and_torques(
        &self,
        ecs_world: &ECSWorld,
        entities_to_remove: &mut LinkedList<Entity>,
    ) {
        uniform_gravity::apply_uniform_gravity(ecs_world);
        spring::apply_spring_forces(ecs_world, entities_to_remove);
    }
}
