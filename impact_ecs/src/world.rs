//! Overarching container and coordinator for ECS.

use super::{
    archetype::{
        Archetype, ArchetypeCompByteView, ArchetypeCompBytes, ArchetypeID, ArchetypeTable,
        ComponentStorageEntry, ComponentStorageEntryMut,
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
#[cfg(not(test))]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EntityID(u64);

#[cfg(test)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EntityID(pub u64);

/// Overall manager for [`Entity`]s in the world and
/// their [`Component`] data.
#[derive(Debug)]
pub struct World {
    /// A map from [`ArchetypeID`] to the index of the corresponding
    /// [`ArchetypeTable`] in the `archetype_tables` vector.
    archetype_index_mapper: KeyIndexMapper<ArchetypeID>,
    archetype_tables: Vec<RwLock<ArchetypeTable>>,
    entity_id_counter: u64,
    n_removed_entities: usize,
}

/// A reference into the entry for an [`Entity`] in the [`World`].
#[derive(Debug)]
pub struct EntityEntry<'a> {
    entity_id: EntityID,
    table: RwLockReadGuard<'a, ArchetypeTable>,
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
    /// - More than one instance of each component type is
    /// provided (use [`World::create_entities`] for that).
    ///
    /// # Examples
    /// ```
    /// # use impact_ecs::{
    /// #    archetype::ArchetypeCompByteView,
    /// #    world::World,
    /// # };
    /// # use impact_ecs_macros::ComponentDoctest;
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
    ///
    /// let entity_1 = world.create_entity(&Distance(5.0))?;
    /// let entity_2 = world.create_entity((&Distance(0.0), &Speed(10.0)))?;
    ///
    /// assert_eq!(world.entity_count(), 2);
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
    /// # use impact_ecs_macros::ComponentDoctest;
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
    /// assert_eq!(world.entity_count(), 3);
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

    /// Returns an [`EntityEntry`] that can be used to access the components
    /// of the given [`Entity`]. If the entity does not exist, [`None`] is
    /// returned.
    ///
    /// # Examples
    /// See [`World::entity`].
    ///
    /// # Concurrency
    /// The returned `EntityEntry` holds a read lock on the
    /// [`ArchetypeTable`] holding the entity. Until the entry is
    /// dropped, attempts to modify the table will be blocked.
    pub fn get_entity(&self, entity: &Entity) -> Option<EntityEntry<'_>> {
        let table_idx = self.get_table_idx(entity.archetype_id()).ok()?;
        let table = self.archetype_tables[table_idx].read().unwrap();
        Some(EntityEntry::new(entity.id(), table))
    }

    /// Returns an [`EntityEntry`] that can be used to access the
    /// components of the given [`Entity`].
    ///
    /// # Panics
    /// If the entity does not exist.
    ///
    /// # Examples
    /// ```
    /// # use impact_ecs::{
    /// #    archetype::ArchetypeCompByteView,
    /// #    world::World,
    /// # };
    /// # use impact_ecs_macros::ComponentDoctest;
    /// # use bytemuck::{Zeroable, Pod};
    /// # use anyhow::Error;
    /// #
    /// # #[repr(C)]
    /// # #[derive(Clone, Copy, Debug, PartialEq, Zeroable, Pod, ComponentDoctest)]
    /// # struct Level(u32);
    /// #
    /// let mut world = World::new();
    ///
    /// let entity = world.create_entity(&Level(1))?;
    /// let entry = world.entity(&entity);
    ///
    /// assert_eq!(entry.n_components(), 1);
    /// assert_eq!(entry.component::<Level>().access(), &Level(1));
    ///
    /// *entry.component_mut::<Level>().access() = Level(11);
    ///
    /// assert_eq!(entry.component::<Level>().access(), &Level(11));
    /// #
    /// # Ok::<(), Error>(())
    /// ```
    ///
    /// # Concurrency
    /// The returned `EntityEntry` holds a read lock on the
    /// [`ArchetypeTable`] holding the entity. Until the entry is
    /// dropped, attempts to modify the table will be blocked.
    pub fn entity(&self, entity: &Entity) -> EntityEntry<'_> {
        self.get_entity(entity).expect("Entity does not exist")
    }

    /// Adds the given [`Component`] to the given [`Entity`].
    /// This changes the [`Archetype`] of the entity, which
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
    /// This changes the [`Archetype`] of the entity, which is why
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
    ) -> impl Iterator<Item = RwLockReadGuard<'_, ArchetypeTable>> {
        self.archetype_tables.iter().filter_map(move |table| {
            let table = table.read().unwrap();
            if table.archetype().contains(&archetype) {
                Some(table)
            } else {
                None
            }
        })
    }

    /// Returns an iterator over all [`ArchetypeTable`]s whose
    /// entities have at least all the component types defined
    /// by the given [`Archetype`], but not any of the given
    /// disallowed component types.
    pub fn find_tables_containing_archetype_except_disallowed<const N: usize>(
        &self,
        archetype: Archetype,
        disallowed_component_ids: [ComponentID; N],
    ) -> impl Iterator<Item = RwLockReadGuard<'_, ArchetypeTable>> {
        self.archetype_tables.iter().filter_map(move |table| {
            let table = table.read().unwrap();
            let table_archetype = table.archetype();
            if table_archetype.contains(&archetype)
                && table_archetype.contains_none_of(&disallowed_component_ids)
            {
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
        archetype_data: ArchetypeCompByteView<'_>,
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
        archetype_data: ArchetypeCompByteView<'_>,
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
        let idx = self.get_table_idx(entity.archetype_id())?;
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
        component_data: ComponentByteView<'_>,
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
        EntityID(id)
    }
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> EntityEntry<'a> {
    fn new(entity_id: EntityID, table: RwLockReadGuard<'a, ArchetypeTable>) -> Self {
        Self { entity_id, table }
    }

    /// Returns the number of components the entity has.
    pub fn n_components(&self) -> usize {
        self.table.archetype().n_components()
    }

    /// Whether the entity has the component specified by the
    /// type parameter `C`.
    pub fn has_component<C: Component>(&self) -> bool {
        self.table
            .archetype()
            .contains_component_id(C::component_id())
    }

    /// Returns a reference to the component specified by the
    /// type parameter `C`. If the entity does not have this
    /// component, [`None`] is returned.
    pub fn get_component<C: Component>(&self) -> Option<ComponentStorageEntry<'_, C>> {
        self.table.get_component_for_entity::<C>(self.entity_id)
    }

    /// Returns a reference to the component specified by the
    /// type parameter `C`.
    ///
    /// # Panics
    /// If the entity does not have the specified component.
    pub fn component<C: Component>(&self) -> ComponentStorageEntry<'_, C> {
        self.get_component::<C>()
            .expect("Requested invalid component")
    }

    /// Returns a mutable reference to the component specified
    /// by the type parameter `C`. If the entity does not have
    /// this component, [`None`] is returned.
    pub fn get_component_mut<C: Component>(&self) -> Option<ComponentStorageEntryMut<'_, C>> {
        self.table.get_component_for_entity_mut::<C>(self.entity_id)
    }

    /// Returns a mutable reference to the component specified
    /// by the type parameter `C`.
    ///
    /// # Panics
    /// If the entity does not have the specified component.
    pub fn component_mut<C: Component>(&self) -> ComponentStorageEntryMut<'_, C> {
        self.get_component_mut::<C>()
            .expect("Requested invalid component")
    }
}

