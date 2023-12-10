//! Calculation of forces and torques on rigid bodies.

mod detailed_drag;
mod spring;
mod uniform_gravity;

pub use detailed_drag::{
    DetailedDragComp, DragLoad, DragLoadMap, DragLoadMapConfig, DragLoadMapRepository,
};
pub use spring::{Spring, SpringComp};
pub use uniform_gravity::UniformGravityComp;

use crate::{physics::UniformMedium, rendering::fre, scene::MeshRepository};
use anyhow::Result;
use impact_ecs::{
    archetype::ArchetypeComponentStorage,
    world::{Entity, EntityEntry, World as ECSWorld},
};
use std::{collections::LinkedList, sync::RwLock};

/// Manager of all systems resulting in forces and torques on rigid bodies.
#[derive(Debug)]
pub struct RigidBodyForceManager {
    drag_load_map_repository: RwLock<DragLoadMapRepository<fre>>,
}

/// Configuration parameters for rigid body force generation.
#[derive(Clone, Debug, Default)]
pub struct RigidBodyForceConfig {
    /// Configuration parameters for the generation of drag load maps. If
    /// [`None`], default parameters are used.
    pub drag_load_map_config: Option<DragLoadMapConfig>,
}

impl RigidBodyForceManager {
    /// Creates a new force manager with the given configuration parameters for
    /// the generation of drag load maps.
    ///
    /// # Errors
    /// Returns an error if any of the configuration parameters are invalid.
    pub fn new(mut config: RigidBodyForceConfig) -> Result<Self> {
        let drag_load_map_config = config.drag_load_map_config.take().unwrap_or_default();

        Ok(Self {
            drag_load_map_repository: RwLock::new(DragLoadMapRepository::new(
                drag_load_map_config,
            )?),
        })
    }

    /// Checks if the entity-to-be with the given components has the components
    /// for being affected by specific forces, and if so, performs any required
    /// setup and adds any required auxiliary components to the entity.
    pub fn add_force_components_for_entity(
        &self,
        mesh_repository: &RwLock<MeshRepository<fre>>,
        components: &mut ArchetypeComponentStorage,
    ) {
        DetailedDragComp::add_drag_load_map_component_for_entity(
            mesh_repository,
            &self.drag_load_map_repository,
            components,
        );
    }

    /// Performs any modifications required to clean up the force manager when
    /// the given entity is removed.
    pub fn handle_entity_removed(&self, _entity: &EntityEntry<'_>) {}

    /// Applies all forces of torques on entities with rigid bodies.
    pub fn apply_forces_and_torques(
        &self,
        ecs_world: &ECSWorld,
        medium: &UniformMedium,
        entities_to_remove: &mut LinkedList<Entity>,
    ) {
        uniform_gravity::apply_uniform_gravity(ecs_world);

        spring::apply_spring_forces(ecs_world, entities_to_remove);

        detailed_drag::apply_detailed_drag(
            ecs_world,
            &self.drag_load_map_repository.read().unwrap(),
            medium,
        );
    }

    /// Performs actions that should be performed after completion of a
    /// simulation step.
    pub fn perform_post_simulation_step_actions(&self, ecs_world: &ECSWorld) {
        spring::synchronize_spring_positions_and_orientations(ecs_world);
    }
}
