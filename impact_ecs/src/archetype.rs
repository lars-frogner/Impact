//! Organization of ECS entities into archetypes.

use super::{
    component::{
        Component, ComponentByteView, ComponentBytes, ComponentID, ComponentInstances,
        ComponentStorage,
    },
    query::StorageAccess,
    util::KeyIndexMapper,
    world::EntityID,
};
use anyhow::{anyhow, bail, Result};
use paste::paste;
use std::{
    any::TypeId,
    collections::{hash_map::DefaultHasher, HashMap, HashSet},
    hash::{Hash, Hasher},
    sync::{RwLock, RwLockReadGuard, RwLockWriteGuard},
};

/// Representation of an archetype.
///
/// An archetype refers to a specific set of [`Component`]s
/// that an entity can have. All entities with the exact
/// same set of components belong to the same archetype.
#[derive(Clone, Debug)]
pub struct Archetype {
    id: ArchetypeID,
    component_ids: HashSet<ComponentID>,
}

/// Unique identifier for an [`Archetype`], obtained by hashing
/// the sorted list of component IDs defining the archetype.
pub type ArchetypeID = u64;

/// A table holding the component data belonging to all entities
/// with a specific archetype.
///
/// The component data is conceptually stored as a table in the
/// following manner:
/// ```txt
/// Entity ID |       Components         |
///           | Position  |   Velocity   |
/// -------------------------------------|
///      ID 0 | {x, y, z} | {vx, vy, vz} |
///      ID 1 | {x, y, z} | {vx, vy, vz} |
///      ID 2 | {x, y, z} | {vx, vy, vz} |
/// ```
/// Each column of component data exists in its own [`ComponentStorage`],
/// with components stored in the same order as the entities in
/// the first column.
///
/// Each `ComponentStorage` is protected from invalid concurrent
/// access by an individual [`RwLock`].
#[derive(Debug)]
pub struct ArchetypeTable {
    archetype: Archetype,
    /// A map from [`EntityID`] to the index of its components
    /// in the [`ComponentStorage`]s.
    entity_index_mapper: KeyIndexMapper<EntityID>,
    /// A map from [`ComponentID`] to the index of the corresponding
    /// [`ComponentStorage`] in the `component_storages` vector.
    component_index_map: HashMap<ComponentID, usize>,
    component_storages: Vec<RwLock<ComponentStorage>>,
}

/// Container holding the [`ComponentBytes`] for a set of
/// components making up a specific [`Archetype`].
#[derive(Clone, Debug)]
pub struct ArchetypeCompBytes {
    archetype: Archetype,
    component_bytes: Vec<ComponentBytes>,
    component_count: usize,
}

/// Container holding the [`ComponentByteView`] for a set
/// of components making up a specific [`Archetype`].
///
/// Instances of this type can be constructed conveniently
/// by converting from a single reference or a tuple of
/// references to anything that implements [`Component`],
/// as shown in the example below.
///
/// # Example 1
/// ```
/// # use impact_ecs::{
/// #    component::Component,
/// #    archetype::ArchetypeCompByteView
/// # };
/// # use impact_ecs_derive::ComponentDoctest;
/// # use bytemuck::{Zeroable, Pod};
/// # use anyhow::Error;
/// #
/// # #[repr(C)]
/// # #[derive(Clone, Copy, Zeroable, Pod, ComponentDoctest)]
/// # struct Position(f32, f32);
/// # #[repr(C)]
/// # #[derive(Clone, Copy, Zeroable, Pod, ComponentDoctest)]
/// # struct Mass(f32);
/// #
/// // Create instances of two components
/// let position = Position(0.0, 0.0);
/// let mass = Mass(5.0);
///
/// // We can convert from a single component..
/// let mass_bytes: ArchetypeCompByteView = (&mass).into();
/// assert_eq!(mass_bytes.n_component_types(), 1);
///
/// // .. or from a tuple of multiple components..
/// let pos_mass_bytes: ArchetypeCompByteView = (&position, &mass).try_into()?;
/// assert_eq!(pos_mass_bytes.n_component_types(), 2);
///
/// // .. or from an array if we use views to the raw bytes
/// let pos_mass_bytes: ArchetypeCompByteView = [
///     position.component_bytes(), mass.component_bytes()
/// ].try_into()?;
/// assert_eq!(pos_mass_bytes.n_component_types(), 2);
/// #
/// # Ok::<(), Error>(())
/// ```
///
/// An `ArchetypeCompByteView` may also be constructed with
/// multiple instances of each component type, by using slices
/// of component instances instead of references to single
/// instances. The following example illustrates this.
///
/// # Example 2
/// ```
/// # use impact_ecs::{
/// #    component::Component,
/// #    archetype::ArchetypeCompByteView
/// # };
/// # use impact_ecs_derive::ComponentDoctest;
/// # use bytemuck::{Zeroable, Pod};
/// # use anyhow::Error;
/// #
/// # #[repr(C)]
/// # #[derive(Clone, Copy, Zeroable, Pod, ComponentDoctest)]
/// # struct Position(f32, f32);
/// # #[repr(C)]
/// # #[derive(Clone, Copy, Zeroable, Pod, ComponentDoctest)]
/// # struct Mass(f32);
/// #
/// // Create multiple instances of each of the two components
/// let positions = [Position(0.0, 0.0), Position(2.0, 1.0), Position(6.0, 5.0)];
/// let masses = [Mass(5.0), Mass(2.0), Mass(7.5)];
///
/// let pos_mass_bytes: ArchetypeCompByteView = (&positions, &masses).try_into()?;
/// assert_eq!(pos_mass_bytes.n_component_types(), 2);
/// assert_eq!(pos_mass_bytes.component_count(), 3);
/// #
/// # Ok::<(), Error>(())
/// ```
#[derive(Clone, Debug)]
pub struct ArchetypeCompByteView<'a> {
    archetype: Archetype,
    component_bytes: Vec<ComponentByteView<'a>>,
    component_count: usize,
}

