//! Overarching container and coordinator for ECS.

use super::{
    archetype::{
        Archetype, ArchetypeComponentStorage, ArchetypeComponents, ArchetypeID, ArchetypeTable,
        ComponentStorageEntry, ComponentStorageEntryMut, SingleInstanceArchetypeComponentStorage,
    },
    component::{Component, ComponentArray, ComponentID, ComponentStorage, SingleInstance},
};
use anyhow::{Result, anyhow, bail};
use bytemuck::{Pod, Zeroable};
use impact_containers::{NoHashKeyIndexMapper, NoHashMap};
use parking_lot::{RwLock, RwLockReadGuard};
use std::{
    collections::hash_map::Entry,
    fmt,
    hash::{self, Hash},
    vec::Drain,
};

/// Unique ID identifying an entity in the world.
///
/// An entity typically refers to an instance of some type of object that has
/// certain specific [`Component`]s that define its properties. An entity can be
/// created using a [`World`].
#[cfg(not(test))]
#[roc_integration::roc(
    category = "primitive",
    package = "pf",
    module = "Entity",
    name = "Id",
    postfix = "_id"
)]
#[repr(C)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Zeroable, Pod)]
pub struct EntityID(u64);

#[cfg(test)]
#[repr(C)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Zeroable, Pod)]
pub struct EntityID(pub(crate) u64);

/// Overall manager for entities in the world and their [`Component`] data.
#[derive(Debug)]
pub struct World {
    entity_archetypes: NoHashMap<EntityID, ArchetypeID>,
    /// A map from [`ArchetypeID`] to the index of the corresponding
    /// [`ArchetypeTable`] in the `archetype_tables` vector.
    archetype_table_indices_by_id: NoHashKeyIndexMapper<ArchetypeID>,
    archetype_tables: Vec<RwLock<ArchetypeTable>>,
    rng: fastrand::Rng,
}

/// A reference into the entry for an entity in the [`World`].
#[derive(Debug)]
pub struct EntityEntry<'a> {
    entity_id: EntityID,
    table: RwLockReadGuard<'a, ArchetypeTable>,
}

/// Helper for staging entities for later creation or removal.
#[derive(Debug)]
pub struct EntityStager {
    to_create_with_id: Vec<EntityToCreateWithID>,
    to_create: Vec<EntityToCreate>,
    to_create_multiple: Vec<EntitiesToCreate>,
    to_remove: Vec<EntityID>,
}

/// The components of an entity that has yet to be created under a specific ID.
#[derive(Debug)]
pub struct EntityToCreateWithID {
    pub entity_id: EntityID,
    pub components: SingleInstance<ArchetypeComponentStorage>,
}

/// The components of an entity that has yet to be created.
#[derive(Debug)]
pub struct EntityToCreate {
    pub components: SingleInstance<ArchetypeComponentStorage>,
}

/// The components of one or more entities of the same archetype that have yet
/// to be created.
#[derive(Debug)]
pub struct EntitiesToCreate {
    pub components: ArchetypeComponentStorage,
}

impl EntityID {
    /// Hashes the given string into an entity ID.
    pub const fn hashed_from_str(input: &str) -> Self {
        Self(const_fnv1a_hash::fnv1a_hash_str_64(input))
    }

    /// Converts the given `u64` into an entity ID. Should only be called
    /// with values returned from [`Self::as_u64`].
    pub const fn from_u64(value: u64) -> Self {
        Self(value)
    }

    /// Returns the `u64` value corresponding to the entity ID.
    pub const fn as_u64(&self) -> u64 {
        self.0
    }
}

impl Hash for EntityID {
    fn hash<H: hash::Hasher>(&self, hasher: &mut H) {
        hasher.write_u64(self.0);
    }
}

impl nohash_hasher::IsEnabled for EntityID {}

impl fmt::Display for EntityID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_u64())
    }
}

impl World {
    /// Creates a new world with no entities. The specified seed is used for
    /// generating random entity IDs.
    pub fn new(seed: u64) -> Self {
        Self {
            entity_archetypes: NoHashMap::default(),
            archetype_table_indices_by_id: NoHashKeyIndexMapper::default(),
            archetype_tables: Vec::new(),
            rng: fastrand::Rng::with_seed(seed),
        }
    }

