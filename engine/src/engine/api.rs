//! Public engine API.

use super::Engine;
use crate::{
    command::EngineCommand,
    lock_order::{OrderedMutex, OrderedRwLock},
    setup,
};
use anyhow::Result;
use impact_ecs::{
    archetype::ArchetypeComponents,
    component::{ComponentArray, SingleInstance},
    world::EntityID,
};
use std::sync::atomic::Ordering;

impl Engine {
    pub fn stage_entity_for_creation_with_id<A, E>(
        &self,
        entity_id: EntityID,
        components: impl TryInto<SingleInstance<ArchetypeComponents<A>>, Error = E>,
    ) -> Result<()>
    where
        A: ComponentArray,
        E: Into<anyhow::Error>,
    {
        self.entity_stager
            .olock()
            .stage_entity_for_creation_with_id(entity_id, components)
    }

    pub fn stage_entity_for_creation<A, E>(
        &self,
        components: impl TryInto<SingleInstance<ArchetypeComponents<A>>, Error = E>,
    ) -> Result<()>
    where
        A: ComponentArray,
        E: Into<anyhow::Error>,
    {
        self.entity_stager
            .olock()
            .stage_entity_for_creation(components)
    }

    pub fn stage_entities_for_creation<A, E>(
        &self,
        components: impl TryInto<ArchetypeComponents<A>, Error = E>,
    ) -> Result<()>
    where
        A: ComponentArray,
        E: Into<anyhow::Error>,
    {
        self.entity_stager
            .olock()
            .stage_entities_for_creation(components)
    }

    pub fn stage_entity_for_removal(&self, entity_id: EntityID) {
        self.entity_stager
            .olock()
            .stage_entity_for_removal(entity_id);
    }

    pub fn create_entity_with_id<A, E>(
        &self,
        entity_id: EntityID,
        components: impl TryInto<SingleInstance<ArchetypeComponents<A>>, Error = E>,
    ) -> Result<()>
    where
        A: ComponentArray,
        E: Into<anyhow::Error>,
    {
        let mut components = components
            .try_into()
            .map_err(E::into)?
            .into_inner()
            .into_storage();

        setup::perform_setup_for_new_entities(self, &mut components)?;

        self.ecs_world
            .owrite()
            .create_entity_with_id(entity_id, SingleInstance::new(components))
    }

    pub fn create_entity<A, E>(
        &self,
        components: impl TryInto<SingleInstance<ArchetypeComponents<A>>, Error = E>,
    ) -> Result<EntityID>
    where
        A: ComponentArray,
        E: Into<anyhow::Error>,
    {
        Ok(self
            .create_entities(components.try_into().map_err(E::into)?.into_inner())?
            .pop()
            .unwrap())
    }

    pub fn create_entities<A, E>(
        &self,
        components: impl TryInto<ArchetypeComponents<A>, Error = E>,
    ) -> Result<Vec<EntityID>>
    where
        A: ComponentArray,
        E: Into<anyhow::Error>,
    {
        let mut components = components.try_into().map_err(E::into)?.into_storage();
        setup::perform_setup_for_new_entities(self, &mut components)?;
        self.ecs_world.owrite().create_entities(components)
    }

    pub fn remove_entity(&self, entity_id: EntityID) -> Result<()> {
        let mut ecs_world = self.ecs_world.owrite();
        setup::perform_cleanup_for_removed_entity(self, &ecs_world.entity(entity_id))?;
        ecs_world.remove_entity(entity_id)
    }

    pub fn enqueue_command(&self, command: EngineCommand) {
        self.command_queue.enqueue_command(command);
    }

    pub fn controls_enabled(&self) -> bool {
        self.controls_enabled.load(Ordering::Relaxed)
    }
}