/// An immutable reference into the entry for an
/// [`Entity`](crate::world::Entity) in an [`ArchetypeTable`].
pub struct TableEntityEntry<'a> {
    info: TableEntityEntryInfo<'a>,
    components: Vec<RwLockReadGuard<'a, ComponentStorage>>,
}

/// An mutable reference into the entry for an
/// [`Entity`](crate::world::Entity) in an [`ArchetypeTable`].
pub struct TableEntityMutEntry<'a> {
    info: TableEntityEntryInfo<'a>,
    components: Vec<RwLockWriteGuard<'a, ComponentStorage>>,
}

/// Common information needed by both [`TableEntityEntry`] and
/// [`TableEntityMutEntry`].
struct TableEntityEntryInfo<'a> {
    archetype: &'a Archetype,
    /// Index of the entity's component in each component storage.
    storage_idx: usize,
    component_index_map: &'a HashMap<ComponentID, usize>,
}

impl Archetype {
    /// Creates a new archetype defined by the component IDs
    /// in the given array. The order of the component IDs
    /// does not affect the result. The array is allowed
    /// to be empty.
    ///
    /// # Errors
    /// Returns an error if the same component ID occurs
    /// multiple times in the array.
    pub fn new_from_component_id_arr<const N: usize>(
        mut component_ids: [ComponentID; N],
    ) -> Result<Self> {
        component_ids.sort();
        Self::new_from_sorted_component_id_arr(component_ids)
    }

    /// Returns the unique ID defining this archetype.
    pub fn id(&self) -> ArchetypeID {
        self.id
    }

    /// Returns the number of component making up the
    /// archetype.
    pub fn n_components(&self) -> usize {
        self.component_ids.len()
    }

    /// Whether this archetype includes the component type
    /// with the given ID.
    pub fn contains_component_id(&self, component_id: ComponentID) -> bool {
        self.component_ids.contains(&component_id)
    }

    /// Whether this archetype includes at least all the component
    /// types included in the given archetype.
    pub fn contains(&self, other: &Self) -> bool {
        self.component_ids.is_superset(&other.component_ids)
    }

    fn new_from_sorted_component_id_arr<const N: usize>(
        component_ids: [ComponentID; N],
    ) -> Result<Self> {
        if N > 0 {
            // Verify that no component is represented multiple times
            let duplicates_exist =
                (1..N).any(|idx| component_ids[idx..].contains(&component_ids[idx - 1]));
            if duplicates_exist {
                bail!("Duplicate component ID when constructing archetype");
            }
        }

        Ok(Self::new_from_sorted_component_ids_unchecked(
            &component_ids,
        ))
    }

    fn new_from_sorted_component_ids_unchecked(component_ids: &[ComponentID]) -> Self {
        let id = Self::create_id_from_sorted_component_ids(component_ids);
        let component_ids = component_ids.iter().cloned().collect();
        Self { id, component_ids }
    }

    /// Obtains an archetype ID by hashing the slice of sorted component IDs.
    fn create_id_from_sorted_component_ids(component_ids: &[ComponentID]) -> ArchetypeID {
        let mut hasher = DefaultHasher::new();
        component_ids.hash(&mut hasher);
        hasher.finish()
    }
}

impl PartialEq for Archetype {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl ArchetypeTable {
    /// Takes an iterable of [`EntityID`]s and references to all the
    /// associated component data (as an [`ArchetypeCompByteView`]),
    /// initializes a table for the corresponding [`Archetype`] and
    /// inserts the given data, one row per entity.
    ///
    /// # Panics
    /// - If the number of entities differs from the number of instances
    /// of each component type.
    /// - If any of the entity IDs are equal.
    pub fn new_with_entities(
        entity_ids: impl IntoIterator<Item = EntityID>,
        components: ArchetypeCompByteView,
    ) -> Self {
        // Initialize mapper between entity ID and index in component storages
        let entity_index_mapper = KeyIndexMapper::new_with_keys(entity_ids);

        Self::new_with_entity_index_mapper(entity_index_mapper, components)
    }