    /// Creates a new entity with the given set of components and assigns it the
    /// given ID. The set of components must be provided as a type that can be
    /// converted to an [`ArchetypeComponents`] object wrapped in a
    /// [`SingleInstance`]. Typically, this will be a tuple of references to
    /// [`Component`] instances, which can be converted into a `SingleInstance`
    /// wrapped
    /// [`ArchetypeComponentView`](crate::archetype::ArchetypeComponentView).
    ///
    /// # Errors
    /// Returns an error if:
    /// - An entity with the specified ID already exists.
    /// - The given set of components does not have a valid [`Archetype`], which
    ///   happens if there are multiple components of the same type.
    ///
    /// # Examples
    /// ```
    /// # use impact_ecs::{
    /// #    world::{EntityID, World},
    /// # };
    /// # use impact_ecs_macros::ComponentDoctest as Component;
    /// # use bytemuck::{Zeroable, Pod};
    /// # use anyhow::Error;
    /// #
    /// # #[repr(C)]
    /// # #[derive(Clone, Copy, Zeroable, Pod, Component)]
    /// # struct Distance(f32);
    /// # #[repr(C)]
    /// # #[derive(Clone, Copy, Zeroable, Pod, Component)]
    /// # struct Speed(f32);
    /// #
    /// let mut world = World::default();
    ///
    /// world.create_entity_with_id(
    ///     EntityID::hashed_from_str("a"), &Distance(5.0),
    /// )?;
    /// world.create_entity_with_id(
    ///     EntityID::hashed_from_str("b"), (&Distance(0.0), &Speed(10.0)),
    /// )?;
    ///
    /// assert_eq!(world.entity_count(), 2);
    /// #
    /// # Ok::<(), Error>(())
    /// ```
    pub fn create_entity_with_id<A, E>(
        &mut self,
        entity_id: EntityID,
        components: impl TryInto<SingleInstance<ArchetypeComponents<A>>, Error = E>,
    ) -> Result<()>
    where
        A: ComponentArray,
        E: Into<anyhow::Error>,
    {
        let components: ArchetypeComponents<A> =
            components.try_into().map_err(E::into)?.into_inner();

        self.try_register_archetype_for_new_entities(entity_id, components.archetype().id())?;

        self.add_entities_to_table([entity_id], components);

        Ok(())
    }