#[cfg(test)]
mod test {
    use super::{super::Component, *};
    use bytemuck::{Pod, Zeroable};

    #[repr(C)]
    #[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod, Component)]
    struct Position(f32, f32, f32);

    #[repr(C)]
    #[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod, Component)]
    struct Temperature(f32);

    const POS: Position = Position(2.5, 3.1, 42.0);
    const POS2: Position = Position(5.2, 1.3, 0.42);
    const TEMP: Temperature = Temperature(-40.0);
    const TEMP2: Temperature = Temperature(140.0);

    #[test]
    fn creating_world_works() {
        let world = World::new();
        assert_eq!(world.entity_count(), 0);
    }

    #[test]
    fn creating_single_entity_works() {
        let mut world = World::new();

        let entity_1 = world.create_entity(&POS).unwrap();
        assert_eq!(world.entity_count(), 1);
        let entry = world.entity(&entity_1);
        assert_eq!(entry.n_components(), 1);
        assert_eq!(entry.component::<Position>().access(), &POS);
        drop(entry);

        let entity_2 = world.create_entity((&POS, &TEMP)).unwrap();
        assert_eq!(world.entity_count(), 2);
        let entry = world.entity(&entity_2);
        assert_eq!(entry.n_components(), 2);
        assert_eq!(entry.component::<Position>().access(), &POS);
        assert_eq!(entry.component::<Temperature>().access(), &TEMP);
    }

