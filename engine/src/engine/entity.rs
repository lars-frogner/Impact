//! Management of entities in the engine.

use crate::setup;

use super::Engine;
use anyhow::Result;
use impact_ecs::{
    archetype::{ArchetypeComponentStorage, ArchetypeComponents},
    component::{ComponentArray, ComponentCategory, ComponentID, SingleInstance},
    world::{EntityID, EntityToCreate, EntityToCreateWithID},
};
use impact_scene::SceneEntityFlags;

impl Engine {
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
            .write()
            .unwrap()
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
        self.ecs_world.write().unwrap().create_entities(components)
    }

    pub fn remove_entity(&self, entity_id: EntityID) -> Result<()> {
        let mut ecs_world = self.ecs_world.write().unwrap();
        setup::perform_cleanup_for_removed_entity(self, &ecs_world.entity(entity_id))?;
        ecs_world.remove_entity(entity_id)
    }

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
            .lock()
            .unwrap()
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
            .lock()
            .unwrap()
            .stage_entity_for_creation(components)
    }

    pub fn stage_entity_for_removal(&self, entity_id: EntityID) {
        self.entity_stager
            .lock()
            .unwrap()
            .stage_entity_for_removal(entity_id);
    }

    pub fn create_staged_entities(&self) -> Result<()> {
        let (entities_to_create, entities_to_create_with_id) =
            self.entity_stager.lock().unwrap().take_entities_to_create();

        for EntityToCreate { components } in entities_to_create {
            self.create_entity(components)?;
        }

        for EntityToCreateWithID {
            entity_id,
            components,
        } in entities_to_create_with_id
        {
            self.create_entity_with_id(entity_id, components)?;
        }

        Ok(())
    }

    pub fn remove_staged_entities(&self) -> Result<()> {
        let entities_to_remove = self.entity_stager.lock().unwrap().take_entities_to_remove();

        for entity_id in entities_to_remove {
            self.remove_entity(entity_id)?;
        }

        Ok(())
    }

    pub fn handle_staged_entities(&self) -> Result<()> {
        self.remove_staged_entities()?;
        self.create_staged_entities()
    }

    /// Unsets the [`SceneEntityFlags::IS_DISABLED`] flag for the specified
    /// entity.
    ///
    /// # Errors
    /// Returns an error if the entity does not exist or does not have the
    /// [`SceneEntityFlags`] component.
    pub fn enable_scene_entity(&self, entity_id: EntityID) -> Result<()> {
        self.with_component_mut(entity_id, |flags: &mut SceneEntityFlags| {
            flags.remove(SceneEntityFlags::IS_DISABLED);
            Ok(())
        })
    }

    /// Sets the [`SceneEntityFlags::IS_DISABLED`] flag for the specified
    /// entity.
    ///
    /// # Errors
    /// Returns an error if the entity does not exist or does not have the
    /// [`SceneEntityFlags`] component.
    pub fn disable_scene_entity(&self, entity_id: EntityID) -> Result<()> {
        self.with_component_mut(entity_id, |flags: &mut SceneEntityFlags| {
            flags.insert(SceneEntityFlags::IS_DISABLED);
            Ok(())
        })
    }

    pub fn extract_component_metadata(
        &self,
        components: &ArchetypeComponentStorage,
    ) -> (Vec<ComponentID>, Vec<&'static str>, Vec<&'static str>) {
        let mut setup_component_ids = Vec::with_capacity(components.n_component_types());
        let mut setup_component_names = Vec::with_capacity(components.n_component_types());
        let mut standard_component_names = Vec::with_capacity(components.n_component_types());

        let component_registry = self.component_registry.read().unwrap();

        for component_id in components.component_ids() {
            let entry = component_registry.component_with_id(component_id);
            match entry.category {
                ComponentCategory::Standard => {
                    standard_component_names.push(entry.name);
                }
                ComponentCategory::Setup => {
                    setup_component_ids.push(component_id);
                    setup_component_names.push(entry.name);
                }
            }
        }

        (
            setup_component_ids,
            setup_component_names,
            standard_component_names,
        )
    }
}
