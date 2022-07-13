use super::{
    archetype::{ArchetypeCompByteView, ArchetypeCompBytes, ArchetypeID, ArchetypeTable},
    component::{Component, ComponentByteView, ComponentID},
    util::KeyIndexMapper,
};
use anyhow::{anyhow, Result};
use std::hash::Hash;

#[derive(Clone, Copy, Debug, PartialEq, Hash)]
pub struct Entity {
    id: EntityID,
    archetype_id: ArchetypeID,
}

pub type EntityID = u64;

#[derive(Debug)]
pub struct World {
    archetype_index_mapper: KeyIndexMapper<ArchetypeID>,
    archetype_tables: Vec<ArchetypeTable>,
    entity_id_counter: EntityID,
}

impl Entity {
    pub fn id(&self) -> EntityID {
        self.id
    }

    pub fn archetype_id(&self) -> ArchetypeID {
        self.archetype_id
    }
}

impl World {
    pub fn new() -> Self {
        Self {
            archetype_index_mapper: KeyIndexMapper::new(),
            archetype_tables: Vec::new(),
            entity_id_counter: 0,
        }
    }

    pub fn create_entity_with_component<C>(&mut self, component: &C) -> Entity
    where
        C: Component,
    {
        self.create_entity_with_archetype_data(component.into())
    }

    pub fn create_entity_with_components<'a>(
        &mut self,
        archetype_data: impl TryInto<ArchetypeCompByteView<'a>, Error = anyhow::Error>,
    ) -> Result<Entity> {
        Ok(self.create_entity_with_archetype_data(archetype_data.try_into()?))
    }

    pub fn remove_entity(&mut self, entity: Entity) -> Result<()> {
        self.remove_entity_data(&entity).map(|_| ())
    }

    pub fn add_component_for_entity<C: Component>(
        &mut self,
        entity: &mut Entity,
        component: &C,
    ) -> Result<()> {
        self.add_component_data_for_entity(entity, component.component_bytes())
    }

    pub fn remove_component_for_entity<C: Component>(&mut self, entity: &mut Entity) -> Result<()> {
        self.remove_component_id_for_entity(entity, C::component_id())
    }

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
        assert_eq!(entity.archetype_id, archetype_id);
        match self.archetype_index_mapper.get(archetype_id) {
            Some(idx) => self.archetype_tables[idx]
                .add_entity(entity, archetype_data)
                .unwrap(),
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
        let existing_archetype_data = self.remove_entity_data(entity)?;
        let mut updated_archetype_data = existing_archetype_data.as_ref();
        updated_archetype_data.add_component_bytes(component_data)?;

        entity.archetype_id = updated_archetype_data.id();

        self.add_entity_with_archetype_data(*entity, updated_archetype_data);
        Ok(())
    }

    fn remove_component_id_for_entity(
        &mut self,
        entity: &mut Entity,
        component_id: ComponentID,
    ) -> Result<()> {
        let existing_archetype_data = self.remove_entity_data(entity)?;
        let mut updated_archetype_data = existing_archetype_data.as_ref();
        updated_archetype_data.remove_component_with_id(component_id)?;

        entity.archetype_id = updated_archetype_data.id();

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
