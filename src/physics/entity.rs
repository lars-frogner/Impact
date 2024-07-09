//! Management of scene data for entities.

use super::rigid_body;
use crate::{gpu::rendering::fre, mesh::MeshRepository, physics::PhysicsSimulator};
use impact_ecs::{archetype::ArchetypeComponentStorage, world::EntityEntry};
use std::sync::RwLock;

impl PhysicsSimulator {
    /// Performs any modifications to the physics simulator required to
    /// accommodate a new entity with the given components, and adds any
    /// additional components to the entity's components.
    pub fn perform_setup_for_new_entity(
        &self,
        mesh_repository: &RwLock<MeshRepository<fre>>,
        components: &mut ArchetypeComponentStorage,
    ) {
        rigid_body::entity::setup_rigid_body_for_new_entity(mesh_repository, components);

        self.rigid_body_force_manager
            .read()
            .unwrap()
            .perform_setup_for_new_entity(mesh_repository, components);
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