    /// Returns the [`Archetype`] of the table.
    pub fn archetype(&self) -> &Archetype {
        &self.archetype
    }

    /// Whether no entities remain in the table.
    pub fn is_empty(&self) -> bool {
        self.entity_index_mapper.is_empty()
    }

    /// Whether the [`Entity`] with the given [`EntityID`] is present in the table.
    pub fn has_entity(&self, entity_id: EntityID) -> bool {
        self.entity_index_mapper.contains_key(entity_id)
    }

    /// Takes an iterable of [`EntityID`]s and references to all the
    /// associated component data (as an [`ArchetypeCompByteView`])
    /// and appends the given data to the table, one row per entity.
    ///
    /// # Panics
    /// - If the number of entities differs from the number of instances
    /// of each component type.
    /// - If any of the given entity IDs are equal to a new or existing
    /// entity ID.
    pub fn add_entities(
        &mut self,
        entity_ids: impl IntoIterator<Item = EntityID>,
        components: ArchetypeCompByteView,
    ) {
        let original_entity_count = self.entity_index_mapper.len();
        self.entity_index_mapper.push_keys(entity_ids);
        let added_entity_count = self.entity_index_mapper.len() - original_entity_count;
        assert_eq!(
            added_entity_count, components.component_count,
            "Number of components per component type differs from number of entities"
        );

        self.component_storages
            .iter_mut()
            .zip(components.component_bytes.into_iter())
            .for_each(|(storage, data)| storage.write().unwrap().push_bytes(data));
    }

    /// Removes the entity with the given [`EntityID`] and all its
    /// data from the table.
    ///
    /// # Returns
    /// The removed component data.
    ///
    /// # Errors
    /// Returns an error if the entity is not present in the table.
    pub fn remove_entity(&mut self, entity_id: EntityID) -> Result<ArchetypeCompBytes> {
        if !self.has_entity(entity_id) {
            bail!("Entity to remove not present in archetype table");
        }
        // Remove the entity from the map and obtain the index
        // of the corresponing component data. We do a swap remove
        // in order to keep the index map consistent when we do a
        // swap remove of component data.
        let idx = self.entity_index_mapper.swap_remove_key(entity_id);

        // Perform an equivalent swap remove of the data at the index we found
        let removed_component_bytes = self
            .component_storages
            .iter_mut()
            .map(|storage| storage.write().unwrap().swap_remove_bytes(idx))
            .collect();

        Ok(ArchetypeCompBytes {
            archetype: self.archetype.clone(),
            component_bytes: removed_component_bytes,
            component_count: 1,
        })
    }

    /// Returns an [`TableEntityEntry`] that can be used to read the components
    /// of the [`Entity`](crate::world::Entity) with the given [`EntityID`].
    /// If the entity is not present in the table, [`None`] is returned.
    ///
    /// # Concurrency
    /// The returned `TableEntityEntry` holds locks to the component storages
    /// in the table until it is dropped. Before then, attempts to modify
    /// the component data will be blocked.
    pub fn get_entity(&self, entity_id: EntityID) -> Option<TableEntityEntry> {
        let storage_idx = self.entity_index_mapper.get(entity_id)?;
        Some(TableEntityEntry::new(
            &self.archetype,
            storage_idx,
            &self.component_index_map,
            self.component_storages
                .iter()
                .map(|storage| storage.read().unwrap())
                .collect(),
        ))
    }

    /// Returns an [`TableEntityEntry`] that can be used to read the components
    /// of the [`Entity`](crate::world::Entity) with the given [`EntityID`].
    ///
    /// # Panics
    /// If the entity is not present in the table.
    ///
    /// # Concurrency
    /// The returned `TableEntityEntry` holds locks to the component storages
    /// in the table until it is dropped. Before then, attempts to modify
    /// the component data will be blocked.
    pub fn entity(&self, entity_id: EntityID) -> TableEntityEntry {
        self.get_entity(entity_id)
            .expect("Entity not present in table")
    }

    /// Returns an [`TableEntityMutEntry`] that can be used to read and modify
    /// the components of the [`Entity`](crate::world::Entity) with the
    /// given [`EntityID`]. If the entity is not present in the table,
    /// [`None`] is returned.
    ///
    /// # Concurrency
    /// The returned `TableEntityMutEntry` holds locks to the component storages
    /// in the table until it is dropped. Before then, attempts to read
    /// or modify the component data will be blocked.
    pub fn get_entity_mut(&mut self, entity_id: EntityID) -> Option<TableEntityMutEntry> {
        let storage_idx = self.entity_index_mapper.get(entity_id)?;
        Some(TableEntityMutEntry::new(
            &self.archetype,
            storage_idx,
            &self.component_index_map,
            self.component_storages
                .iter_mut()
                .map(|storage| storage.write().unwrap())
                .collect(),
        ))
    }

