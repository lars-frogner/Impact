//! Calculation of forces and torques on rigid bodies.

pub mod detailed_drag;
pub mod entity;
pub mod spring;
pub mod uniform_gravity;

use crate::physics::{
    UniformMedium, motion::components::Static, rigid_body::components::RigidBodyComp,
};
use anyhow::Result;
use detailed_drag::{DragLoadMapConfig, DragLoadMapRepository};
use impact_ecs::{
    query,
    world::{EntityID, World as ECSWorld},
};
use impact_scene::components::SceneEntityFlagsComp;
use serde::{Deserialize, Serialize};
use std::sync::RwLock;

/// Manager of all systems resulting in forces and torques on rigid bodies.
#[derive(Debug)]
pub struct RigidBodyForceManager {
    drag_load_map_repository: RwLock<DragLoadMapRepository<f32>>,
}

/// Configuration parameters for rigid body force generation.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct RigidBodyForceConfig {
    /// Configuration parameters for the generation of drag load maps.
    pub drag_load_map_config: DragLoadMapConfig,
}

impl RigidBodyForceManager {
    /// Creates a new force manager with the given configuration parameters for
    /// the generation of drag load maps.
    ///
    /// # Errors
    /// Returns an error if any of the configuration parameters are invalid.
    pub fn new(config: RigidBodyForceConfig) -> Result<Self> {
        Ok(Self {
            drag_load_map_repository: RwLock::new(DragLoadMapRepository::new(
                config.drag_load_map_config,
            )?),
        })
    }

    /// Applies all forces of torques on entities with rigid bodies.
    pub fn apply_forces_and_torques(
        &self,
        ecs_world: &ECSWorld,
        medium: &UniformMedium,
        entities_to_remove: &mut Vec<EntityID>,
    ) {
        reset_forces_and_torques(ecs_world);

        uniform_gravity::systems::apply_uniform_gravity(ecs_world);

        spring::systems::apply_spring_forces(ecs_world, entities_to_remove);

        detailed_drag::systems::apply_detailed_drag(
            ecs_world,
            &self.drag_load_map_repository.read().unwrap(),
            medium,
        );
    }

    /// Performs actions that should be performed after completion of a
    /// simulation step.
    pub fn perform_post_simulation_step_actions(&self, ecs_world: &ECSWorld) {
        spring::systems::synchronize_spring_positions_and_orientations(ecs_world);
    }
}

fn reset_forces_and_torques(ecs_world: &ECSWorld) {
    query!(
        ecs_world,
        |rigid_body: &mut RigidBodyComp, flags: &SceneEntityFlagsComp| {
            if flags.is_disabled() {
                return;
            }
            rigid_body.0.reset_force_and_torque();
        },
        ![Static]
    );
}