    #[test]
    fn creating_two_entities_works() {
        let mut world = World::new();
        let entities = world.create_entities(&[TEMP, TEMP2]).unwrap();

        assert_eq!(entities.len(), 2);
        assert_eq!(world.entity_count(), 2);

        let entry = world.entity(&entities[0]);
        assert_eq!(entry.n_components(), 1);
        assert_eq!(entry.component::<Temperature>().access(), &TEMP);
        drop(entry);

        let entry = world.entity(&entities[1]);
        assert_eq!(entry.n_components(), 1);
        assert_eq!(entry.component::<Temperature>().access(), &TEMP2);
        drop(entry);

        let entities = world
            .create_entities((&[POS, POS2], &[TEMP, TEMP2]))
            .unwrap();
        assert_eq!(entities.len(), 2);
        assert_eq!(world.entity_count(), 4);

        let entry = world.entity(&entities[0]);
        assert_eq!(entry.n_components(), 2);
        assert_eq!(entry.component::<Position>().access(), &POS);
        assert_eq!(entry.component::<Temperature>().access(), &TEMP);
        drop(entry);

        let entry = world.entity(&entities[1]);
        assert_eq!(entry.n_components(), 2);
        assert_eq!(entry.component::<Position>().access(), &POS2);
        assert_eq!(entry.component::<Temperature>().access(), &TEMP2);
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
    #[should_panic]
    fn removing_same_entity_twice_fails() {
        let mut world = World::new();
        let entity = world.create_entity(&POS).unwrap();
        world.remove_entity(&entity).unwrap();
        world.remove_entity(&entity).unwrap();
    }

    #[test]
    fn adding_component_for_entity_works() {
        let mut world = World::new();
        let mut entity = world.create_entity(&POS).unwrap();
        world.add_component_for_entity(&mut entity, &TEMP).unwrap();

        assert_eq!(
            entity.archetype_id(),
            Archetype::new_from_component_id_arr([
                Position::component_id(),
                Temperature::component_id()
            ])
            .unwrap()
            .id()
        );

        let entry = world.entity(&entity);
        assert_eq!(entry.n_components(), 2);
        assert_eq!(entry.component::<Position>().access(), &POS);
        assert_eq!(entry.component::<Temperature>().access(), &TEMP);
    }

    #[test]
    #[should_panic]
    fn adding_existing_component_for_entity_fails() {
        let mut world = World::new();
        let mut entity = world.create_entity(&POS).unwrap();
        world.add_component_for_entity(&mut entity, &POS).unwrap();
    }

    #[test]
    fn removing_component_for_entity_works() {
        let mut world = World::new();
        let mut entity = world.create_entity((&POS, &TEMP)).unwrap();
        assert!(world.entity(&entity).has_component::<Position>());
        world
            .remove_component_for_entity::<Position>(&mut entity)
            .unwrap();

        assert_eq!(
            entity.archetype_id(),
            Archetype::new_from_component_id_arr([Temperature::component_id()])
                .unwrap()
                .id()
        );

        let entry = world.entity(&entity);
        assert_eq!(entry.n_components(), 1);
        assert_eq!(entry.component::<Temperature>().access(), &TEMP);
        assert!(!entry.has_component::<Position>());
    }

    #[test]
    fn removing_all_components_for_entity_works() {
        let mut world = World::new();
        let mut entity = world.create_entity((&POS, &TEMP)).unwrap();
        world
            .remove_component_for_entity::<Position>(&mut entity)
            .unwrap();
        world
            .remove_component_for_entity::<Temperature>(&mut entity)
            .unwrap();
        let entry = world.entity(&entity);
        assert_eq!(entry.n_components(), 0);
        assert_eq!(
            entity.archetype_id(),
            Archetype::new_from_component_id_arr([]).unwrap().id()
        );
    }

    #[test]
    #[should_panic]
    fn removing_absent_component_from_entity_fails() {
        let mut world = World::new();
        let mut entity = world.create_entity(&POS).unwrap();
        world
            .remove_component_for_entity::<Temperature>(&mut entity)
            .unwrap();
    }

    #[test]
    fn modifying_component_for_entity_works() {
        let mut world = World::new();
        let entity = world.create_entity((&POS, &TEMP)).unwrap();
        let entry = world.entity(&entity);
        *entry.component_mut::<Temperature>().access() = TEMP2;
        assert_eq!(entry.component::<Position>().access(), &POS);
        assert_eq!(entry.component::<Temperature>().access(), &TEMP2);
    }
}

#[cfg(test)]
mod query_test {
    use super::{
        super::{query, Component},
        *,
    };
    use bytemuck::{Pod, Zeroable};