    /// Returns an [`TableEntityMutEntry`] that can be used to read and modify
    /// the components of the [`Entity`](crate::world::Entity) with the
    /// given [`EntityID`].
    ///
    /// # Panics
    /// If the entity is not present in the table.
    ///
    /// # Concurrency
    /// The returned `TableEntityMutEntry` holds locks to the component storages
    /// in the table until it is dropped. Before then, attempts to read
    /// or modify the component data will be blocked.
    pub fn entity_mut(&mut self, entity_id: EntityID) -> TableEntityMutEntry {
        self.get_entity_mut(entity_id)
            .expect("Entity not present in table")
    }

    /// Provides access to a [`ComponentStorage`] (guarded by a [`RwLock`]).
    ///
    /// The component type to access is given by the `C` type parameter,
    /// while the `A` type parameter specifies what kind of access (i.e.
    /// read or write, see [`StorageAccess`]).
    ///
    /// # Errors
    /// Returns an error if `C` is not one of the component types present
    /// in the table.
    pub fn access_component_storage<'g, 's: 'g, C, A>(&'s self) -> Result<A::Guard>
    where
        C: Component,
        A: StorageAccess<'g, C>,
    {
        Ok(A::access(self.get_component_storage(C::component_id())?))
    }

    fn new_with_entity_index_mapper(
        entity_index_mapper: KeyIndexMapper<EntityID>,
        ArchetypeCompByteView {
            archetype,
            component_bytes,
            component_count,
        }: ArchetypeCompByteView,
    ) -> Self {
        assert_eq!(
            entity_index_mapper.len(),
            component_count,
            "Number of components per component type differs from number of entities"
        );
        Self {
            archetype,
            entity_index_mapper,
            // For component IDs we don't need a full `KeyIndexMapper`, so we just
            // unwrap to the underlying `HashMap`
            component_index_map: KeyIndexMapper::new_with_keys(
                component_bytes.iter().map(ComponentByteView::component_id),
            )
            .into_map(),
            // Initialize storages with component data for the provided entity
            component_storages: component_bytes
                .into_iter()
                .map(|bytes| RwLock::new(ComponentStorage::new_with_bytes(bytes)))
                .collect(),
        }
    }

    fn get_component_storage(
        &self,
        component_id: ComponentID,
    ) -> Result<&RwLock<ComponentStorage>> {
        let idx = *self
            .component_index_map
            .get(&component_id)
            .ok_or_else(|| anyhow!("Component not present in archetype table"))?;
        Ok(&self.component_storages[idx])
    }
}

impl<'a> TableEntityEntry<'a> {
    fn new(
        archetype: &'a Archetype,
        storage_idx: usize,
        component_index_map: &'a HashMap<ComponentID, usize>,
        components: Vec<RwLockReadGuard<'a, ComponentStorage>>,
    ) -> Self {
        Self {
            info: TableEntityEntryInfo {
                archetype,
                storage_idx,
                component_index_map,
            },
            components,
        }
    }

    /// Returns the number of components the entity has.
    pub fn n_components(&self) -> usize {
        self.info.n_components()
    }

    /// Whether the entity has the component specified by the
    /// type parameter `C`.
    pub fn has_component<C: Component>(&self) -> bool {
        self.info.has_component::<C>()
    }

    /// Returns a reference to the component specified by the
    /// type parameter `C`. If the entity does not have this
    /// component, [`None`] is returned.
    pub fn get_component<C: Component>(&self) -> Option<&C> {
        let component_idx = *self.info.component_index_map.get(&C::component_id())?;
        Some(&self.components[component_idx].slice()[self.info.storage_idx])
    }

    /// Returns a reference to the component specified by the
    /// type parameter `C`.
    ///
    /// # Panics
    /// If the entity does not have the specified component.
    pub fn component<C: Component>(&self) -> &C {
        self.get_component::<C>()
            .expect("Requested invalid component")
    }
}

impl<'a> TableEntityMutEntry<'a> {
    fn new(
        archetype: &'a Archetype,
        storage_idx: usize,
        component_index_map: &'a HashMap<ComponentID, usize>,
        components: Vec<RwLockWriteGuard<'a, ComponentStorage>>,
    ) -> Self {
        Self {
            info: TableEntityEntryInfo {
                archetype,
                storage_idx,
                component_index_map,
            },
            components,
        }
    }

    /// Returns the number of components the entity has.
    pub fn n_components(&self) -> usize {
        self.info.n_components()
    }

    /// Whether the entity has the component specified by the
    /// type parameter `C`.
    pub fn has_component<C: Component>(&self) -> bool {
        self.info.has_component::<C>()
    }

    /// Returns a mutable reference to the component specified
    /// by the type parameter `C`. If the entity does not have
    /// this component, [`None`] is returned.
    pub fn get_component<C: Component>(&mut self) -> Option<&mut C> {
        let component_idx = *self.info.component_index_map.get(&C::component_id())?;
        Some(&mut self.components[component_idx].slice_mut()[self.info.storage_idx])
    }