    /// Creates a new entity with the given set of components. The set of
    /// components must be provided as a type that can be converted to an
    /// [`ArchetypeComponents`] object wrapped in a [`SingleInstance`].
    /// Typically, this will be a tuple of references to [`Component`]
    /// instances, which can be converted into a `SingleInstance` wrapped
    /// [`ArchetypeComponentView`](crate::archetype::ArchetypeComponentView).
    ///
    /// # Returns
    /// The randomly generated ID of the new entity.
    ///
    /// # Errors
    /// Returns an error if the given set of components does not have a valid
    /// [`Archetype`], which happens if there are multiple components of the
    /// same type.
    ///
    /// # Examples
    /// ```
    /// # use impact_ecs::{
    /// #    world::World,
    /// # };
    /// # use impact_ecs_macros::ComponentDoctest as Component;
    /// # use bytemuck::{Zeroable, Pod};
    /// # use anyhow::Error;
    /// #
    /// # #[repr(C)]
    /// # #[derive(Clone, Copy, Zeroable, Pod, Component)]
    /// # struct Distance(f32);
    /// # #[repr(C)]
    /// # #[derive(Clone, Copy, Zeroable, Pod, Component)]
    /// # struct Speed(f32);
    /// #
    /// let mut world = World::default();
    ///
    /// let entity_1_id = world.create_entity(&Distance(5.0))?;
    /// let entity_2_id = world.create_entity((&Distance(0.0), &Speed(10.0)))?;
    ///
    /// assert_eq!(world.entity_count(), 2);
    /// #
    /// # Ok::<(), Error>(())
    /// ```
    pub fn create_entity<A, E>(
        &mut self,
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

    /// Creates multiple new entities with the given set of components. The set
    /// of components must be provided as a type that can be converted to an
    /// [`ArchetypeComponents`] object. Typically, this will be a tuple of
    /// slices with [`Component`] instances, which can be converted into an
    /// [`ArchetypeComponentView`](crate::archetype::ArchetypeComponentView).
    ///
    /// # Returns
    /// The randomly generated IDs of the new entities.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The given set of components does not have a valid [`Archetype`], which
    ///   happens if there are multiple components of the same type.
    /// - If the number of component instances provided for each component type
    ///   is not the same.
    ///
    /// # Examples
    /// ```
    /// # use impact_ecs::{
    /// #    world::World,
    /// # };
    /// # use impact_ecs_macros::ComponentDoctest as Component;
    /// # use bytemuck::{Zeroable, Pod};
    /// # use anyhow::Error;
    /// #
    /// # #[repr(C)]
    /// # #[derive(Clone, Copy, Zeroable, Pod, Component)]
    /// # struct Distance(f32);
    /// # #[repr(C)]
    /// # #[derive(Clone, Copy, Zeroable, Pod, Component)]
    /// # struct Speed(f32);
    /// #
    /// let mut world = World::default();
    /// let entity_ids = world.create_entities(
    ///     (
    ///         &[Distance(0.0),Distance(1.0), Distance(2.0)],
    ///         &[Speed(2.0), Speed(1.0), Speed(0.0)]
    ///     )
    /// )?;
    /// assert_eq!(entity_ids.len(), 3);
    /// assert_eq!(world.entity_count(), 3);
    /// #
    /// # Ok::<(), Error>(())
    /// ```
    pub fn create_entities<A, E>(
        &mut self,
        components: impl TryInto<ArchetypeComponents<A>, Error = E>,
    ) -> Result<Vec<EntityID>>
    where
        A: ComponentArray,
        E: Into<anyhow::Error>,
    {
        Ok(self.create_entities_with_archetype_components(components.try_into().map_err(E::into)?))
    }

    /// Returns the current number of entities in the world.
    pub fn entity_count(&self) -> usize {
        self.entity_archetypes.len()
    }

    /// Removes the specified entity and all of its components from the world.
    ///
    /// # Errors
    /// Returns an error if the entity to remove does not exist.
    pub fn remove_entity(&mut self, entity_id: EntityID) -> Result<()> {
        self.remove_entity_data(entity_id).map(|_| ())?;
        self.entity_archetypes.remove(&entity_id);
        Ok(())
    }

    /// Removes all entities and their components from the world.
    pub fn remove_all_entities(&mut self) {
        self.entity_archetypes.clear();
        for table in &self.archetype_tables {
            table.write().remove_all_entities();
        }
        self.archetype_tables.clear();
        self.archetype_table_indices_by_id.clear();
    }

    /// Returns an [`EntityEntry`] that can be used to access the components of
    /// the specified entity. If the entity does not exist, [`None`] is
    /// returned.
    ///
    /// # Examples
    /// See [`World::entity`].
    ///
    /// # Concurrency
    /// The returned `EntityEntry` holds a read lock on the
    /// [`ArchetypeTable`] holding the entity. Until the entry is
    /// dropped, attempts to modify the table will be blocked.
    pub fn get_entity(&self, entity_id: EntityID) -> Option<EntityEntry<'_>> {
        let archetype_id = *self.entity_archetypes.get(&entity_id)?;
        let table_idx = self.archetype_table_indices_by_id.get(archetype_id)?;
        let table = self.archetype_tables[table_idx].read();
        Some(EntityEntry::new(entity_id, table))
    }

