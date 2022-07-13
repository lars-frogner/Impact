//! Overarching container and coordinator for ECS.

use super::{
    archetype::{ArchetypeCompByteView, ArchetypeCompBytes, ArchetypeID, ArchetypeTable},
    component::{Component, ComponentByteView, ComponentID},
    util::KeyIndexMapper,
};
use anyhow::{anyhow, Result};
use std::hash::Hash;

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
    archetype_tables: Vec<ArchetypeTable>,
    entity_id_counter: EntityID,
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
        }
    }

    /// Creates a new [`Entity`] with a single given component.
    pub fn create_entity_with_component<C>(&mut self, component: &C) -> Entity
    where
        C: Component,
    {
        self.create_entity_with_archetype_data(component.into())
    }

    /// Creates a new [`Entity`] with the given set of components.
    /// The given set of components must be provided as a type
    /// that can be converted to an [`ArchetypeCompByteView`].
    /// Typically, this will be a tuple of references to [`Component`]
    /// instances.
    ///
    /// # Errors
    /// Returns an error if the given set of components do not
    /// have a valid [`Archetype`](super::archetype::Archetype),
    /// which happens if there are multiple component of the same
    /// type.
    ///
    /// # Examples
    /// ```
    /// # use impact::ecs::{
    /// #    archetype::ArchetypeCompByteView,
    /// #    component::Component,
    /// # };
    /// # use bytemuck::{Zeroable, Pod};
    /// # use anyhow::Error;
    /// # use World;
    /// #
    /// # #[repr(C)]
    /// # #[derive(Clone, Copy, Zeroable, Pod)]
    /// # struct Distance(f32);
    /// # #[repr(C)]
    /// # #[derive(Clone, Copy, Zeroable, Pod)]
    /// # struct Speed(f32);
    /// #
    /// let mut world = World::new();
    /// let entity = world.create_entity_with_components((&Distance(0.0), &Speed(10.0)))?;
    /// ```
    pub fn create_entity_with_components<'a>(
        &mut self,
        components: impl TryInto<ArchetypeCompByteView<'a>, Error = anyhow::Error>,
    ) -> Result<Entity> {
        Ok(self.create_entity_with_archetype_data(components.try_into()?))
    }

    /// Removes the given [`Entity`] and all of its components
    /// from the world.
    ///
    /// # Errors
    /// Returns an error if the entity to remove does not exist.
    pub fn remove_entity(&mut self, entity: Entity) -> Result<()> {
        self.remove_entity_data(&entity).map(|_| ())
    }

    /// Adds the given [`Component`] to the given [`Entity`].
    /// This changes the [`Archetype`](super::archetype::Archetype),
    /// if the entity, which is why the entity must be given as a
    /// mutable reference.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The entity does not exist.
    /// - The entity already has a component of the same type.
    pub fn add_component_for_entity<C: Component>(
        &mut self,
        entity: &mut Entity,
        component: &C,
    ) -> Result<()> {
        self.add_component_data_for_entity(entity, component.component_bytes())
    }

    /// Removes the given [`Component`] from the given [`Entity`].
    /// This changes the [`Archetype`](super::archetype::Archetype),
    /// if the entity, which is why the entity must be given as a
    /// mutable reference.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The entity does not exist.
    /// - The entity does not have a component of the specified
    /// component type to remove.
    pub fn remove_component_for_entity<C: Component>(&mut self, entity: &mut Entity) -> Result<()> {
        self.remove_component_id_for_entity(entity, C::component_id())
    }

    /// Returns an iterator over all [`ArchetypeTable`]s whose
    /// entities have at least all the component types defined
    /// by the [`Archetype`](super::archetype::Archetype) of the
    /// given ID.
    pub fn find_tables_containing_archetype(
        &self,
        archetype_id: ArchetypeID,
    ) -> Result<impl Iterator<Item = &ArchetypeTable>> {
        let idx = self.get_table_idx(archetype_id)?;
        Ok(std::iter::once(&self.archetype_tables[idx]))
    }

    fn get_table_idx(&self, id: ArchetypeID) -> Result<usize> {
        self.archetype_index_mapper
            .get(id)
            .ok_or_else(|| anyhow!("Archetype not present"))
    }

    fn create_entity_with_archetype_data(
        &mut self,
        archetype_data: ArchetypeCompByteView,
    ) -> Entity {
        let entity = self.create_entity(archetype_data.id());
        self.add_entity_with_archetype_data(entity, archetype_data);
        entity
    }

    fn add_entity_with_archetype_data(
        &mut self,
        entity: Entity,
        archetype_data: ArchetypeCompByteView,
    ) {
        let archetype_id = archetype_data.id();
        // If archetypes are not consistent we have a bug
        assert_eq!(entity.archetype_id, archetype_id);

        match self.archetype_index_mapper.get(archetype_id) {
            // If we already have a table for the archetype, we add
            // the entity to it
            Some(idx) => self.archetype_tables[idx]
                .add_entity(entity, archetype_data)
                .unwrap(),
            // If we don't have the table, initialize it with the entity
            // as the first entry
            None => {
                self.archetype_index_mapper.push_key(archetype_id);
                self.archetype_tables
                    .push(ArchetypeTable::new_with_entity(entity, archetype_data));
            }
        }
    }

    fn remove_entity_data(&mut self, entity: &Entity) -> Result<ArchetypeCompBytes> {
        let idx = self.get_table_idx(entity.archetype_id)?;
        let table = &mut self.archetype_tables[idx];

        let removed_archetype_data = table.remove_entity(entity)?;

        // If we removed the last entity in the table, there is no
        // reason the keep the table any more
        if table.is_empty() {
            self.remove_archetype_table_at_idx(idx);
        }

        Ok(removed_archetype_data)
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
        updated_archetype_data.add_component_bytes(component_data)?;

        // Set new archetype for the entity
        entity.archetype_id = updated_archetype_data.id();

        // Finally we insert the modified entity into the appropriate table
        self.add_entity_with_archetype_data(*entity, updated_archetype_data);
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
        entity.archetype_id = updated_archetype_data.id();

        // Finally we insert the modified entity into the appropriate table
        self.add_entity_with_archetype_data(*entity, updated_archetype_data);
        Ok(())
    }

    fn create_entity(&mut self, archetype_id: ArchetypeID) -> Entity {
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
    use super::{
        super::query::{IntoComponentQuery, Read, Write},
        *,
    };
    use bytemuck::{Pod, Zeroable};

    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    struct Position {
        pos: [f32; 3],
    }

    #[repr(C)]
    #[derive(Copy, Clone, Debug, Zeroable, Pod)]
    struct Temperature {
        temp: f64,
    }

    #[test]
    fn creating_world_works() {
        let world = World::new();
    }

    #[test]
    fn creating_entity_works() {
        let mut world = World::new();
        let entity = world.create_entity_with_component(&Position {
            pos: [0.0, 1.0, 2.0],
        });
    }

    #[test]
    fn removing_entity_works() {
        let mut world = World::new();
        let entity = world.create_entity_with_component(&Position {
            pos: [0.0, 1.0, 2.0],
        });
        world.remove_entity(entity).unwrap();
    }

    #[test]
    fn adding_component_for_entity_works() {
        let mut world = World::new();
        let mut entity = world.create_entity_with_component(&Position {
            pos: [0.0, 1.0, 2.0],
        });
        world
            .add_component_for_entity(&mut entity, &Temperature { temp: -5.0 })
            .unwrap();
        dbg!(world);
        dbg!(entity);
    }

    #[test]
    fn querying_entity_works() {
        let mut world = World::new();
        let entity = world
            .create_entity_with_components((
                &Position {
                    pos: [0.0, 1.0, 2.0],
                },
                &Temperature { temp: -5.0 },
            ))
            .unwrap();
        let mut query = <(Read<Position>, Write<Temperature>)>::query(&mut world).unwrap();
        for (pos, temp) in query.iter_mut() {
            temp.temp = 42.0 * pos.pos[2] as f64;
        }
        for (pos, temp) in query.iter_mut() {
            dbg!(pos, temp);
        }
    }
}