    /// Returns a mutable reference to the component specified
    /// by the type parameter `C`.
    ///
    /// # Panics
    /// If the entity does not have the specified component.
    pub fn component<C: Component>(&mut self) -> &mut C {
        self.get_component::<C>()
            .expect("Requested invalid component")
    }
}

impl<'a> TableEntityEntryInfo<'a> {
    fn n_components(&self) -> usize {
        self.archetype.n_components()
    }

    fn has_component<C: Component>(&self) -> bool {
        self.archetype.contains_component_id(C::component_id())
    }
}

impl ArchetypeCompBytes {
    /// Returns the unique ID for the archetype corresponding
    /// to the set of components whose bytes are stored here.
    pub fn archetype_id(&self) -> ArchetypeID {
        self.archetype.id()
    }

    /// Returns the number of component types present in the bytes
    /// stored here.
    pub fn n_component_types(&self) -> usize {
        self.archetype.n_components()
    }

    /// Returns the number of instances of each component type
    /// present in the bytes stored here.
    pub fn component_count(&self) -> usize {
        self.component_count
    }

    /// Returns an [`ArchetypeCompByteView`] referencing the component
    /// bytes.
    pub fn as_ref(&self) -> ArchetypeCompByteView {
        ArchetypeCompByteView {
            archetype: self.archetype.clone(),
            component_bytes: self
                .component_bytes
                .iter()
                .map(ComponentBytes::as_ref)
                .collect(),
            component_count: self.component_count(),
        }
    }
}

impl<'a> ArchetypeCompByteView<'a> {
    /// Returns the unique ID for the archetype corresponding
    /// to the set of components whose bytes are referenced here.
    pub fn archetype_id(&self) -> ArchetypeID {
        self.archetype.id()
    }

    /// Returns the number of component types present in the bytes
    /// referenced here.
    pub fn n_component_types(&self) -> usize {
        self.archetype.n_components()
    }

    /// Returns the number of instances of each component type
    /// present in the bytes referenced here.
    pub fn component_count(&self) -> usize {
        self.component_count
    }

    /// Includes the given component in the set of components
    /// whose bytes are referenced here. Note that this changes
    /// the corresponding archetype.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The type of the given component is already present.
    /// - The number of component instances differs between
    /// the new and the existing component types.
    pub fn add_new_component(&mut self, component_bytes: ComponentByteView<'a>) -> Result<()> {
        if self.component_bytes.is_empty() {
            self.component_count = component_bytes.component_count();
        } else if component_bytes.component_count() != self.component_count() {
            bail!("Inconsistent number of component instances in added component data");
        }

        // Find where to insert the given component to keep
        // the components sorted by ID
        match self.component_bytes.binary_search_by_key(
            &component_bytes.component_id(),
            ComponentByteView::component_id,
        ) {
            // Panic if the component is already present, otherwise insert
            // at the correct position
            Ok(_) => {
                bail!("Component already exists for archetype")
            }
            Err(idx) => self.component_bytes.insert(idx, component_bytes),
        }

        // Update archetype
        self.archetype = Self::find_archetype_from_sorted_components(&self.component_bytes);

        Ok(())
    }

    /// Removes the component with the given ID from the set
    /// of components whose bytes are referenced here. Note
    /// that this changes the corresponding archetype.
    ///
    /// # Errors
    /// Returns an error if the component type to remove is
    /// not present.
    pub fn remove_component_with_id(&mut self, component_id: ComponentID) -> Result<()> {
        let idx = self
            .component_bytes
            .binary_search_by_key(&component_id, ComponentByteView::component_id)
            .map_err(|_| anyhow!("Tried to remove missing component"))?;

        self.component_bytes.remove(idx);

        // Update archetype
        self.archetype = Self::find_archetype_from_sorted_components(&self.component_bytes);

        Ok(())
    }

    fn find_archetype_from_sorted_components(
        component_data: &[ComponentByteView<'a>],
    ) -> Archetype {
        let component_ids: Vec<_> = component_data
            .iter()
            .map(ComponentByteView::component_id)
            .collect();
        Archetype::new_from_sorted_component_ids_unchecked(&component_ids)
    }
}

// Implement `TryFrom` so that an array of `ComponentByteView`s can
// be converted into an `ArchetypeCompByteView`.
impl<'a, const N: usize> TryFrom<[ComponentByteView<'a>; N]> for ArchetypeCompByteView<'a> {
    type Error = anyhow::Error;

    fn try_from(mut component_data: [ComponentByteView<'a>; N]) -> Result<Self> {
        // Find the number of component instances and check that this is the
        // same for all the component types
        let mut component_iter = component_data.iter();
        let component_count = component_iter
            .next()
            .map_or(0, ComponentByteView::component_count);
        if component_iter.any(|view| view.component_count() != component_count) {
            bail!("The number of component instances differs between component types");
        }

        // Make sure components are sorted by id
        component_data.sort_by_key(|data| data.component_id());

        // Use arbitrary type ID to initialize array for component IDs
        // (will be overwritten)
        let dummy_type_id = TypeId::of::<u8>();
        let mut component_ids = [dummy_type_id; N];

        // Populate array of component IDs
        component_ids
            .iter_mut()
            .zip(component_data.iter())
            .for_each(|(id, data)| *id = data.component_id());

        let archetype = Archetype::new_from_sorted_component_id_arr(component_ids)?;

        Ok(Self {
            archetype,
            component_bytes: component_data.to_vec(),
            component_count,
        })
    }
}