    /// Returns an [`EntityEntry`] that can be used to access the components of
    /// the specified entity.
    ///
    /// # Panics
    /// If the entity does not exist.
    ///
    /// # Examples
    /// ```
    /// # use impact_ecs::{
    /// #    world::World,
    /// # };
    /// # use impact_ecs_macros::ComponentDoctest as Component;
    /// # use bytemuck::{Zeroable, Pod};
    /// # use anyhow::Error;
    /// #
    /// # #[repr(C)]
    /// # #[derive(Clone, Copy, Debug, PartialEq, Zeroable, Pod, Component)]
    /// # struct Level(u32);
    /// #
    /// let mut world = World::default();
    ///
    /// let entity_id = world.create_entity(&Level(1))?;
    /// let entry = world.entity(entity_id);
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
    pub fn entity(&self, entity_id: EntityID) -> EntityEntry<'_> {
        self.get_entity(entity_id).expect("Entity does not exist")
    }

    /// Adds the given [`Component`] to the specified entity. This changes the
    /// [`Archetype`] of the entity.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The entity does not exist.
    /// - The entity already has a components of the same type.
    pub fn add_component_for_entity(
        &mut self,
        entity_id: EntityID,
        component: &impl Component,
    ) -> Result<()> {
        self.add_component_storage_for_entity(entity_id, component.into_storage())
    }

    /// Removes the given [`Component`] from the specified entity. This changes
    /// the [`Archetype`] of the entity.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The entity does not exist.
    /// - The entity does not have a components of the specified component type
    ///   to remove.
    pub fn remove_component_for_entity<C: Component>(&mut self, entity_id: EntityID) -> Result<()> {
        self.remove_component_id_for_entity(entity_id, C::component_id())
    }

    /// Returns an iterator over all [`ArchetypeTable`]s whose
    /// entities have at least all the component types defined
    /// by the given [`Archetype`].
    pub fn find_tables_containing_archetype(
        &self,
        archetype: Archetype,
    ) -> impl Iterator<Item = RwLockReadGuard<'_, ArchetypeTable>> {
        self.archetype_tables.iter().filter_map(move |table| {
            let table = table.read();
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
            let table = table.read();
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

    /// Generates a new [`EntityID`].
    pub fn create_entity_id(&mut self) -> EntityID {
        let mut id = self.rng.u64(..);
        while self.entity_archetypes.contains_key(&EntityID(id)) {
            id = self.rng.u64(..);
        }
        EntityID(id)
    }

    fn get_entity_archetype(&self, entity_id: EntityID) -> Result<ArchetypeID> {
        self.entity_archetypes
            .get(&entity_id)
            .copied()
            .ok_or_else(|| anyhow!("Entity with ID {entity_id} not present"))
    }

    fn get_table_idx(&self, archetype_id: ArchetypeID) -> Result<usize> {
        self.archetype_table_indices_by_id
            .get(archetype_id)
            .ok_or_else(|| anyhow!("Archetype with ID {} not present", archetype_id.as_u32()))
    }

    fn create_entities_with_archetype_components(
        &mut self,
        components: ArchetypeComponents<impl ComponentArray>,
    ) -> Vec<EntityID> {
        let entity_ids: Vec<_> = (0..components.component_count())
            .map(|_| self.create_entity_id())
            .collect();

        self.register_archetype_for_entities(&entity_ids, components.archetype().id());
        self.add_entities_to_table(entity_ids.iter().copied(), components);

        entity_ids
    }

    fn register_archetype_for_entities(
        &mut self,
        entity_ids: &[EntityID],
        archetype_id: ArchetypeID,
    ) {
        self.entity_archetypes.reserve(entity_ids.len());
        for entity_id in entity_ids {
            self.entity_archetypes.insert(*entity_id, archetype_id);
        }
    }

    fn try_register_archetype_for_new_entities(
        &mut self,
        entity_id: EntityID,
        archetype_id: ArchetypeID,
    ) -> Result<()> {
        match self.entity_archetypes.entry(entity_id) {
            Entry::Vacant(entry) => {
                entry.insert(archetype_id);
            }
            Entry::Occupied(_) => {
                bail!("Entity with ID {entity_id} already exists");
            }
        }
        Ok(())
    }

    fn add_entities_to_table(
        &mut self,
        entity_ids: impl IntoIterator<Item = EntityID>,
        components: ArchetypeComponents<impl ComponentArray>,
    ) {
        let archetype_id = components.archetype().id();
        if let Some(idx) = self.archetype_table_indices_by_id.get(archetype_id) {
            // If we already have a table for the archetype, we add
            // the entity to it
            self.archetype_tables[idx]
                .write()
                .add_entities(entity_ids, components);
        } else {
            // If we don't have the table, initialize it with the entity
            // as the first entry
            self.archetype_table_indices_by_id.push_key(archetype_id);
            self.archetype_tables
                .push(RwLock::new(ArchetypeTable::new_with_entities(
                    entity_ids, components,
                )));
        }
    }

    fn remove_entity_data(
        &mut self,
        entity_id: EntityID,
    ) -> Result<SingleInstance<ArchetypeComponentStorage>> {
        let archetype_id = self.get_entity_archetype(entity_id)?;
        let idx = self.get_table_idx(archetype_id)?;
        let mut table = self.archetype_tables[idx].write();

        let removed_component_data = table.remove_entity(entity_id)?;

        // If we removed the last entity in the table, there is no
        // reason the keep the table any more
        if table.is_empty() {
            drop(table);
            self.remove_archetype_table_at_idx(idx);
        }

        Ok(removed_component_data)
    }

    fn remove_archetype_table_at_idx(&mut self, idx: usize) {
        self.archetype_table_indices_by_id
            .swap_remove_key_at_idx(idx);
        self.archetype_tables.swap_remove(idx);
    }

    fn add_component_storage_for_entity(
        &mut self,
        entity_id: EntityID,
        component_storage: ComponentStorage,
    ) -> Result<()> {
        // Since the archetype of the entity changes when adding a
        // component, we need to first remove it from the old table
        let mut components = self.remove_entity_data(entity_id)?.into_inner();

        // We then add the component to the entity's data
        components.add_new_component_type(component_storage)?;

        // Set new archetype for the entity
        self.entity_archetypes
            .insert(entity_id, components.archetype().id());

        // Finally we insert the modified entity into the appropriate table
        self.add_entities_to_table([entity_id], components);
        Ok(())
    }

    fn remove_component_id_for_entity(
        &mut self,
        entity_id: EntityID,
        component_id: ComponentID,
    ) -> Result<()> {
        // Since the archetype of the entity changes when removing a
        // component, we need to first remove it from the old table
        let mut components = self.remove_entity_data(entity_id)?.into_inner();

        // We then remove the component from the entity's data
        components.remove_component_type_with_id(component_id)?;

        // Set new archetype for the entity
        self.entity_archetypes
            .insert(entity_id, components.archetype().id());

        // Finally we insert the modified entity into the appropriate table
        self.add_entities_to_table([entity_id], components);
        Ok(())
    }
}

impl Default for World {
    fn default() -> Self {
        Self::new(0)
    }
}

impl<'a> EntityEntry<'a> {
    fn new(entity_id: EntityID, table: RwLockReadGuard<'a, ArchetypeTable>) -> Self {
        Self { entity_id, table }
    }

    /// Returns the [`EntityID`] of the entity.
    pub fn id(&self) -> EntityID {
        self.entity_id
    }

    /// Returns the [`Archetype`] of the entity.
    pub fn archetype(&self) -> &Archetype {
        self.table.archetype()
    }

    /// Returns the number of components the entity has.
    pub fn n_components(&self) -> usize {
        self.table.archetype().n_components()
    }

    /// Whether the entity has the component specified by the
    /// type parameter `C`.
    pub fn has_component<C: Component>(&self) -> bool {
        self.table.archetype().contains_component::<C>()
    }

    /// Returns a reference to the component specified by the
    /// type parameter `C`. If the entity does not have this
    /// component, [`None`] is returned.
    ///
    /// # Concurrency
    /// The returned reference is wrapped in a [`ComponentStorageEntry`]
    /// that holds a read lock on the [`ComponentStorage`] where the
    /// component resides. The lock is released when the entry is
    /// dropped.
    pub fn get_component<C: Component>(&self) -> Option<ComponentStorageEntry<'_, C>> {
        self.table.get_component_for_entity::<C>(self.entity_id)
    }

    /// Returns a reference to the component specified by the
    /// type parameter `C`.
    ///
    /// # Panics
    /// If the entity does not have the specified component.
    ///
    /// # Concurrency
    /// The returned reference is wrapped in a [`ComponentStorageEntry`]
    /// that holds a read lock on the [`ComponentStorage`] where the
    /// component resides. The lock is released when the entry is
    /// dropped.
    pub fn component<C: Component>(&self) -> ComponentStorageEntry<'_, C> {
        self.get_component::<C>()
            .expect("Requested invalid component")
    }

    /// Returns a mutable reference to the component specified
    /// by the type parameter `C`. If the entity does not have
    /// this component, [`None`] is returned.
    ///
    /// # Concurrency
    /// The returned reference is wrapped in a [`ComponentStorageEntryMut`]
    /// that holds a write lock on the [`ComponentStorage`] where the
    /// component resides. The lock is released when the entry is
    /// dropped.
    pub fn get_component_mut<C: Component>(&self) -> Option<ComponentStorageEntryMut<'_, C>> {
        self.table.get_component_for_entity_mut::<C>(self.entity_id)
    }

    /// Returns a mutable reference to the component specified
    /// by the type parameter `C`.
    ///
    /// # Panics
    /// If the entity does not have the specified component.
    ///
    /// # Concurrency
    /// The returned reference is wrapped in a [`ComponentStorageEntryMut`]
    /// that holds a write lock on the [`ComponentStorage`] where the
    /// component resides. The lock is released when the entry is
    /// dropped.
    pub fn component_mut<C: Component>(&self) -> ComponentStorageEntryMut<'_, C> {
        self.get_component_mut::<C>()
            .expect("Requested invalid component")
    }

    /// Returns a [`SingleInstanceArchetypeComponentStorage`] containing the
    /// cloned data for all the components of the entity.
    pub fn cloned_components(&self) -> SingleInstanceArchetypeComponentStorage {
        self.table
            .get_cloned_components_for_entity(self.entity_id)
            .unwrap()
    }
}

