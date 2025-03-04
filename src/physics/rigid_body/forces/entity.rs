//! Management of force data for entities.

use super::{RigidBodyForceManager, detailed_drag};
use crate::mesh::MeshRepository;
use impact_ecs::{archetype::ArchetypeComponentStorage, world::EntityEntry};
use std::sync::RwLock;

impl RigidBodyForceManager {
    /// Checks if the entity-to-be with the given components has the components
    /// for being affected by specific forces, and if so, performs any required
    /// setup and adds any required auxiliary components to the entity.
    pub fn perform_setup_for_new_entity(
        &self,
        mesh_repository: &RwLock<MeshRepository<f32>>,
        components: &mut ArchetypeComponentStorage,
    ) {
        detailed_drag::entity::setup_drag_load_map_for_new_entity(
            mesh_repository,
            &self.drag_load_map_repository,
            components,
        );
    }

    /// Performs any modifications required to clean up the force manager when
    /// the given entity is removed.
    pub fn perform_cleanup_for_removed_entity(&self, _entity: &EntityEntry<'_>) {}
}