/// Macro for implementing [`From<C>`] or [`TryFrom<C>`] for
/// [`ArchetypeCompByteView`], where `C` respectively is a single
/// [`Component`] reference/slice or tuple of references/slices.
macro_rules! impl_archetype_conversion {
    ($c:ident) => {
        // For a single instance of a single component type
        impl<'a, $c> From<&'a $c> for ArchetypeCompByteView<'a>
        where
            $c: 'a + Component,
        {
            fn from(component: &'a $c) -> Self {
                [component.component_bytes()].try_into().unwrap()
            }
        }
        // For a a slice of instances of a single component type
        impl<'a, $c> From<&'a [$c]> for ArchetypeCompByteView<'a>
        where
            $c: 'a + Component,
        {
            fn from(component_slice: &'a [$c]) -> Self {
                [component_slice.component_bytes()].try_into().unwrap()
            }
        }
        // For a fixed length slice of instances of a single component type
        impl<'a, const N: usize, $c> From<&'a [$c; N]> for ArchetypeCompByteView<'a>
        where
            $c: 'a + Component,
        {
            fn from(component_slice: &'a [$c; N]) -> Self {
                [component_slice.component_bytes()].try_into().unwrap()
            }
        }
    };
    (($($c:ident),*)) => {
        // For single instances of multiple component types
        impl<'a, $($c),*> TryFrom<($(&'a $c),*)> for ArchetypeCompByteView<'a>
        where
        $($c: 'a + Component,)*
        {
            type Error = anyhow::Error;
            #[allow(non_snake_case)]
            fn try_from(($(paste! { [<component_ $c>] }),*): ($(&'a $c),*)) -> Result<Self> {
                [$(paste! { [<component_ $c>] }.component_bytes()),*].try_into()
            }
        }
        // For slices of instances of multiple component types
        impl<'a, $($c),*> TryFrom<($(&'a [$c]),*)> for ArchetypeCompByteView<'a>
        where
        $($c: 'a + Component,)*
        {
            type Error = anyhow::Error;
            #[allow(non_snake_case)]
            fn try_from(($(paste! { [<component_slice_ $c>] }),*): ($(&'a [$c]),*)) -> Result<Self> {
                [$(paste! { [<component_slice_ $c>] }.component_bytes()),*].try_into()
            }
        }
        // For fixed size slices of instances of multiple component types
        impl<'a, const N: usize, $($c),*> TryFrom<($(&'a [$c; N]),*)> for ArchetypeCompByteView<'a>
        where
        $($c: 'a + Component,)*
        {
            type Error = anyhow::Error;
            #[allow(non_snake_case)]
            fn try_from(($(paste! { [<component_slice_ $c>] }),*): ($(&'a [$c; N]),*)) -> Result<Self> {
                [$(paste! { [<component_slice_ $c>] }.component_bytes()),*].try_into()
            }
        }
    };
}

impl_archetype_conversion!(C1);
impl_archetype_conversion!((C1, C2));
impl_archetype_conversion!((C1, C2, C3));
impl_archetype_conversion!((C1, C2, C3, C4));
impl_archetype_conversion!((C1, C2, C3, C4, C5));
impl_archetype_conversion!((C1, C2, C3, C4, C5, C6));
impl_archetype_conversion!((C1, C2, C3, C4, C5, C6, C7));
impl_archetype_conversion!((C1, C2, C3, C4, C5, C6, C7, C8));

#[cfg(test)]
mod test {
    use super::{super::Component, *};
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

    const BYTE: Byte = Byte(42);
    const POS: Position = Position(-9.8, 12.5, 7.3);
    const RECT: Rectangle = Rectangle {
        center: [2.5, 2.0],
        dimensions: [12.3, 8.9],
    };

    #[test]
    fn larger_archetypes_contain_smaller_archetypes() {
        let with_all_components = Archetype::new_from_component_id_arr([
            Byte::component_id(),
            Position::component_id(),
            Rectangle::component_id(),
        ])
        .unwrap();
        let without_byte = Archetype::new_from_component_id_arr([
            Rectangle::component_id(),
            Position::component_id(),
        ])
        .unwrap();
        let without_position =
            Archetype::new_from_component_id_arr([Byte::component_id(), Rectangle::component_id()])
                .unwrap();
        let empty = Archetype::new_from_component_id_arr([]).unwrap();

        assert!(with_all_components.contains(&with_all_components));
        assert!(with_all_components.contains(&without_byte));
        assert!(!without_byte.contains(&with_all_components));
        assert!(!without_byte.contains(&without_position));
        assert!(with_all_components.contains(&empty));
        assert!(!empty.contains(&with_all_components));
    }

    #[test]
    #[should_panic]
    fn conversion_of_two_comp_array_with_two_equal_comps_to_byte_view_fails() {
        let _: ArchetypeCompByteView = [BYTE.component_bytes(), BYTE.component_bytes()]
            .try_into()
            .unwrap();
    }

    #[test]
    #[should_panic]
    fn conversion_of_three_comp_array_with_two_equal_comps_to_byte_view_fails() {
        let _: ArchetypeCompByteView = [
            BYTE.component_bytes(),
            POS.component_bytes(),
            BYTE.component_bytes(),
        ]
        .try_into()
        .unwrap();
    }

    #[test]
    fn valid_conversion_of_comp_arrays_to_byte_views_succeed() {
        let view: ArchetypeCompByteView = [].try_into().unwrap();
        assert_eq!(
            view.archetype,
            Archetype::new_from_component_id_arr([]).unwrap()
        );

        let view: ArchetypeCompByteView = [BYTE.component_bytes()].try_into().unwrap();
        assert_eq!(
            view.archetype,
            Archetype::new_from_component_id_arr([Byte::component_id()]).unwrap()
        );

        let view: ArchetypeCompByteView = [BYTE.component_bytes(), POS.component_bytes()]
            .try_into()
            .unwrap();
        assert_eq!(
            view.archetype,
            Archetype::new_from_component_id_arr([Byte::component_id(), Position::component_id()])
                .unwrap()
        );

        let view: ArchetypeCompByteView = [
            BYTE.component_bytes(),
            POS.component_bytes(),
            RECT.component_bytes(),
        ]
        .try_into()
        .unwrap();
        assert_eq!(
            view.archetype,
            Archetype::new_from_component_id_arr([
                Byte::component_id(),
                Position::component_id(),
                Rectangle::component_id()
            ])
            .unwrap()
        );
    }

    #[test]
    fn order_of_comps_for_byte_view_does_not_matter() {
        let view_1: ArchetypeCompByteView = (&BYTE, &POS, &RECT).try_into().unwrap();
        let view_2: ArchetypeCompByteView = (&POS, &BYTE, &RECT).try_into().unwrap();
        let view_3: ArchetypeCompByteView = (&RECT, &BYTE, &POS).try_into().unwrap();
        assert_eq!(view_2.archetype, view_1.archetype);
        assert_eq!(view_3.archetype, view_1.archetype);
    }

    #[test]
    #[should_panic]
    fn conversion_of_two_comp_tuple_with_two_equal_comps_to_byte_view_fails() {
        let _: ArchetypeCompByteView = (&POS, &POS).try_into().unwrap();
    }

    #[test]
    #[should_panic]
    fn conversion_of_three_comp_tuple_with_two_equal_comps_to_byte_view_fails() {
        let _: ArchetypeCompByteView = (&POS, &BYTE, &POS).try_into().unwrap();
    }

    #[test]
    fn valid_conversion_of_comp_tuples_to_byte_views_succeed() {
        let view: ArchetypeCompByteView = (&BYTE).into();
        assert_eq!(
            view.archetype,
            Archetype::new_from_component_id_arr([Byte::component_id()]).unwrap()
        );

        let view: ArchetypeCompByteView = (&BYTE, &POS).try_into().unwrap();
        assert_eq!(
            view.archetype,
            Archetype::new_from_component_id_arr([Byte::component_id(), Position::component_id()])
                .unwrap()
        );

        let view: ArchetypeCompByteView = (&BYTE, &POS, &RECT).try_into().unwrap();
        assert_eq!(
            view.archetype,
            Archetype::new_from_component_id_arr([
                Byte::component_id(),
                Position::component_id(),
                Rectangle::component_id()
            ])
            .unwrap()
        );
    }

    #[test]
    fn adding_components_to_archetype_byte_view_works() {
        let mut view: ArchetypeCompByteView = [].try_into().unwrap();
        view.add_new_component(BYTE.component_bytes()).unwrap();
        assert_eq!(
            view.archetype,
            Archetype::new_from_component_id_arr([Byte::component_id()]).unwrap()
        );

        view.add_new_component(POS.component_bytes()).unwrap();
        assert_eq!(
            view.archetype,
            Archetype::new_from_component_id_arr([Byte::component_id(), Position::component_id()])
                .unwrap()
        );

        view.add_new_component(RECT.component_bytes()).unwrap();
        assert_eq!(
            view.archetype,
            Archetype::new_from_component_id_arr([
                Byte::component_id(),
                Position::component_id(),
                Rectangle::component_id()
            ])
            .unwrap()
        );
    }

    #[test]
    #[should_panic]
    fn adding_existing_component_to_archetype_byte_view_fails() {
        let mut view: ArchetypeCompByteView = (&BYTE, &POS, &RECT).try_into().unwrap();
        view.add_new_component(POS.component_bytes()).unwrap();
    }

    #[test]
    fn removing_components_from_archetype_byte_view_works() {
        let mut view: ArchetypeCompByteView = (&BYTE, &POS, &RECT).try_into().unwrap();
        view.remove_component_with_id(Byte::component_id()).unwrap();
        assert_eq!(
            view.archetype,
            Archetype::new_from_component_id_arr([
                Position::component_id(),
                Rectangle::component_id()
            ])
            .unwrap()
        );

        view.remove_component_with_id(Rectangle::component_id())
            .unwrap();
        assert_eq!(
            view.archetype,
            Archetype::new_from_component_id_arr([Position::component_id()]).unwrap()
        );

        view.remove_component_with_id(Position::component_id())
            .unwrap();
        assert_eq!(
            view.archetype,
            Archetype::new_from_component_id_arr([]).unwrap()
        );
    }

    #[test]
    #[should_panic]
    fn removing_missing_component_from_archetype_byte_view_fails() {
        let mut view: ArchetypeCompByteView = (&BYTE, &RECT).try_into().unwrap();
        view.remove_component_with_id(Position::component_id())
            .unwrap();
    }

    #[test]
    #[should_panic]
    fn removing_component_from_empty_archetype_byte_view_fails() {
        let mut view: ArchetypeCompByteView = [].try_into().unwrap();
        view.remove_component_with_id(Position::component_id())
            .unwrap();
    }

    #[test]
    fn constructing_table_works() {
        let table = ArchetypeTable::new_with_entities([0], (&BYTE).into());
        assert!(table.has_entity(0));
        assert_eq!(table.entity(0).component::<Byte>(), &BYTE);

        let table = ArchetypeTable::new_with_entities([42], (&RECT, &POS).try_into().unwrap());
        assert!(table.has_entity(42));
        let entity = table.entity(42);
        assert_eq!(entity.component::<Position>(), &POS);
        assert_eq!(entity.component::<Rectangle>(), &RECT);

        let table =
            ArchetypeTable::new_with_entities([10], (&BYTE, &RECT, &POS).try_into().unwrap());
        assert!(table.has_entity(10));
        let entity = table.entity(10);
        assert_eq!(entity.component::<Byte>(), &BYTE);
        assert_eq!(entity.component::<Position>(), &POS);
        assert_eq!(entity.component::<Rectangle>(), &RECT);
    }

    #[test]
    fn adding_entity_to_table_works() {
        let mut table = ArchetypeTable::new_with_entities([0], (&BYTE).into());
        table.add_entities([1], (&BYTE).into());
        assert!(table.has_entity(0));
        assert_eq!(table.entity(0).component::<Byte>(), &BYTE);
        assert!(table.has_entity(1));
        assert_eq!(table.entity(1).component::<Byte>(), &BYTE);

        let mut table = ArchetypeTable::new_with_entities([3], (&RECT, &POS).try_into().unwrap());
        table.add_entities([7], (&RECT, &POS).try_into().unwrap());
        assert!(table.has_entity(3));
        let entity = table.entity(3);
        assert_eq!(entity.component::<Position>(), &POS);
        assert_eq!(entity.component::<Rectangle>(), &RECT);
        assert!(table.has_entity(7));
        let entity = table.entity(7);
        assert_eq!(entity.component::<Position>(), &POS);
        assert_eq!(entity.component::<Rectangle>(), &RECT);
    }

    #[test]
    #[should_panic]
    fn adding_existing_entity_to_table_fails() {
        let mut table = ArchetypeTable::new_with_entities([0], (&BYTE).into());
        table.add_entities([0], (&BYTE).into());
    }

    #[test]
    fn removing_entity_from_table_works() {
        let mut table = ArchetypeTable::new_with_entities([0], (&RECT, &POS).try_into().unwrap());
        table.add_entities([1], (&RECT, &POS).try_into().unwrap());

        table.remove_entity(0).unwrap();
        assert!(!table.has_entity(0));
        assert!(table.has_entity(1));

        table.remove_entity(1).unwrap();
        assert!(table.is_empty());
    }

    #[test]
    #[should_panic]
    fn removing_missing_entity_from_table_fails() {
        let mut table = ArchetypeTable::new_with_entities([0], (&RECT, &POS).try_into().unwrap());
        table.remove_entity(1).unwrap();
    }

    #[test]
    #[should_panic]
    fn removing_entity_from_empty_table_fails() {
        let mut table = ArchetypeTable::new_with_entities([0], (&RECT, &POS).try_into().unwrap());
        table.remove_entity(0).unwrap();
        table.remove_entity(0).unwrap();
    }
}