impl EntityStager {
    /// Creates a new stager with no staged entities.
    pub fn new() -> Self {
        Self {
            to_create_with_id: Vec::new(),
            to_create: Vec::new(),
            to_create_multiple: Vec::new(),
            to_remove: Vec::new(),
        }
    }

    /// Stages the entity defined by the given components for later creation
    /// under the given ID.
    pub fn stage_entity_for_creation_with_id<A, E>(
        &mut self,
        entity_id: EntityID,
        components: impl TryInto<SingleInstance<ArchetypeComponents<A>>, Error = E>,
    ) -> Result<()>
    where
        A: ComponentArray,
        E: Into<anyhow::Error>,
    {
        let components = components
            .try_into()
            .map_err(E::into)?
            .into_inner()
            .into_storage();

        self.to_create_with_id.push(EntityToCreateWithID {
            entity_id,
            components: SingleInstance::new(components),
        });

        Ok(())
    }

    /// Stages the entity defined by the given components for later creation.
    pub fn stage_entity_for_creation<A, E>(
        &mut self,
        components: impl TryInto<SingleInstance<ArchetypeComponents<A>>, Error = E>,
    ) -> Result<()>
    where
        A: ComponentArray,
        E: Into<anyhow::Error>,
    {
        let components = components
            .try_into()
            .map_err(E::into)?
            .into_inner()
            .into_storage();

        self.to_create.push(EntityToCreate {
            components: SingleInstance::new(components),
        });

        Ok(())
    }

