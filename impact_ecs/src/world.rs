//! Overarching container and coordinator for ECS.

use super::{
    archetype::{
        Archetype, ArchetypeCompByteView, ArchetypeCompBytes, ArchetypeID, ArchetypeTable,
    },
    component::{Component, ComponentByteView, ComponentID},
    util::KeyIndexMapper,
};
use anyhow::{anyhow, Result};
use std::{
    hash::Hash,
    sync::{RwLock, RwLockReadGuard},
};

/// Handle to an entity in the world.
///
/// An entity typically refers to an instance of some
/// type of object that has certain specific [`Component`]s
/// that define its properties. An entity can be created
/// using a [`World`].
#[derive(Clone, Copy, Debug, PartialEq, Hash)]
pub struct Entity {
    id: EntityID,
    archetype_id: ArchetypeID,
}

/// Unique ID identifying an [`Entity`].
pub type EntityID = u64;

/// Overall manager for [`Entity`]s in the world and
/// their [`Component`] data.
#[derive(Debug)]
pub struct World {
    /// A map from [`ArchetypeID`] to the index of the corresponding
    /// [`ArchetypeTable`] in the `archetype_tables` vector.
    archetype_index_mapper: KeyIndexMapper<ArchetypeID>,
    archetype_tables: Vec<RwLock<ArchetypeTable>>,
    entity_id_counter: EntityID,
    n_removed_entities: usize,
}

impl Entity {
    /// Returns the ID that uniquely identifies this entity.
    pub fn id(&self) -> EntityID {
        self.id
    }

    /// Returns the ID that uniquely identifies the
    /// [`Archetype`](super::archetype::Archetype)
    /// this entity belongs to.
    pub fn archetype_id(&self) -> ArchetypeID {
        self.archetype_id
    }
}

impl World {
    /// Creates a new world with no entities.
    pub fn new() -> Self {
        Self {
            archetype_index_mapper: KeyIndexMapper::new(),
            archetype_tables: Vec::new(),
            entity_id_counter: 0,
            n_removed_entities: 0,
        }
    }

