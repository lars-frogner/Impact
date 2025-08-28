//! Management of entities in the engine.

use super::Engine;
use crate::lock_order::OrderedMutex;
use anyhow::Result;
use impact_ecs::{
    archetype::ArchetypeComponentStorage,
    component::{ComponentCategory, ComponentID},
    world::{EntitiesToCreate, EntityID, EntityToCreate, EntityToCreateWithID},
};
use impact_scene::SceneEntityFlags;
use tinyvec::TinyVec;

type ComponentMetadataList<T> = TinyVec<[T; 16]>;

impl Engine {
    /// Creates entities staged for creation and removes entities staged for
    /// removal.
    pub(crate) fn handle_staged_entities(&self) -> Result<()> {
        let mut entity_stager = self.entity_stager.olock();

        for EntityToCreateWithID {
            entity_id,
            components,
        } in entity_stager.drain_entities_to_create_with_id()
        {
            self.create_entity_with_id(entity_id, components)?;
        }

        for EntityToCreate { components } in entity_stager.drain_single_entities_to_create() {
            self.create_entity(components)?;
        }

        for EntitiesToCreate { components } in entity_stager.drain_multi_entities_to_create() {
            self.create_entities(components)?;
        }

        for entity_id in entity_stager.drain_entities_to_remove() {
            self.remove_entity(entity_id)?;
        }

        Ok(())
    }

    /// Unsets the [`SceneEntityFlags::IS_DISABLED`] flag for the specified
    /// entity.
    ///
    /// # Errors
    /// Returns an error if the entity does not exist or does not have the
    /// [`SceneEntityFlags`] component.
    pub(crate) fn enable_scene_entity(&self, entity_id: EntityID) -> Result<()> {
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
    pub(crate) fn disable_scene_entity(&self, entity_id: EntityID) -> Result<()> {
        self.with_component_mut(entity_id, |flags: &mut SceneEntityFlags| {
            flags.insert(SceneEntityFlags::IS_DISABLED);
            Ok(())
        })
    }

    pub(crate) fn extract_component_metadata(
        &self,
        components: &ArchetypeComponentStorage,
    ) -> (
        ComponentMetadataList<ComponentID>,
        ComponentMetadataList<&'static str>,
        ComponentMetadataList<&'static str>,
    ) {
        let mut setup_component_ids = TinyVec::with_capacity(components.n_component_types());
        let mut setup_component_names = TinyVec::with_capacity(components.n_component_types());
        let mut standard_component_names = TinyVec::with_capacity(components.n_component_types());

        for component_id in components.component_ids() {
            let component_metadata = self.component_metadata_registry.metadata(component_id);
            match component_metadata.category {
                ComponentCategory::Standard => {
                    standard_component_names.push(component_metadata.name);
                }
                ComponentCategory::Setup => {
                    setup_component_ids.push(component_id);
                    setup_component_names.push(component_metadata.name);
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