    /// Stages the entities of the same archetype defined by the given
    /// components for later creation.
    pub fn stage_entities_for_creation<A, E>(
        &mut self,
        components: impl TryInto<ArchetypeComponents<A>, Error = E>,
    ) -> Result<()>
    where
        A: ComponentArray,
        E: Into<anyhow::Error>,
    {
        let components = components.try_into().map_err(E::into)?.into_storage();

        self.to_create_multiple
            .push(EntitiesToCreate { components });

        Ok(())
    }

    /// Stages the entity with the given ID for later removal.
    pub fn stage_entity_for_removal(&mut self, entity_id: EntityID) {
        self.to_remove.push(entity_id);
    }

    /// Returns a draining iterator over entities staged for creation with IDs
    /// since the last time this method was called.
    pub fn drain_entities_to_create_with_id(&mut self) -> Drain<'_, EntityToCreateWithID> {
        self.to_create_with_id.drain(..)
    }

    /// Returns a draining iterator over single entities staged for creation
    /// since the last time this method was called.
    pub fn drain_single_entities_to_create(&mut self) -> Drain<'_, EntityToCreate> {
        self.to_create.drain(..)
    }

    /// Returns a draining iterator over multi-entities staged for creation
    /// since the last time this method was called.
    pub fn drain_multi_entities_to_create(&mut self) -> Drain<'_, EntitiesToCreate> {
        self.to_create_multiple.drain(..)
    }

    /// Returns a draining iterator over entities staged for removal since the
    /// last time this method was called.
    pub fn drain_entities_to_remove(&mut self) -> Drain<'_, EntityID> {
        self.to_remove.drain(..)
    }
}