    #[repr(C)]
    #[derive(Clone, Copy, Debug, PartialEq, Zeroable, Pod, Component)]
    struct Byte(u8);

    #[repr(C)]
    #[derive(Clone, Copy, Debug, PartialEq, Zeroable, Pod, Component)]
    struct Position(f32, f32, f32);

    #[repr(C)]
    #[derive(Clone, Copy, Debug, PartialEq, Zeroable, Pod, Component)]
    struct Rectangle {
        center: [f32; 2],
        dimensions: [f32; 2],
    }

    const BYTE: Byte = Byte(7);
    const BYTE2: Byte = Byte(55);
    const POS: Position = Position(1.5, -7.7, 0.1);
    const POS2: Position = Position(0.0, 1e-5, 0.001);
    const RECT: Rectangle = Rectangle {
        center: [2.5, 2.0],
        dimensions: [12.3, 8.9],
    };

    /// These query invocations should all compile successfully.
    #[allow(dead_code)]
    fn test_valid_query_inputs() {
        #![allow(clippy::unnecessary_mut_passed)]

        let mut world = World::new();

        query!(world, |_byte: &Byte| {});

        query!(&world, |_byte: &mut Byte| {});

        query!(world, |_pos: &mut Position, _byte: &Byte| {});

        query!(&mut world, |_byte: &mut Byte, _pos: &mut Position| {});

        query!(world, |_byte: &Byte,
                       _rect: &mut Rectangle,
                       _pos: &Position| {});

        query!(world, |_byte: &mut Byte,
                       _pos: &mut Position,
                       _rect: &mut Rectangle| {});
    }

    #[test]
    fn single_entity_single_comp_read_works() {
        let mut world = World::new();
        world.create_entity(&BYTE).unwrap();

        let mut count = 0;
        query!(world, |byte: &Byte| {
            assert_eq!(byte, &BYTE);
            count += 1;
        });
        assert_eq!(count, 1);
    }

    #[test]
    fn single_entity_two_of_two_matching_comp_read_works() {
        let mut world = World::new();
        world.create_entity((&BYTE, &POS)).unwrap();

        let mut count = 0;
        query!(world, |byte: &Byte, pos: &Position| {
            assert_eq!(byte, &BYTE);
            assert_eq!(pos, &POS);
            count += 1;
        });
        assert_eq!(count, 1);
    }