    /// Creates a new [`Entity`] with the given set of components.
    /// The given set of components must be provided as a type
    /// that can be converted to an [`ArchetypeCompByteView`].
    /// Typically, this will be a tuple of references to [`Component`]
    /// instances.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The given set of components does not have a valid
    /// [`Archetype`], which happens if there are multiple
    /// components of the same type.
    /// - The more than one instance of each component type is
    /// provided (use [`World::create_entities`] for that).
    ///
    /// # Examples
    /// ```
    /// # use impact_ecs::{
    /// #    archetype::ArchetypeCompByteView,
    /// #    world::World,
    /// # };
    /// # use impact_ecs_derive::ComponentDoctest;
    /// # use bytemuck::{Zeroable, Pod};
    /// # use anyhow::Error;
    /// #
    /// # #[repr(C)]
    /// # #[derive(Clone, Copy, Zeroable, Pod, ComponentDoctest)]
    /// # struct Distance(f32);
    /// # #[repr(C)]
    /// # #[derive(Clone, Copy, Zeroable, Pod, ComponentDoctest)]
    /// # struct Speed(f32);
    /// #
    /// let mut world = World::new();
    /// let entity_1 = world.create_entity(&Distance(5.0))?;
    /// let entity_2 = world.create_entity((&Distance(0.0), &Speed(10.0)))?;
    /// #
    /// # Ok::<(), Error>(())
    /// ```
    pub fn create_entity<'a, E>(
        &mut self,
        components: impl TryInto<ArchetypeCompByteView<'a>, Error = E>,
    ) -> Result<Entity>
    where
        E: Into<anyhow::Error>,
    {
        self.create_entities(components).and_then(|mut entities| {
            if entities.len() == 1 {
                Ok(entities.pop().unwrap())
            } else {
                Err(anyhow!("Got components for more than one entity"))
            }
        })
    }

    /// Creates multiple new entities with the given set of components.
    /// The given set of components must be provided as a type
    /// that can be converted to an [`ArchetypeCompByteView`].
    /// Typically, this will be a tuple of slices with [`Component`]
    /// instances.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The given set of components does not have a valid
    /// [`Archetype`], which happens if there are multiple
    /// components of the same type.
    /// - If the number of component instances provided for each
    /// component type is not the same.
    ///
    /// # Examples
    /// ```
    /// # use impact_ecs::{
    /// #    archetype::ArchetypeCompByteView,
    /// #    world::World,
    /// # };
    /// # use impact_ecs_derive::ComponentDoctest;
    /// # use bytemuck::{Zeroable, Pod};
    /// # use anyhow::Error;
    /// #
    /// # #[repr(C)]
    /// # #[derive(Clone, Copy, Zeroable, Pod, ComponentDoctest)]
    /// # struct Distance(f32);
    /// # #[repr(C)]
    /// # #[derive(Clone, Copy, Zeroable, Pod, ComponentDoctest)]
    /// # struct Speed(f32);
    /// #
    /// let mut world = World::new();
    /// let entities = world.create_entities(
    ///     (
    ///         &[Distance(0.0),Distance(1.0), Distance(2.0)],
    ///         &[Speed(2.0), Speed(1.0), Speed(0.0)]
    ///     )
    /// )?;
    /// assert_eq!(entities.len(), 3);
    /// #
    /// # Ok::<(), Error>(())
    /// ```
    pub fn create_entities<'a, E>(
        &mut self,
        components: impl TryInto<ArchetypeCompByteView<'a>, Error = E>,
    ) -> Result<Vec<Entity>>
    where
        E: Into<anyhow::Error>,
    {
        Ok(self.create_entities_with_component_bytes(components.try_into().map_err(E::into)?))
    }

    /// Returns the current number of entities in the world.
    pub fn entity_count(&self) -> usize {
        (self.entity_id_counter as usize) - self.n_removed_entities
    }

    /// Removes the given [`Entity`] and all of its components
    /// from the world.
    ///
    /// # Errors
    /// Returns an error if the entity to remove does not exist.
    pub fn remove_entity(&mut self, entity: &Entity) -> Result<()> {
        self.remove_entity_data(entity).map(|_| ())?;
        self.n_removed_entities += 1;
        Ok(())
    }

    /// Adds the given [`Component`] to the given [`Entity`].
    /// This changes the [`Archetype`], if the entity, which
    /// is why the entity must be given as a
    /// mutable reference.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The entity does not exist.
    /// - The entity already has a components of the same type.
    pub fn add_component_for_entity(
        &mut self,
        entity: &mut Entity,
        component: &impl Component,
    ) -> Result<()> {
        self.add_component_data_for_entity(entity, component.component_bytes())
    }

    /// Removes the given [`Component`] from the given [`Entity`].
    /// This changes the [`Archetype`], if the entity, which is why
    /// the entity must be given as a mutable reference.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The entity does not exist.
    /// - The entity does not have a components of the specified
    /// component type to remove.
    pub fn remove_component_for_entity<C: Component>(&mut self, entity: &mut Entity) -> Result<()> {
        self.remove_component_id_for_entity(entity, C::component_id())
    }

    /// Returns an iterator over all [`ArchetypeTable`]s whose
    /// entities have at least all the component types defined
    /// by the given [`Archetype`].
    pub fn find_tables_containing_archetype(
        &self,
        archetype: Archetype,
    ) -> impl Iterator<Item = RwLockReadGuard<ArchetypeTable>> {
        self.archetype_tables.iter().filter_map(move |table| {
            let table = table.read().unwrap();
            if table.archetype().contains(&archetype) {
                Some(table)
            } else {
                None
            }
        })
    }

    fn get_table_idx(&self, id: ArchetypeID) -> Result<usize> {
        self.archetype_index_mapper
            .get(id)
            .ok_or_else(|| anyhow!("Archetype not present"))
    }

    fn create_entities_with_component_bytes(
        &mut self,
        archetype_data: ArchetypeCompByteView,
    ) -> Vec<Entity> {
        let archetype_id = archetype_data.archetype_id();
        let entities: Vec<_> = (0..archetype_data.component_count())
            .map(|_| self.create_next_entity(archetype_id))
            .collect();
        self.add_entities_to_table(entities.iter().map(Entity::id), archetype_data);
        entities
    }

    fn add_entities_to_table(
        &mut self,
        entity_ids: impl IntoIterator<Item = EntityID>,
        archetype_data: ArchetypeCompByteView,
    ) {
        let archetype_id = archetype_data.archetype_id();
        match self.archetype_index_mapper.get(archetype_id) {
            // If we already have a table for the archetype, we add
            // the entity to it
            Some(idx) => self.archetype_tables[idx]
                .write()
                .unwrap()
                .add_entities(entity_ids, archetype_data),
            // If we don't have the table, initialize it with the entity
            // as the first entry
            None => {
                self.archetype_index_mapper.push_key(archetype_id);
                self.archetype_tables
                    .push(RwLock::new(ArchetypeTable::new_with_entities(
                        entity_ids,
                        archetype_data,
                    )));
            }
        }
    }

    fn remove_entity_data(&mut self, entity: &Entity) -> Result<ArchetypeCompBytes> {
        let idx = self.get_table_idx(entity.archetype_id)?;
        let mut table = self.archetype_tables[idx].write().unwrap();

        let removed_component_data = table.remove_entity(entity.id())?;

        // If we removed the last entity in the table, there is no
        // reason the keep the table any more
        if table.is_empty() {
            drop(table);
            self.remove_archetype_table_at_idx(idx);
        }

        Ok(removed_component_data)
    }

    fn remove_archetype_table_at_idx(&mut self, idx: usize) {
        self.archetype_index_mapper.swap_remove_key_at_idx(idx);
        self.archetype_tables.swap_remove(idx);
    }

    fn add_component_data_for_entity(
        &mut self,
        entity: &mut Entity,
        component_data: ComponentByteView,
    ) -> Result<()> {
        // Since the archetype of the entity changes when adding a
        // component, we need to first remove it from the old table
        let existing_archetype_data = self.remove_entity_data(entity)?;

        // We then add the component to the entity's data
        let mut updated_archetype_data = existing_archetype_data.as_ref();
        updated_archetype_data.add_new_component(component_data)?;

        // Set new archetype for the entity
        entity.archetype_id = updated_archetype_data.archetype_id();

        // Finally we insert the modified entity into the appropriate table
        self.add_entities_to_table([entity.id()], updated_archetype_data);
        Ok(())
    }

    fn remove_component_id_for_entity(
        &mut self,
        entity: &mut Entity,
        component_id: ComponentID,
    ) -> Result<()> {
        // Since the archetype of the entity changes when removing a
        // component, we need to first remove it from the old table
        let existing_archetype_data = self.remove_entity_data(entity)?;

        // We then remove the component from the entity's data
        let mut updated_archetype_data = existing_archetype_data.as_ref();
        updated_archetype_data.remove_component_with_id(component_id)?;

        // Set new archetype for the entity
        entity.archetype_id = updated_archetype_data.archetype_id();

        // Finally we insert the modified entity into the appropriate table
        self.add_entities_to_table([entity.id()], updated_archetype_data);
        Ok(())
    }

    fn create_next_entity(&mut self, archetype_id: ArchetypeID) -> Entity {
        let id = self.create_entity_id();
        Entity { id, archetype_id }
    }

    fn create_entity_id(&mut self) -> EntityID {
        let id = self.entity_id_counter;
        self.entity_id_counter += 1;
        id
    }
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod test {
    use super::{super::Component, *};
    use bytemuck::{Pod, Zeroable};

    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
    struct Position(f32, f32, f32);

    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod, Component)]
    struct Temperature(f32);

    const POS: Position = Position(2.5, 3.1, 42.0);
    const TEMP: Temperature = Temperature(-40.0);

    #[test]
    fn creating_world_works() {
        let world = World::new();
        assert_eq!(world.entity_count(), 0);
    }

    #[test]
    fn creating_single_entity_works() {
        let mut world = World::new();

        world.create_entity(&POS).unwrap();
        assert_eq!(world.entity_count(), 1);

        world.create_entity((&POS, &TEMP)).unwrap();
        assert_eq!(world.entity_count(), 2);

        // TODO: Check components
    }

    #[test]
    fn creating_multiple_entities_works() {
        todo!()
    }

    #[test]
    fn removing_entity_works() {
        let mut world = World::new();
        let entity = world.create_entity(&POS).unwrap();
        assert_eq!(world.entity_count(), 1);

        world.remove_entity(&entity).unwrap();
        assert_eq!(world.entity_count(), 0);
    }

    #[test]
    fn adding_component_for_entity_works() {
        let mut world = World::new();
        let mut entity = world.create_entity(&POS).unwrap();
        world.add_component_for_entity(&mut entity, &TEMP).unwrap();
        // TODO: Check components
    }

    #[test]
    fn removing_component_for_entity_works() {
        let mut world = World::new();
        let mut entity = world.create_entity((&POS, &TEMP)).unwrap();
        world
            .remove_component_for_entity::<Position>(&mut entity)
            .unwrap();
        // TODO: Check components
    }
}
