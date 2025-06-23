//! Management of scene data for entities.

use super::{collision, rigid_body};
use crate::physics::PhysicsSimulator;
use anyhow::Result;
use impact_ecs::{archetype::ArchetypeComponentStorage, world::EntityEntry};
use impact_mesh::MeshRepository;
use std::sync::RwLock;

impl PhysicsSimulator {
    /// Performs any modifications to the physics simulator required to
    /// accommodate a new entity with the given components, and adds any
    /// additional components to the entity's components.
    pub fn perform_setup_for_new_entity(
        &self,
        mesh_repository: &RwLock<MeshRepository>,
        components: &mut ArchetypeComponentStorage,
    ) -> Result<()> {
        rigid_body::entity::setup_rigid_body_for_new_entity(mesh_repository, components)?;

        self.rigid_body_force_manager
            .read()
            .unwrap()
            .perform_setup_for_new_entity(mesh_repository, components)?;

        collision::entity::setup_collidable_for_new_entity(&self.collision_world, components);

        Ok(())
    }

    /// Performs any modifications required to clean up the physics simulator
    /// when the given entity is removed.
    pub fn perform_cleanup_for_removed_entity(&self, entity: &EntityEntry<'_>) {
        self.rigid_body_force_manager
            .read()
            .unwrap()
            .perform_cleanup_for_removed_entity(entity);
    }
}