impl Default for EntityStager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        super::{Component, archetype_of},
        *,
    };
    use bytemuck::{Pod, Zeroable};

    #[repr(C)]
    #[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod, Component)]
    struct Position(f32, f32, f32);

    #[repr(C)]
    #[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod, Component)]
    struct Temperature(f32);

    type LikePosition = Position;

    const POS: Position = Position(2.5, 3.1, 42.0);
    const POS2: Position = Position(5.2, 1.3, 0.42);
    const LIKEPOS: LikePosition = Position(5.5, 5.1, 52.0);
    const TEMP: Temperature = Temperature(-40.0);
    const TEMP2: Temperature = Temperature(140.0);

    #[test]
    fn creating_world_works() {
        let world = World::default();
        assert_eq!(world.entity_count(), 0);
    }

    #[test]
    fn creating_single_entity_with_id_works() {
        let mut world = World::default();

        let entity_1_id = EntityID::hashed_from_str("entity_1");
        world.create_entity_with_id(entity_1_id, &POS).unwrap();
        assert_eq!(world.entity_count(), 1);
        let entry = world.entity(entity_1_id);
        assert_eq!(entry.archetype(), &archetype_of!(Position));
        assert_eq!(entry.n_components(), 1);
        assert_eq!(entry.component::<Position>().access(), &POS);
        drop(entry);

        let entity_2_id = EntityID::hashed_from_str("entity_2");
        world
            .create_entity_with_id(entity_2_id, (&POS, &TEMP))
            .unwrap();
        assert_eq!(world.entity_count(), 2);
        let entry = world.entity(entity_2_id);
        assert_eq!(entry.archetype(), &archetype_of!(Position, Temperature));
        assert_eq!(entry.n_components(), 2);
        assert_eq!(entry.component::<Position>().access(), &POS);
        assert_eq!(entry.component::<Temperature>().access(), &TEMP);
    }

    #[test]
    #[should_panic]
    fn creating_two_entities_with_same_id_fails() {
        let mut world = World::default();
        let entity_id = EntityID::hashed_from_str("entity");
        world.create_entity_with_id(entity_id, &POS).unwrap();
        world.create_entity_with_id(entity_id, &POS).unwrap();
    }

    #[test]
    #[should_panic]
    fn creating_entity_with_id_and_two_aliased_comps_fails() {
        let mut world = World::default();
        world
            .create_entity_with_id(EntityID::hashed_from_str("entity"), (&POS, &LIKEPOS))
            .unwrap();
    }

    #[test]
    #[should_panic]
    fn creating_entity_with_id_and_two_of_three_aliased_comps_fails() {
        let mut world = World::default();
        world
            .create_entity_with_id(EntityID::hashed_from_str("entity"), (&POS, &TEMP, &LIKEPOS))
            .unwrap();
    }

    #[test]
    fn creating_single_entity_works() {
        let mut world = World::default();

        let entity_1_id = world.create_entity(&POS).unwrap();
        assert_eq!(world.entity_count(), 1);
        let entry = world.entity(entity_1_id);
        assert_eq!(entry.archetype(), &archetype_of!(Position));
        assert_eq!(entry.n_components(), 1);
        assert_eq!(entry.component::<Position>().access(), &POS);
        drop(entry);

        let entity_2_id = world.create_entity((&POS, &TEMP)).unwrap();
        assert_eq!(world.entity_count(), 2);
        let entry = world.entity(entity_2_id);
        assert_eq!(entry.archetype(), &archetype_of!(Position, Temperature));
        assert_eq!(entry.n_components(), 2);
        assert_eq!(entry.component::<Position>().access(), &POS);
        assert_eq!(entry.component::<Temperature>().access(), &TEMP);
    }

    #[test]
    #[should_panic]
    fn creating_entity_with_two_aliased_comps_fails() {
        let mut world = World::default();
        world.create_entity((&POS, &LIKEPOS)).unwrap();
    }

    #[test]
    #[should_panic]
    fn creating_entity_with_two_of_three_aliased_comps_fails() {
        let mut world = World::default();
        world.create_entity((&POS, &TEMP, &LIKEPOS)).unwrap();
    }

    #[test]
    fn creating_two_entities_works() {
        let mut world = World::default();
        let entity_ids = world.create_entities(&[TEMP, TEMP2]).unwrap();

        assert_eq!(entity_ids.len(), 2);
        assert_eq!(world.entity_count(), 2);

        let entry = world.entity(entity_ids[0]);
        assert_eq!(entry.n_components(), 1);
        assert_eq!(entry.component::<Temperature>().access(), &TEMP);
        drop(entry);

        let entry = world.entity(entity_ids[1]);
        assert_eq!(entry.n_components(), 1);
        assert_eq!(entry.component::<Temperature>().access(), &TEMP2);
        drop(entry);

        let entity_ids = world
            .create_entities((&[POS, POS2], &[TEMP, TEMP2]))
            .unwrap();
        assert_eq!(entity_ids.len(), 2);
        assert_eq!(world.entity_count(), 4);

        let entry = world.entity(entity_ids[0]);
        assert_eq!(entry.n_components(), 2);
        assert_eq!(entry.component::<Position>().access(), &POS);
        assert_eq!(entry.component::<Temperature>().access(), &TEMP);
        drop(entry);

        let entry = world.entity(entity_ids[1]);
        assert_eq!(entry.n_components(), 2);
        assert_eq!(entry.component::<Position>().access(), &POS2);
        assert_eq!(entry.component::<Temperature>().access(), &TEMP2);
    }

    #[test]
    fn removing_entity_works() {
        let mut world = World::default();
        let entity_id = world.create_entity(&POS).unwrap();
        assert_eq!(world.entity_count(), 1);

        world.remove_entity(entity_id).unwrap();
        assert_eq!(world.entity_count(), 0);
        assert!(world.get_entity(entity_id).is_none());
    }

    #[test]
    #[should_panic]
    fn removing_same_entity_twice_fails() {
        let mut world = World::default();
        let entity_id = world.create_entity(&POS).unwrap();
        world.remove_entity(entity_id).unwrap();
        world.remove_entity(entity_id).unwrap();
    }

    #[test]
    fn removing_all_entities_from_empty_world_works() {
        let mut world = World::default();
        world.remove_all_entities();
        assert_eq!(world.entity_count(), 0);
    }

    #[test]
    fn removing_all_entities_from_single_entity_world_works() {
        let mut world = World::default();
        let entity_id = world.create_entity(&POS).unwrap();
        world.remove_all_entities();
        assert_eq!(world.entity_count(), 0);
        assert!(world.get_entity(entity_id).is_none());
    }

    #[test]
    fn removing_all_entities_from_multi_entity_world_works() {
        let mut world = World::default();
        let entity_ids = world
            .create_entities((&[POS, POS2], &[TEMP, TEMP2]))
            .unwrap();
        world.remove_all_entities();
        assert_eq!(world.entity_count(), 0);
        assert!(world.get_entity(entity_ids[0]).is_none());
        assert!(world.get_entity(entity_ids[1]).is_none());
    }

    #[test]
    fn adding_component_for_entity_works() {
        let mut world = World::default();
        let entity_id = world.create_entity(&POS).unwrap();
        world.add_component_for_entity(entity_id, &TEMP).unwrap();

        let entry = world.entity(entity_id);
        assert_eq!(entry.archetype(), &archetype_of!(Position, Temperature));
        assert_eq!(entry.n_components(), 2);
        assert_eq!(entry.component::<Position>().access(), &POS);
        assert_eq!(entry.component::<Temperature>().access(), &TEMP);
    }

    #[test]
    #[should_panic]
    fn adding_existing_component_for_entity_fails() {
        let mut world = World::default();
        let entity_id = world.create_entity(&POS).unwrap();
        world.add_component_for_entity(entity_id, &POS).unwrap();
    }

    #[test]
    fn removing_component_for_entity_works() {
        let mut world = World::default();
        let entity_id = world.create_entity((&POS, &TEMP)).unwrap();
        assert!(world.entity(entity_id).has_component::<Position>());
        world
            .remove_component_for_entity::<Position>(entity_id)
            .unwrap();

        let entry = world.entity(entity_id);
        assert_eq!(entry.archetype(), &archetype_of!(Temperature));
        assert_eq!(entry.n_components(), 1);
        assert_eq!(entry.component::<Temperature>().access(), &TEMP);
        assert!(!entry.has_component::<Position>());
    }

    #[test]
    fn removing_all_components_for_entity_works() {
        let mut world = World::default();
        let entity_id = world.create_entity((&POS, &TEMP)).unwrap();
        world
            .remove_component_for_entity::<Position>(entity_id)
            .unwrap();
        world
            .remove_component_for_entity::<Temperature>(entity_id)
            .unwrap();
        let entry = world.entity(entity_id);
        assert_eq!(entry.archetype(), &archetype_of!());
        assert_eq!(entry.n_components(), 0);
    }

    #[test]
    #[should_panic]
    fn removing_absent_component_from_entity_fails() {
        let mut world = World::default();
        let entity_id = world.create_entity(&POS).unwrap();
        world
            .remove_component_for_entity::<Temperature>(entity_id)
            .unwrap();
    }

    #[test]
    fn modifying_component_for_entity_works() {
        let mut world = World::default();
        let entity_id = world.create_entity((&POS, &TEMP)).unwrap();
        let entry = world.entity(entity_id);
        *entry.component_mut::<Temperature>().access() = TEMP2;
        assert_eq!(entry.component::<Position>().access(), &POS);
        assert_eq!(entry.component::<Temperature>().access(), &TEMP2);
    }
}