    #[test]
    fn single_entity_one_of_two_matching_comp_read_works() {
        let mut world = World::new();
        world.create_entity((&BYTE, &POS)).unwrap();

        let mut count = 0;
        query!(world, |pos: &Position| {
            assert_eq!(pos, &POS);
            count += 1;
        });
        assert_eq!(count, 1);
    }

    #[test]
    fn single_entity_single_comp_write_works() {
        let mut world = World::new();
        world.create_entity(&BYTE).unwrap();

        query!(world, |byte: &mut Byte| {
            assert_eq!(byte, &BYTE);
            *byte = BYTE2;
        });
        query!(world, |byte: &Byte| {
            assert_eq!(byte, &BYTE2);
        });
    }

    #[test]
    fn single_entity_two_of_two_matching_comp_write_works() {
        let mut world = World::new();
        world.create_entity((&BYTE, &POS)).unwrap();

        query!(world, |pos: &mut Position, byte: &mut Byte| {
            assert_eq!(pos, &POS);
            assert_eq!(byte, &BYTE);
            *pos = POS2;
            *byte = BYTE2;
        });
        query!(world, |byte: &Byte, pos: &Position| {
            assert_eq!(pos, &POS2);
            assert_eq!(byte, &BYTE2);
        });
    }

    #[test]
    fn single_entity_two_of_two_matching_comp_mixed_write_works() {
        let mut world = World::new();
        world.create_entity((&BYTE, &POS)).unwrap();

        query!(world, |pos: &mut Position, byte: &Byte| {
            assert_eq!(pos, &POS);
            assert_eq!(byte, &BYTE);
            pos.1 = f32::from(byte.0);
        });
        query!(world, |byte: &Byte, pos: &Position| {
            assert_eq!(pos.1, f32::from(byte.0));
        });
    }

    #[test]
    fn two_equal_entities_single_comp_read_works() {
        let mut world = World::new();
        world.create_entities(&[BYTE, BYTE]).unwrap();

        let mut count = 0;
        query!(world, |byte: &Byte| {
            assert_eq!(byte, &BYTE);
            count += 1;
        });
        assert_eq!(count, 2);
    }

    #[test]
    fn one_of_two_matching_single_comp_entities_works() {
        let mut world = World::new();
        world.create_entity(&BYTE).unwrap();
        world.create_entity(&POS).unwrap();

        let mut count = 0;
        query!(world, |pos: &Position| {
            assert_eq!(pos, &POS);
            count += 1;
        });
        assert_eq!(count, 1);
    }

    #[test]
    fn ono_of_two_matching_two_comp_entities_works() {
        let mut world = World::new();
        world.create_entity((&BYTE, &RECT)).unwrap();
        world.create_entity((&POS, &BYTE)).unwrap();

        let mut count = 0;
        query!(world, |rect: &Rectangle, byte: &Byte| {
            assert_eq!(rect, &RECT);
            assert_eq!(byte, &BYTE);
            count += 1;
        });
        assert_eq!(count, 1);
    }

    #[test]
    fn two_of_two_partially_matching_two_comp_entities_works() {
        let mut world = World::new();
        world.create_entity((&BYTE, &RECT)).unwrap();
        world.create_entity((&POS, &BYTE)).unwrap();

        let mut count = 0;
        query!(world, |byte: &Byte| {
            assert_eq!(byte, &BYTE);
            count += 1;
        });
        assert_eq!(count, 2);
    }

    #[test]
    fn zero_of_two_matching_two_comp_entities_works() {
        let mut world = World::new();
        world.create_entity((&BYTE, &RECT)).unwrap();
        world.create_entity((&POS, &BYTE)).unwrap();

        let mut count = 0;
        query!(world, |_pos: &Position, _rect: &Rectangle| {
            count += 1;
        });
        assert_eq!(count, 0);
    }
}
