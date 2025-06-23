//! Management of entities in the engine.

use super::Engine;
use crate::{
    gizmo,
    scene::{SceneEntityFlags, components::SceneEntityFlagsComp},
};
use anyhow::Result;
use impact_ecs::{
    archetype::{ArchetypeComponentStorage, ArchetypeComponents},
    component::{ComponentArray, ComponentCategory, ComponentID, SingleInstance},
    world::EntityID,
};

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

        self.perform_setup_for_new_entities(&mut components)?;

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
        self.perform_setup_for_new_entities(&mut components)?;
        self.ecs_world.write().unwrap().create_entities(components)
    }

    pub fn remove_entity(&self, entity_id: EntityID) -> Result<()> {
        let mut ecs_world = self.ecs_world.write().unwrap();

        let entry = ecs_world.entity(entity_id);

        self.simulator()
            .read()
            .unwrap()
            .perform_cleanup_for_removed_entity(&entry);

        let mut render_resources_desynchronized = false;

        self.scene()
            .read()
            .unwrap()
            .perform_cleanup_for_removed_entity(&entry, &mut render_resources_desynchronized);

        drop(entry);

        if render_resources_desynchronized {
            self.renderer()
                .read()
                .unwrap()
                .declare_render_resources_desynchronized();
        }

        ecs_world.remove_entity(entity_id)
    }

    /// Unsets the [`SceneEntityFlags::IS_DISABLED`] flag for the specified
    /// entity.
    ///
    /// # Errors
    /// Returns an error if the entity does not exist or does not have the
    /// [`SceneEntityFlagsComp`] component.
    pub fn enable_scene_entity(&self, entity_id: EntityID) -> Result<()> {
        self.with_component_mut(entity_id, |flags: &mut SceneEntityFlagsComp| {
            flags.0.remove(SceneEntityFlags::IS_DISABLED);
            Ok(())
        })
    }

    /// Sets the [`SceneEntityFlags::IS_DISABLED`] flag for the specified
    /// entity.
    ///
    /// # Errors
    /// Returns an error if the entity does not exist or does not have the
    /// [`SceneEntityFlagsComp`] component.
    pub fn disable_scene_entity(&self, entity_id: EntityID) -> Result<()> {
        self.with_component_mut(entity_id, |flags: &mut SceneEntityFlagsComp| {
            flags.0.insert(SceneEntityFlags::IS_DISABLED);
            Ok(())
        })
    }

    fn perform_setup_for_new_entities(
        &self,
        components: &mut ArchetypeComponentStorage,
    ) -> Result<()> {
        let mut render_resources_desynchronized = false;

        self.scene().read().unwrap().perform_setup_for_new_entity(
            self.graphics_device(),
            &self.assets().read().unwrap(),
            components,
            &mut render_resources_desynchronized,
        )?;

        self.simulator()
            .read()
            .unwrap()
            .perform_setup_for_new_entity(
                self.scene().read().unwrap().mesh_repository(),
                components,
            )?;

        self.scene().read().unwrap().add_new_entity_to_scene_graph(
            self.renderer(),
            &self.ecs_world,
            components,
            &mut render_resources_desynchronized,
        )?;

        gizmo::entity::setup_gizmos_for_new_entity(
            &self.gizmo_manager().read().unwrap(),
            components,
        );

        if render_resources_desynchronized {
            self.renderer()
                .read()
                .unwrap()
                .declare_render_resources_desynchronized();
        }

        let (setup_component_ids, setup_component_names, standard_component_names) =
            self.extract_component_metadata(components);

        log::info!(
            "Creating {} entities:\nSetup components:\n    {}\nStandard components:\n    {}",
            components.component_count(),
            setup_component_names.join("\n    "),
            standard_component_names.join("\n    "),
        );

        // Remove all setup components
        components.remove_component_types_with_ids(setup_component_ids)?;

        Ok(())
    }

    fn extract_component_metadata(
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
