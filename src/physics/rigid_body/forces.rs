//! Calculation of forces and torques on rigid bodies.

mod detailed_drag;
mod spring;
mod uniform_gravity;

pub use detailed_drag::{
    DetailedDragComp, DragLoad, DragLoadMap, DragLoadMapComp, DragLoadMapConfig,
    DragLoadMapRepository,
};
pub use spring::{Spring, SpringComp, SpringState};
pub use uniform_gravity::UniformGravityComp;

use crate::{
    component::ComponentRegistry, gpu::rendering::fre, mesh::MeshRepository, physics::UniformMedium,
};
use anyhow::Result;
use impact_ecs::{
    archetype::ArchetypeComponentStorage,
    world::{Entity, EntityEntry, World as ECSWorld},
};
use std::sync::RwLock;

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
    pub fn perform_setup_for_new_entity(
        &self,
        mesh_repository: &RwLock<MeshRepository<fre>>,
        components: &mut ArchetypeComponentStorage,
    ) {
        detailed_drag::setup_drag_load_map_for_new_entity(
            mesh_repository,
            &self.drag_load_map_repository,
            components,
        );
    }

    /// Performs any modifications required to clean up the force manager when
    /// the given entity is removed.
    pub fn perform_cleanup_for_removed_entity(&self, _entity: &EntityEntry<'_>) {}

    /// Applies all forces of torques on entities with rigid bodies.
    pub fn apply_forces_and_torques(
        &self,
        ecs_world: &ECSWorld,
        medium: &UniformMedium,
        entities_to_remove: &mut Vec<Entity>,
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

/// Registers all rigid body force
/// [`Component`](impact_ecs::component::Component)s.
pub fn register_rigid_body_force_components(registry: &mut ComponentRegistry) -> Result<()> {
    register_component!(registry, UniformGravityComp)?;
    register_component!(registry, SpringComp)?;
    register_setup_component!(registry, DetailedDragComp)?;
    register_component!(registry, DragLoadMapComp)
}
