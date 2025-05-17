//! Organization of ECS entities into archetypes.

use super::{
    NoHashKeyIndexMapper, NoHashMap, NoHashSet,
    component::{
        CanHaveSingleInstance, Component, ComponentArray, ComponentID, ComponentSlice,
        ComponentStorage, ComponentView, SingleInstance,
    },
    world::{Entity, EntityID},
};
use anyhow::{Result, anyhow, bail};
use bytemuck::{Pod, Zeroable};
use impact_containers::KeyIndexMapper;
use impact_ecs_macros::archetype_of;
use paste::paste;
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    iter,
    marker::PhantomData,
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
    component_ids: NoHashSet<ComponentID>,
}

/// Unique identifier for an [`Archetype`], obtained by hashing
/// the sorted list of component IDs defining the archetype.
#[repr(C)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Zeroable, Pod)]
pub struct ArchetypeID(u32);

/// Container holding [`ComponentArray`]s for a set
/// of components making up a specific [`Archetype`].
///
/// See also the type instantiations [`ArchetypeComponentView`]
/// and [`ArchetypeComponentStorage`].
#[derive(Debug)]
pub struct ArchetypeComponents<A> {
    archetype: Archetype,
    component_index_map: NoHashKeyIndexMapper<ComponentID>,
    component_arrays: Vec<A>,
    component_count: usize,
}

/// Container holding [`ComponentView`]s referencing a set
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
/// #    component::{Component, ComponentSlice, SingleInstance},
/// #    archetype::ArchetypeComponentView
/// # };
/// # use impact_ecs_macros::ComponentDoctest as Component;
/// # use bytemuck::{Zeroable, Pod};
/// # use anyhow::Error;
/// #
/// # #[repr(C)]
/// # #[derive(Clone, Copy, Zeroable, Pod, Component)]
/// # struct Position(f32, f32);
/// # #[repr(C)]
/// # #[derive(Clone, Copy, Zeroable, Pod, Component)]
/// # struct Mass(f32);
/// #
/// // Create instances of two components
/// let position = Position(0.0, 0.0);
/// let mass = Mass(5.0);
///
/// // We can convert from a single component..
/// let mass_view: ArchetypeComponentView<'_> = (&mass).into();
/// assert_eq!(mass_view.n_component_types(), 1);
///
/// // .. or from a tuple of multiple components..
/// let pos_mass_view: ArchetypeComponentView<'_> = (&position, &mass).try_into()?;
/// assert_eq!(pos_mass_view.n_component_types(), 2);
///
/// // .. or from an array if we first convert into `ComponentView`s
/// let pos_mass_view: ArchetypeComponentView<'_> = [
///     (&position).persistent_view(), (&mass).persistent_view()
/// ].try_into()?;
/// assert_eq!(pos_mass_view.n_component_types(), 2);
///
/// // We can also convert directly into a `SingleInstance`-wrapped view
/// let mass_view: SingleInstance<ArchetypeComponentView<'_>> = (&mass).into();
/// // ..
/// #
/// # Ok::<(), Error>(())
/// ```
///
/// An `ArchetypeComponentView` may also be constructed with
/// multiple instances of each component type, by using slices
/// of component instances instead of references to single
/// instances. The following example illustrates this.
///
/// # Example 2
/// ```
/// # use impact_ecs::{
/// #    component::Component,
/// #    archetype::ArchetypeComponentView
/// # };
/// # use impact_ecs_macros::ComponentDoctest as Component;
/// # use bytemuck::{Zeroable, Pod};
/// # use anyhow::Error;
/// #
/// # #[repr(C)]
/// # #[derive(Clone, Copy, Zeroable, Pod, Component)]
/// # struct Position(f32, f32);
/// # #[repr(C)]
/// # #[derive(Clone, Copy, Zeroable, Pod, Component)]
/// # struct Mass(f32);
/// #
/// // Create multiple instances of each of the two components
/// let positions = [Position(0.0, 0.0), Position(2.0, 1.0), Position(6.0, 5.0)];
/// let masses = [Mass(5.0), Mass(2.0), Mass(7.5)];
///
/// let pos_mass_view: ArchetypeComponentView<'_> = (&positions, &masses).try_into()?;
/// assert_eq!(pos_mass_view.n_component_types(), 2);
/// assert_eq!(pos_mass_view.component_count(), 3);
/// #
/// # Ok::<(), Error>(())
/// ```
pub type ArchetypeComponentView<'a> = ArchetypeComponents<ComponentView<'a>>;

/// Container holding [`ComponentStorage`]s for a set
/// of components making up a specific [`Archetype`].
pub type ArchetypeComponentStorage = ArchetypeComponents<ComponentStorage>;

/// Container holding [`SingleInstance`] [`ComponentStorage`]s for a set
/// of components making up a specific [`Archetype`].
pub type SingleInstanceArchetypeComponentStorage =
    ArchetypeComponents<SingleInstance<ComponentStorage>>;

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
    component_index_map: NoHashMap<ComponentID, usize>,
    component_storages: Vec<RwLock<ComponentStorage>>,
}

/// An immutable reference into the entry for an
/// [`Entity`](crate::world::Entity) in an [`ArchetypeTable`].
#[derive(Debug)]
pub struct TableEntityEntry<'a> {
    info: TableEntityEntryInfo<'a>,
    components: Vec<RwLockReadGuard<'a, ComponentStorage>>,
}

/// An mutable reference into the entry for an
/// [`Entity`](crate::world::Entity) in an [`ArchetypeTable`].
#[derive(Debug)]
pub struct TableEntityMutEntry<'a> {
    info: TableEntityEntryInfo<'a>,
    components: Vec<RwLockWriteGuard<'a, ComponentStorage>>,
}

/// Common information needed by both [`TableEntityEntry`] and
/// [`TableEntityMutEntry`].
#[derive(Debug)]
struct TableEntityEntryInfo<'a> {
    archetype: &'a Archetype,
    /// Index of the entity's component in each component storage.
    entity_idx: usize,
    component_index_map: &'a NoHashMap<ComponentID, usize>,
}

/// Immutable reference to the entry for a specific component
/// instance in a [`ComponentStorage`]
#[derive(Debug)]
pub struct ComponentStorageEntry<'a, C> {
    entity_idx: usize,
    storage: RwLockReadGuard<'a, ComponentStorage>,
    _phantom: PhantomData<C>,
}

/// Mutable reference to the entry for a specific component
/// instance in a [`ComponentStorage`]
#[derive(Debug)]
pub struct ComponentStorageEntryMut<'a, C> {
    entity_idx: usize,
    storage: RwLockWriteGuard<'a, ComponentStorage>,
    _phantom: PhantomData<C>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum InclusionResult {
    Added,
    Overwritten,
}

impl ArchetypeID {
    /// Converts the given `u32` into an archetype ID. Should only be called
    /// with values returned from [`Self::as_u32`].
    pub const fn from_u32(value: u32) -> Self {
        Self(value)
    }

    /// Returns the `u32` value corresponding to the archetype ID.
    pub const fn as_u32(&self) -> u32 {
        self.0
    }
}

impl Hash for ArchetypeID {
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        hasher.write_u32(self.0);
    }
}

impl nohash_hasher::IsEnabled for ArchetypeID {}

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

    /// Whether this archetype includes components of type `C`.
    pub fn contains_component<C: Component>(&self) -> bool {
        self.contains_component_id(C::component_id())
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

    /// Whether the archetype includes none of the component IDs
    /// in the given array. If an empty array is given, the result
    /// is always `true`.
    pub fn contains_none_of<const N: usize>(&self, component_ids: &[ComponentID; N]) -> bool {
        !(0..N).any(|idx| self.contains_component_id(component_ids[idx]))
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

    fn new_from_sorted_component_ids(component_ids: &[ComponentID]) -> Result<Self> {
        if !component_ids.is_empty() {
            // Verify that no component is represented multiple times
            let duplicates_exist = (1..component_ids.len())
                .any(|idx| component_ids[idx..].contains(&component_ids[idx - 1]));
            if duplicates_exist {
                bail!("Duplicate component ID when constructing archetype");
            }
        }

        Ok(Self::new_from_sorted_component_ids_unchecked(component_ids))
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
        ArchetypeID(hasher.finish() as u32)
    }
}

impl PartialEq for Archetype {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<A> ArchetypeComponents<A>
where
    A: ComponentArray,
{
    fn new(archetype: Archetype, component_arrays: Vec<A>, component_count: usize) -> Self {
        let component_index_map = KeyIndexMapper::with_hasher_and_keys(
            nohash_hasher::BuildNoHashHasher::default(),
            component_arrays.iter().map(A::component_id),
        );

        Self {
            archetype,
            component_index_map,
            component_arrays,
            component_count,
        }
    }

    /// Convert the given array of component arrays into an
    /// [`ArchetypeComponents`].
    ///
    /// # Errors
    /// Returns an error if:
    /// - The same component type occurs more than once in the array.
    /// - The component arrays do not have the same component count.
    pub fn try_from_array_of_component_arrays<const N: usize>(
        component_arrays: [A; N],
    ) -> Result<Self> {
        // Find the number of component instances and check that this is the
        // same for all the component types
        let mut component_iter = component_arrays.iter();
        let component_count = component_iter.next().map_or(0, A::component_count);
        if component_iter.any(|array| array.component_count() != component_count) {
            bail!("The number of component instances differs between component types");
        }

        let mut component_ids = [ComponentID::dummy(); N];

        // Populate array of component IDs
        component_ids
            .iter_mut()
            .zip(component_arrays.iter())
            .for_each(|(id, array)| *id = array.component_id());

        // Make sure components IDs are sorted before determining archetype
        component_ids.sort();

        let archetype = Archetype::new_from_sorted_component_id_arr(component_ids)?;

        Ok(Self::new(
            archetype,
            component_arrays.to_vec(),
            component_count,
        ))
    }

    /// Convert the given [`Vec`] of component arrays into an
    /// [`ArchetypeComponents`].
    ///
    /// # Errors
    /// Returns an error if:
    /// - The same component type occurs more than once in the array.
    /// - The component arrays do not have the same component count.
    pub fn try_from_vec_of_component_arrays(component_arrays: Vec<A>) -> Result<Self> {
        // Find the number of component instances and check that this is the
        // same for all the component types
        let mut component_iter = component_arrays.iter();
        let component_count = component_iter.next().map_or(0, A::component_count);
        if component_iter.any(|array| array.component_count() != component_count) {
            bail!("The number of component instances differs between component types");
        }

        let mut component_ids: Vec<_> = component_arrays.iter().map(A::component_id).collect();

        // Make sure components IDs are sorted before determining archetype
        component_ids.sort();

        let archetype = Archetype::new_from_sorted_component_ids(&component_ids)?;

        Ok(Self::new(archetype, component_arrays, component_count))
    }

    /// Creates a new empty [`ArchetypeComponents`] value.
    pub fn empty() -> Self {
        Self {
            archetype: archetype_of!(),
            component_index_map: NoHashKeyIndexMapper::default(),
            component_arrays: Vec::new(),
            component_count: 0,
        }
    }

    /// Returns the archetype corresponding to the set of
    /// contained component types.
    pub fn archetype(&self) -> &Archetype {
        &self.archetype
    }

    /// Returns the number of contained component types.
    pub fn n_component_types(&self) -> usize {
        self.archetype.n_components()
    }

    /// Whether there is a contained component of type `C`.
    pub fn has_component_type<C: Component>(&self) -> bool {
        self.component_index_map.contains_key(C::component_id())
    }

    /// Returns the number of instances of each contained
    /// component type.
    pub fn component_count(&self) -> usize {
        self.component_count
    }

    /// Returns an iterator over the contained component IDs.
    pub fn component_ids(&self) -> impl Iterator<Item = ComponentID> {
        self.component_index_map.key_at_each_idx()
    }

    /// Returns a slice with all the contained instances of
    /// component type `C`.
    ///
    /// # Panics
    /// If none of the contained components are of type `C`.
    pub fn components_of_type<C: Component>(&self) -> &[C] {
        let component_idx = self
            .component_index_map
            .get(C::component_id())
            .expect("Requested invalid component");

        self.component_arrays[component_idx]
            .view()
            .component_instances()
    }

    /// Returns an iterator with the number of items equal to
    /// [`component_count`]. If `C` is the type of a contained component, each
    /// item will be a [`Some`] holding a reference to a different component
    /// instance. Otherwise, each item will be a [`None`].
    pub fn get_option_iter_for_component_of_type<C: Component>(
        &self,
    ) -> Box<dyn Iterator<Item = Option<&C>> + '_> {
        if let Some(component_idx) = self.component_index_map.get(C::component_id()) {
            Box::new(
                self.component_arrays[component_idx]
                    .view()
                    .component_instances()
                    .iter()
                    .map(Some),
            )
        } else {
            Box::new(iter::repeat_n(None, self.component_count()))
        }
    }

    /// Includes the given array of component instances of a new
    /// type in the set of contained components. Note that this
    /// changes the corresponding archetype.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The type of the given component instances is already present.
    /// - The number of component instances differs between the new and the
    ///   existing component types.
    pub fn add_new_component_type(&mut self, component_array: A) -> Result<()> {
        self.add_new_component_type_without_updating_archetype(component_array)?;
        self.archetype = Self::find_archetype(&self.component_arrays);
        Ok(())
    }

    /// Includes each given array of component instance in the
    /// set of contained components, overwriting the existing
    /// instances if the same component type is already present.
    /// Note that the corresponding archetype will change if any
    /// new component types are added.
    ///
    /// # Errors
    /// Returns an error if the number of component instances
    /// differs between the new and the existing component types.
    ///
    /// Even if including some of the component arrays fails, all
    /// the other component arrays will still be included.
    pub fn add_or_overwrite_component_types(
        &mut self,
        component_arrays: impl IntoIterator<Item = A>,
    ) -> Result<()> {
        let mut must_update_archetype = false;
        let mut result = Ok(());

        for component_array in component_arrays {
            match self.add_or_overwrite_component_type_without_updating_archetype(component_array) {
                Ok(InclusionResult::Added) => {
                    must_update_archetype = true;
                }
                Ok(InclusionResult::Overwritten) => {}
                Err(error) => {
                    result = Err(error);
                }
            }
        }

        if must_update_archetype {
            self.archetype = Self::find_archetype(&self.component_arrays);
        }

        result
    }

    /// Removes all the instances for the component type with
    /// the given ID. Note that this changes the corresponding
    /// archetype.
    ///
    /// # Errors
    /// Returns an error if the component type to remove is
    /// not present.
    pub fn remove_component_type_with_id(&mut self, component_id: ComponentID) -> Result<()> {
        let idx = self
            .component_index_map
            .try_swap_remove_key(component_id)
            .map_err(|_err| anyhow!("Tried to remove missing component type"))?;

        self.component_arrays.swap_remove(idx);

        // Update archetype
        self.archetype = Self::find_archetype(&self.component_arrays);

        Ok(())
    }

    /// Removes all the instances for all the component types with the given
    /// IDs. Note that this changes the corresponding archetype.
    ///
    /// # Errors
    /// Returns an error if any of the component types to remove is not present.
    pub fn remove_component_types_with_ids(
        &mut self,
        component_ids: impl IntoIterator<Item = ComponentID>,
    ) -> Result<()> {
        for component_id in component_ids {
            let idx = self
                .component_index_map
                .try_swap_remove_key(component_id)
                .map_err(|_err| anyhow!("Tried to remove missing component type"))?;

            self.component_arrays.swap_remove(idx);
        }

        // Update archetype
        self.archetype = Self::find_archetype(&self.component_arrays);

        Ok(())
    }

    /// Consumes this container and returns an iterator over
    /// all the contained [`ComponentArray`]s.
    pub fn into_component_arrays(self) -> impl IntoIterator<Item = A> {
        self.component_arrays.into_iter()
    }

    /// Converts all the contained component arrays into
    /// [`ComponentStorage`]s and returns them in a new
    /// [`ArchetypeComponentStorage`].
    pub fn into_storage(self) -> ArchetypeComponentStorage {
        let Self {
            archetype,
            component_index_map,
            component_arrays,
            component_count,
        } = self;

        ArchetypeComponents {
            archetype,
            component_index_map,
            component_arrays: component_arrays
                .into_iter()
                .map(ComponentArray::into_storage)
                .collect(),
            component_count,
        }
    }

    fn add_new_component_type_without_updating_archetype(
        &mut self,
        component_array: A,
    ) -> Result<()> {
        let component_count = component_array.component_count();

        if !self.component_arrays.is_empty() && (component_count != self.component_count()) {
            bail!("Inconsistent number of component instances in added component data");
        }

        self.component_index_map
            .try_push_key(component_array.component_id())
            .map_err(|_err| anyhow!("Tried to add component type that already exists"))?;

        self.component_arrays.push(component_array);

        self.component_count = component_count;

        Ok(())
    }

    fn add_or_overwrite_component_type_without_updating_archetype(
        &mut self,
        component_array: A,
    ) -> Result<InclusionResult> {
        let component_count = component_array.component_count();

        if !self.component_arrays.is_empty() && (component_count != self.component_count()) {
            bail!("Inconsistent number of component instances in added component data");
        }

        match self
            .component_index_map
            .try_push_key(component_array.component_id())
        {
            Ok(_) => {
                self.component_arrays.push(component_array);
                self.component_count = component_count;
                Ok(InclusionResult::Added)
            }
            Err(idx) => {
                self.component_arrays[idx] = component_array;
                Ok(InclusionResult::Overwritten)
            }
        }
    }

    fn find_archetype(component_arrays: &[A]) -> Archetype {
        let mut component_ids: Vec<_> = component_arrays.iter().map(A::component_id).collect();
        component_ids.sort();
        Archetype::new_from_sorted_component_ids_unchecked(&component_ids)
    }
}

impl<A> CanHaveSingleInstance for ArchetypeComponents<A>
where
    A: ComponentArray,
{
    fn instance_count(&self) -> usize {
        self.component_count()
    }
}

impl<A, const N: usize> TryFrom<[A; N]> for ArchetypeComponents<A>
where
    A: ComponentArray,
{
    type Error = anyhow::Error;

    fn try_from(component_arrays: [A; N]) -> Result<Self> {
        Self::try_from_array_of_component_arrays(component_arrays)
    }
}

impl<A> TryFrom<Vec<A>> for ArchetypeComponents<A>
where
    A: ComponentArray,
{
    type Error = anyhow::Error;

    fn try_from(component_arrays: Vec<A>) -> Result<Self> {
        Self::try_from_vec_of_component_arrays(component_arrays)
    }
}

impl<A> SingleInstance<ArchetypeComponents<A>>
where
    A: ComponentArray,
{
    /// Converts the given array of [`SingleInstance`] wrapped
    /// [`ComponentArray`]s into a `SingleInstance` wrapped
    /// [`ArchetypeComponents`].
    ///
    /// # Errors
    /// Returns an error if:
    /// - The given array is empty.
    /// - The same component type occurs more than once in the array.
    pub fn try_from_array_of_single_instances<const N: usize>(
        component_arrays: [SingleInstance<A>; N],
    ) -> Result<Self> {
        if component_arrays.is_empty() {
            bail!("Tried to create empty single instance `ArchetypeComponents`");
        }

        let mut component_ids = [ComponentID::dummy(); N];

        // Populate array of component IDs
        component_ids
            .iter_mut()
            .zip(component_arrays.iter())
            .for_each(|(id, array)| *id = array.component_id());

        // Make sure components IDs are sorted before determining archetype
        component_ids.sort();

        let archetype = Archetype::new_from_sorted_component_id_arr(component_ids)?;

        Ok(SingleInstance::new_unchecked(ArchetypeComponents::new(
            archetype,
            component_arrays
                .into_iter()
                .map(SingleInstance::into_inner)
                .collect(),
            1,
        )))
    }

    /// Converts the given [`Vec`] of [`SingleInstance`] wrapped
    /// [`ComponentArray`]s into a `SingleInstance` wrapped
    /// [`ArchetypeComponents`].
    ///
    /// # Errors
    /// Returns an error if:
    /// - The given vector is empty.
    /// - The same component type occurs more than once in the vector.
    pub fn try_from_vec_of_single_instances(
        component_arrays: Vec<SingleInstance<A>>,
    ) -> Result<Self> {
        if component_arrays.is_empty() {
            bail!("Tried to create empty single instance `ArchetypeComponents`");
        }

        let mut component_ids: Vec<_> = component_arrays
            .iter()
            .map(|array| array.component_id())
            .collect();

        // Make sure components IDs are sorted before determining archetype
        component_ids.sort();

        let archetype = Archetype::new_from_sorted_component_ids(&component_ids)?;

        Ok(SingleInstance::new_unchecked(ArchetypeComponents::new(
            archetype,
            component_arrays
                .into_iter()
                .map(SingleInstance::into_inner)
                .collect(),
            1,
        )))
    }
}

impl<A, const N: usize> TryFrom<[SingleInstance<A>; N]> for SingleInstance<ArchetypeComponents<A>>
where
    A: ComponentArray,
{
    type Error = anyhow::Error;

    fn try_from(component_arrays: [SingleInstance<A>; N]) -> Result<Self> {
        Self::try_from_array_of_single_instances(component_arrays)
    }
}

impl<A> TryFrom<Vec<SingleInstance<A>>> for SingleInstance<ArchetypeComponents<A>>
where
    A: ComponentArray,
{
    type Error = anyhow::Error;

    fn try_from(component_arrays: Vec<SingleInstance<A>>) -> Result<Self> {
        Self::try_from_vec_of_single_instances(component_arrays)
    }
}

impl ArchetypeComponentStorage {
    /// Creates a new [`ArchetypeComponentStorage`] storing the data
    /// of the given value that may be convertible into an
    /// [`ArchetypeComponentView`].
    ///
    /// # Errors
    /// Returns an error if the conversion of the input value into
    /// [`ArchetypeComponentView`] fails.
    pub fn try_from_view<'a, E>(
        view: impl TryInto<ArchetypeComponentView<'a>, Error = E>,
    ) -> Result<Self>
    where
        E: Into<anyhow::Error>,
    {
        view.try_into()
            .map(|view| view.into_storage())
            .map_err(E::into)
    }

    /// Creates a new [`ArchetypeComponentStorage`] storing the data
    /// of the given value that is convertible into an
    /// [`ArchetypeComponentView`].
    pub fn from_view<'a, V>(view: V) -> Self
    where
        V: Into<ArchetypeComponentView<'a>>,
    {
        view.into().into_storage()
    }

    /// Creates a new [`SingleInstance`] wrapped [`ArchetypeComponentStorage`]
    /// storing the data of the given value that may be convertible into a
    /// `SingleInstance` wrapped [`ArchetypeComponentView`].
    ///
    /// # Errors
    /// Returns an error if the conversion of the input value into
    /// [`SingleInstance<ArchetypeComponentView>`] fails.
    pub fn try_from_single_instance_view<'a, E>(
        view: impl TryInto<SingleInstance<ArchetypeComponentView<'a>>, Error = E>,
    ) -> Result<SingleInstance<Self>>
    where
        E: Into<anyhow::Error>,
    {
        view.try_into()
            .map(|view| SingleInstance::new_unchecked(view.into_inner().into_storage()))
            .map_err(E::into)
    }

    /// Creates a new [`SingleInstance`] wrapped [`ArchetypeComponentStorage`]
    /// storing the data of the given value that is convertible into a
    /// `SingleInstance` wrapped [`ArchetypeComponentView`].
    pub fn from_single_instance_view<'a, V>(view: V) -> SingleInstance<Self>
    where
        V: Into<SingleInstance<ArchetypeComponentView<'a>>>,
    {
        SingleInstance::new_unchecked(view.into().into_inner().into_storage())
    }

    /// Creates an empty [`ComponentStorage`] for components
    /// of type `C`, with preallocated capacity for the same
    /// number of component instances as contained here.
    ///
    /// This is a useful starting point when we want to add a
    /// storage for a new component type.
    pub fn new_storage_with_capacity<C: Component>(&self) -> ComponentStorage {
        ComponentStorage::with_capacity::<C>(self.component_count)
    }
}

impl SingleInstance<ArchetypeComponentStorage> {
    /// Converts this single-instance storage into a storage containing copies
    /// of the instance component data for the given number of instances.
    ///
    /// # Panics
    /// If `n_instances` is zero.
    pub fn duplicate_instance(self, n_instances: usize) -> ArchetypeComponentStorage {
        let mut archetype_storage = self.into_inner();

        archetype_storage.component_arrays = archetype_storage
            .component_arrays
            .into_iter()
            .map(|storage| SingleInstance::new_unchecked(storage).duplicate_instance(n_instances))
            .collect();

        archetype_storage.component_count = n_instances;

        archetype_storage
    }

    /// Creates an [`ArchetypeComponentStorage`] containing both the given
    /// [`ArchetypeComponents`] as well as the components in this storage whose
    /// types are not among the given components. If there are multiple
    /// instances of each component type in the given `ArchetypeComponents`
    /// container, the component data in this storage will be duplicated to
    /// match the number of instances before merging.
    ///
    /// # Errors
    /// Returns an error if the conversion of `components` into
    /// `ArchetypeComponents` fails.
    pub fn combined_with<A, E>(
        self,
        components: impl TryInto<ArchetypeComponents<A>, Error = E>,
    ) -> Result<ArchetypeComponentStorage>
    where
        A: ComponentArray,
        E: Into<anyhow::Error>,
    {
        let components: ArchetypeComponents<A> = components.try_into().map_err(E::into)?;

        let mut duplicated_components = self.duplicate_instance(components.component_count());

        duplicated_components
            .add_or_overwrite_component_types(
                components
                    .into_component_arrays()
                    .into_iter()
                    .map(A::into_storage),
            )
            .unwrap();

        Ok(duplicated_components)
    }
}

impl<'a> From<ArchetypeComponentView<'a>> for ArchetypeComponentStorage {
    fn from(view: ArchetypeComponentView<'a>) -> Self {
        view.into_storage()
    }
}

impl ArchetypeTable {
    /// Takes an iterable of [`EntityID`]s and all the associated
    /// component data (as an [`ArchetypeComponents`]), initializes
    /// a table for the corresponding [`Archetype`] and inserts the
    /// given data, one row per entity.
    ///
    /// # Panics
    /// - If the number of entities differs from the number of instances of each
    ///   component type.
    /// - If any of the entity IDs are equal.
    pub(crate) fn new_with_entities(
        entity_ids: impl IntoIterator<Item = EntityID>,
        components: ArchetypeComponents<impl ComponentArray>,
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

    /// Whether the [`Entity`](crate::world::Entity) with the given [`EntityID`]
    /// is present in the table.
    pub fn has_entity(&self, entity_id: EntityID) -> bool {
        self.entity_index_mapper.contains_key(entity_id)
    }

    /// Returns an iterator over all [`Entity`]s whose components
    /// are stored in the table.
    pub fn all_entities(&self) -> impl Iterator<Item = Entity> {
        self.entity_index_mapper
            .key_at_each_idx()
            .map(|entity_id| Entity::new(entity_id, self.archetype().id()))
    }

    /// Takes an iterable of [`EntityID`]s and all the associated
    /// component data (as an [`ArchetypeComponents`]) and appends
    /// the given data to the table, one row per entity.
    ///
    /// # Panics
    /// - If the number of entities differs from the number of instances of each
    ///   component type.
    /// - If any of the given entity IDs are equal to a new or existing entity
    ///   ID.
    pub(crate) fn add_entities(
        &mut self,
        entity_ids: impl IntoIterator<Item = EntityID>,
        components: ArchetypeComponents<impl ComponentArray>,
    ) {
        let original_entity_count = self.entity_index_mapper.len();
        self.entity_index_mapper.push_keys(entity_ids);
        let added_entity_count = self.entity_index_mapper.len() - original_entity_count;
        assert_eq!(
            added_entity_count, components.component_count,
            "Number of components per component type differs from number of entities"
        );

        for array in components.into_component_arrays() {
            let storage_idx = self.component_index_map[&array.component_id()];
            self.component_storages[storage_idx]
                .write()
                .unwrap()
                .push_array(&array);
        }
    }

    /// Removes the entity with the given [`EntityID`] and all its data from the
    /// table.
    ///
    /// # Returns
    /// The removed component data in a
    /// [`SingleInstance<ArchetypeComponentStorage>`].
    ///
    /// # Errors
    /// Returns an error if the entity is not present in the table.
    pub(crate) fn remove_entity(
        &mut self,
        entity_id: EntityID,
    ) -> Result<SingleInstance<ArchetypeComponentStorage>> {
        if !self.has_entity(entity_id) {
            bail!("Entity to remove not present in archetype table");
        }
        // Remove the entity from the map and obtain the index
        // of the corresponing component data. We do a swap remove
        // in order to keep the index map consistent when we do a
        // swap remove of component data.
        let idx = self.entity_index_mapper.swap_remove_key(entity_id);

        // Perform an equivalent swap remove of the data at the index we found
        let removed_component_arrays = self
            .component_storages
            .iter()
            .map(|storage| storage.write().unwrap().swap_remove(idx).into_inner())
            .collect();

        Ok(SingleInstance::new_unchecked(ArchetypeComponents::new(
            self.archetype.clone(),
            removed_component_arrays,
            1,
        )))
    }

    /// Removes all entities and their data from the table.
    pub(crate) fn remove_all_entities(&mut self) {
        self.entity_index_mapper.clear();
        for storage in &self.component_storages {
            storage.write().unwrap().clear();
        }
    }

    /// Returns a reference to the component of the type specified
    /// by the `C` type parameter belonging to the entity with the
    /// given [`EntityID`]. If the entity is not present in the table
    /// or if it does not have the specified component, [`None`] is
    /// returned.
    ///
    /// # Concurrency
    /// The returned reference is wrapped in a [`ComponentStorageEntry`]
    /// that holds a read lock on the [`ComponentStorage`] where the
    /// component resides. The lock is released when the entry is
    /// dropped.
    pub fn get_component_for_entity<C: Component>(
        &self,
        entity_id: EntityID,
    ) -> Option<ComponentStorageEntry<'_, C>> {
        let component_idx = *self.component_index_map.get(&C::component_id())?;
        let entity_idx = self.entity_index_mapper.get(entity_id)?;
        Some(ComponentStorageEntry::new(
            self.component_storages[component_idx].read().unwrap(),
            entity_idx,
        ))
    }

    /// Returns a mutable reference to the component of the type
    /// specified by the `C` type parameter belonging to the entity
    /// with the given [`EntityID`]. If the entity is not present in
    /// the table or if it does not have the specified component,
    /// [`None`] is returned.
    ///
    /// # Concurrency
    /// The returned reference is wrapped in a [`ComponentStorageEntryMut`]
    /// that holds a write lock on the [`ComponentStorage`] where the
    /// component resides. The lock is released when the entry is
    /// dropped.
    pub fn get_component_for_entity_mut<C: Component>(
        &self,
        entity_id: EntityID,
    ) -> Option<ComponentStorageEntryMut<'_, C>> {
        let component_idx = *self.component_index_map.get(&C::component_id())?;
        let entity_idx = self.entity_index_mapper.get(entity_id)?;
        Some(ComponentStorageEntryMut::new(
            self.component_storages[component_idx].write().unwrap(),
            entity_idx,
        ))
    }

    /// Returns a [`TableEntityEntry`] that can be used to read the components
    /// of the [`Entity`](crate::world::Entity) with the given [`EntityID`].
    /// If the entity is not present in the table, [`None`] is returned.
    ///
    /// # Concurrency
    /// The returned `TableEntityEntry` holds locks to the component storages
    /// in the table until it is dropped. Before then, attempts to modify
    /// the component data will be blocked.
    pub fn get_entity(&self, entity_id: EntityID) -> Option<TableEntityEntry<'_>> {
        let entity_idx = self.entity_index_mapper.get(entity_id)?;
        Some(TableEntityEntry::new(
            &self.archetype,
            entity_idx,
            &self.component_index_map,
            self.component_storages
                .iter()
                .map(|storage| storage.read().unwrap())
                .collect(),
        ))
    }

    /// Returns a [`TableEntityEntry`] that can be used to read the components
    /// of the [`Entity`](crate::world::Entity) with the given [`EntityID`].
    ///
    /// # Panics
    /// If the entity is not present in the table.
    ///
    /// # Concurrency
    /// The returned `TableEntityEntry` holds locks to the component storages
    /// in the table until it is dropped. Before then, attempts to modify
    /// the component data will be blocked.
    pub fn entity(&self, entity_id: EntityID) -> TableEntityEntry<'_> {
        self.get_entity(entity_id)
            .expect("Entity not present in table")
    }

    /// Returns a [`TableEntityMutEntry`] that can be used to read and modify
    /// the components of the [`Entity`](crate::world::Entity) with the
    /// given [`EntityID`]. If the entity is not present in the table,
    /// [`None`] is returned.
    ///
    /// # Concurrency
    /// The returned `TableEntityMutEntry` holds locks to the component storages
    /// in the table until it is dropped. Before then, attempts to read
    /// or modify the component data will be blocked.
    pub fn get_entity_mut(&self, entity_id: EntityID) -> Option<TableEntityMutEntry<'_>> {
        let entity_idx = self.entity_index_mapper.get(entity_id)?;
        Some(TableEntityMutEntry::new(
            &self.archetype,
            entity_idx,
            &self.component_index_map,
            self.component_storages
                .iter()
                .map(|storage| storage.write().unwrap())
                .collect(),
        ))
    }

    /// Returns a [`TableEntityMutEntry`] that can be used to read and modify
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
    pub fn entity_mut(&self, entity_id: EntityID) -> TableEntityMutEntry<'_> {
        self.get_entity_mut(entity_id)
            .expect("Entity not present in table")
    }

    /// Returns a reference to the [`ComponentStorage`] for components
    /// with the given [`ComponentID`]. The storage is guarded by a
    /// [`RwLock`] that must be acquired before the storage can be
    /// accessed.
    ///
    /// # Errors
    /// Returns an error if the given ID does not represent one of the
    /// component types present in the table.
    pub fn get_component_storage(
        &self,
        component_id: ComponentID,
    ) -> Result<&RwLock<ComponentStorage>> {
        let idx = *self
            .component_index_map
            .get(&component_id)
            .ok_or_else(|| anyhow!("Component not present in archetype table"))?;
        Ok(&self.component_storages[idx])
    }

    /// Returns a reference to the [`ComponentStorage`] for components
    /// with the given [`ComponentID`]. The storage is guarded by a
    /// [`RwLock`] that must be acquired before the storage can be
    /// accessed.
    ///
    /// # Panice
    /// If the given ID does not represent one of the component types
    /// present in the table.
    pub fn component_storage(&self, component_id: ComponentID) -> &RwLock<ComponentStorage> {
        self.get_component_storage(component_id).unwrap()
    }

    fn new_with_entity_index_mapper(
        entity_index_mapper: KeyIndexMapper<EntityID>,
        components: ArchetypeComponents<impl ComponentArray>,
    ) -> Self {
        let ArchetypeComponents {
            archetype,
            component_index_map,
            component_arrays,
            component_count,
        } = components;

        assert_eq!(
            entity_index_mapper.len(),
            component_count,
            "Number of components per component type differs from number of entities"
        );
        Self {
            archetype,
            entity_index_mapper,
            component_index_map: component_index_map.into_map(),
            // Create storages with component data for the provided entity
            component_storages: component_arrays
                .into_iter()
                .map(|array| RwLock::new(array.into_storage()))
                .collect(),
        }
    }
}

impl<'a> TableEntityEntry<'a> {
    fn new(
        archetype: &'a Archetype,
        entity_idx: usize,
        component_index_map: &'a NoHashMap<ComponentID, usize>,
        components: Vec<RwLockReadGuard<'a, ComponentStorage>>,
    ) -> Self {
        Self {
            info: TableEntityEntryInfo {
                archetype,
                entity_idx,
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
    ///
    /// # Panics
    /// If `C` is a zero-sized type.
    pub fn get_component<C: Component>(&self) -> Option<&C> {
        let component_idx = *self.info.component_index_map.get(&C::component_id())?;
        Some(&self.components[component_idx].slice()[self.info.entity_idx])
    }

    /// Returns a reference to the component specified by the
    /// type parameter `C`.
    ///
    /// # Panics
    /// - If the entity does not have the specified component.
    /// - If `C` is a zero-sized type.
    pub fn component<C: Component>(&self) -> &C {
        self.get_component::<C>()
            .expect("Requested invalid component")
    }
}

impl<'a> TableEntityMutEntry<'a> {
    fn new(
        archetype: &'a Archetype,
        entity_idx: usize,
        component_index_map: &'a NoHashMap<ComponentID, usize>,
        components: Vec<RwLockWriteGuard<'a, ComponentStorage>>,
    ) -> Self {
        Self {
            info: TableEntityEntryInfo {
                archetype,
                entity_idx,
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
    ///
    /// # Panics
    /// If `C` is a zero-sized type.
    pub fn get_component<C: Component>(&mut self) -> Option<&mut C> {
        let component_idx = *self.info.component_index_map.get(&C::component_id())?;
        Some(&mut self.components[component_idx].slice_mut()[self.info.entity_idx])
    }

    /// Returns a mutable reference to the component specified
    /// by the type parameter `C`.
    ///
    /// # Panics
    /// - If the entity does not have the specified component.
    /// - If `C` is a zero-sized type.
    pub fn component<C: Component>(&mut self) -> &mut C {
        self.get_component::<C>()
            .expect("Requested invalid component")
    }
}

impl TableEntityEntryInfo<'_> {
    fn n_components(&self) -> usize {
        self.archetype.n_components()
    }

    fn has_component<C: Component>(&self) -> bool {
        self.archetype.contains_component_id(C::component_id())
    }
}

impl<'a, C> ComponentStorageEntry<'a, C>
where
    C: Component,
{
    fn new(storage: RwLockReadGuard<'a, ComponentStorage>, entity_idx: usize) -> Self {
        assert!(entity_idx < storage.component_count());
        Self {
            entity_idx,
            storage,
            _phantom: PhantomData,
        }
    }

    /// Returns an immutable reference to the component instance.
    ///
    /// # Panics
    /// If `C` is a zero-sized type.
    pub fn access(&self) -> &C {
        &self.storage.slice::<C>()[self.entity_idx]
    }
}

impl<'a, C> ComponentStorageEntryMut<'a, C>
where
    C: Component,
{
    fn new(storage: RwLockWriteGuard<'a, ComponentStorage>, entity_idx: usize) -> Self {
        assert!(entity_idx < storage.component_count());
        Self {
            entity_idx,
            storage,
            _phantom: PhantomData,
        }
    }

    /// Returns a mutable reference to the component instance.
    pub fn access(&mut self) -> &mut C {
        &mut self.storage.slice_mut::<C>()[self.entity_idx]
    }
}

/// Macro for implementing [`From<C>`] or [`TryFrom<C>`] for
/// [`ArchetypeComponentView`], where `C` respectively is a single
/// [`Component`] reference/slice or tuple of references/slices.
macro_rules! impl_archetype_conversion {
    ($c:ident) => {
        // For a single instance of a single component type
        impl<'a, $c> From<&'a $c> for ArchetypeComponentView<'a>
        where
            $c: 'a + Component,
        {
            fn from(component: &'a $c) -> Self {
                [component.persistent_view()].try_into().unwrap()
            }
        }
        impl<'a, $c> From<&'a $c> for SingleInstance<ArchetypeComponentView<'a>>
        where
            $c: 'a + Component,
        {
            fn from(component: &'a $c) -> Self {
                SingleInstance::new_unchecked(component.try_into().unwrap())
            }
        }
        // For a a slice of instances of a single component type
        impl<'a, $c> From<&'a [$c]> for ArchetypeComponentView<'a>
        where
            $c: 'a + Component,
        {
            fn from(component_slice: &'a [$c]) -> Self {
                [component_slice.persistent_view()].try_into().unwrap()
            }
        }
        // For a fixed length slice of instances of a single component type
        impl<'a, const N: usize, $c> From<&'a [$c; N]> for ArchetypeComponentView<'a>
        where
            $c: 'a + Component,
        {
            fn from(component_slice: &'a [$c; N]) -> Self {
                [component_slice.persistent_view()].try_into().unwrap()
            }
        }
    };
    (($($c:ident),*)) => {
        // For single instances of multiple component types
        impl<'a, $($c),*> TryFrom<($(&'a $c),*)> for ArchetypeComponentView<'a>
        where
        $($c: 'a + Component,)*
        {
            type Error = anyhow::Error;
            #[allow(non_snake_case)]
            fn try_from(($(paste! { [<component_ $c>] }),*): ($(&'a $c),*)) -> Result<Self> {
                [$(paste! { [<component_ $c>] }.persistent_view()),*].try_into()
            }
        }
        impl<'a, $($c),*> TryFrom<($(&'a $c),*)> for SingleInstance<ArchetypeComponentView<'a>>
        where
        $($c: 'a + Component,)*
        {
            type Error = anyhow::Error;
            #[allow(non_snake_case)]
            fn try_from(components: ($(&'a $c),*)) -> Result<Self> {
                components.try_into().map(SingleInstance::new_unchecked)
            }
        }
        // For slices of instances of multiple component types
        impl<'a, $($c),*> TryFrom<($(&'a [$c]),*)> for ArchetypeComponentView<'a>
        where
        $($c: 'a + Component,)*
        {
            type Error = anyhow::Error;
            #[allow(non_snake_case)]
            fn try_from(($(paste! { [<component_slice_ $c>] }),*): ($(&'a [$c]),*)) -> Result<Self> {
                [$(paste! { [<component_slice_ $c>] }.persistent_view()),*].try_into()
            }
        }
        // For fixed size slices of instances of multiple component types
        impl<'a, const N: usize, $($c),*> TryFrom<($(&'a [$c; N]),*)> for ArchetypeComponentView<'a>
        where
        $($c: 'a + Component,)*
        {
            type Error = anyhow::Error;
            #[allow(non_snake_case)]
            fn try_from(($(paste! { [<component_slice_ $c>] }),*): ($(&'a [$c; N]),*)) -> Result<Self> {
                [$(paste! { [<component_slice_ $c>] }.persistent_view()),*].try_into()
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
impl_archetype_conversion!((C1, C2, C3, C4, C5, C6, C7, C8, C9));
impl_archetype_conversion!((C1, C2, C3, C4, C5, C6, C7, C8, C9, C10));
impl_archetype_conversion!((C1, C2, C3, C4, C5, C6, C7, C8, C9, C10, C11));
impl_archetype_conversion!((C1, C2, C3, C4, C5, C6, C7, C8, C9, C10, C11, C12));
impl_archetype_conversion!((C1, C2, C3, C4, C5, C6, C7, C8, C9, C10, C11, C12, C13));
impl_archetype_conversion!((C1, C2, C3, C4, C5, C6, C7, C8, C9, C10, C11, C12, C13, C14));
impl_archetype_conversion!((
    C1, C2, C3, C4, C5, C6, C7, C8, C9, C10, C11, C12, C13, C14, C15
));
impl_archetype_conversion!((
    C1, C2, C3, C4, C5, C6, C7, C8, C9, C10, C11, C12, C13, C14, C15, C16
));

#[cfg(test)]
mod tests {
    use super::{
        super::{Component, archetype_of},
        *,
    };
    use crate::component::ComponentInstance;
    use bytemuck::{Pod, Zeroable};
    use std::collections::HashSet;

    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, PartialEq, Zeroable, Pod, Component)]
    struct Marked;

    #[repr(transparent)]
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
    const BYTE2: Byte = Byte(11);
    const POS: Position = Position(-9.8, 12.5, 7.3);
    const POS2: Position = Position(-0.01, 6.6, 22.3);
    const RECT: Rectangle = Rectangle {
        center: [2.5, 2.0],
        dimensions: [12.3, 8.9],
    };

    #[test]
    fn archetype_macro_works() {
        let archetype = Archetype::new_from_component_id_arr([
            Byte::component_id(),
            Position::component_id(),
            Rectangle::component_id(),
        ])
        .unwrap();
        let archetype_from_macro = archetype_of!(Byte, Position, Rectangle);
        assert_eq!(archetype, archetype_from_macro);
    }

    #[test]
    fn larger_archetypes_contain_smaller_archetypes() {
        let with_all_components = archetype_of!(Byte, Position, Rectangle);
        let without_byte = archetype_of!(Rectangle, Position);
        let without_position = archetype_of!(Byte, Rectangle);
        let empty = archetype_of!();

        assert!(with_all_components.contains(&with_all_components));
        assert!(with_all_components.contains(&without_byte));
        assert!(!without_byte.contains(&with_all_components));
        assert!(!without_byte.contains(&without_position));
        assert!(with_all_components.contains(&empty));
        assert!(!empty.contains(&with_all_components));
    }

    #[test]
    fn archetypes_do_not_contain_other_components() {
        let without_position = archetype_of!(Byte, Rectangle);
        let without_position_and_byte = archetype_of!(Rectangle);
        let empty = archetype_of!();

        assert!(without_position.contains_none_of(&[Position::component_id()]));
        assert!(
            without_position_and_byte
                .contains_none_of(&[Position::component_id(), Byte::component_id()])
        );
        assert!(empty.contains_none_of(&[
            Position::component_id(),
            Byte::component_id(),
            Rectangle::component_id()
        ]));

        assert!(!without_position.contains_none_of(&[Byte::component_id()]));
        assert!(!without_position_and_byte.contains_none_of(&[Rectangle::component_id()]));

        assert!(without_position.contains_none_of(&[]));
        assert!(without_position_and_byte.contains_none_of(&[]));
        assert!(empty.contains_none_of(&[]));
    }

    #[test]
    #[should_panic]
    fn conversion_of_two_comp_array_with_two_equal_comps_to_archetype_view_fails() {
        let _: ArchetypeComponentView<'_> = [(&BYTE).view(), (&BYTE).view()].try_into().unwrap();
    }

    #[test]
    #[should_panic]
    fn conversion_of_two_comp_array_with_two_equal_comp_slices_to_archetype_view_fails() {
        let _: ArchetypeComponentView<'_> = [(&[BYTE, BYTE]).view(), (&[BYTE, BYTE]).view()]
            .try_into()
            .unwrap();
    }

    #[test]
    #[should_panic]
    fn conversion_of_three_comp_array_with_two_equal_comps_to_archetype_view_fails() {
        let _: ArchetypeComponentView<'_> = [(&BYTE).view(), (&POS).view(), (&BYTE).view()]
            .try_into()
            .unwrap();
    }

    #[test]
    #[should_panic]
    fn conversion_of_three_comp_array_with_two_equal_comp_slices_to_archetype_view_fails() {
        let _: ArchetypeComponentView<'_> = [
            (&[BYTE, BYTE]).view(),
            (&[POS, POS]).view(),
            (&[BYTE, BYTE]).view(),
        ]
        .try_into()
        .unwrap();
    }

    #[test]
    fn creating_empty_archetype_view_works() {
        let view = ArchetypeComponentView::empty();
        assert_eq!(view.archetype(), &archetype_of!());
        assert_eq!(view.n_component_types(), 0);
        assert_eq!(view.component_count(), 0);
    }

    #[test]
    #[should_panic]
    fn converting_to_empty_single_instance_archetype_view_fails() {
        SingleInstance::<ArchetypeComponentView<'_>>::try_from_array_of_single_instances([])
            .unwrap();
    }

    #[test]
    fn valid_conversions_of_comp_view_arrays_to_archetype_views_succeed() {
        let view: ArchetypeComponentView<'_> = [].try_into().unwrap();
        assert_eq!(view.archetype(), &archetype_of!());
        assert_eq!(view.n_component_types(), 0);
        assert_eq!(view.component_count(), 0);

        let view: ArchetypeComponentView<'_> = [(&Marked).view()].try_into().unwrap();
        assert_eq!(view.archetype(), &archetype_of!(Marked));
        assert_eq!(view.n_component_types(), 1);
        assert_eq!(view.component_count(), 1);
        assert!(view.has_component_type::<Marked>());

        let view =
            SingleInstance::<ArchetypeComponentView<'_>>::try_from_array_of_single_instances([
                (&Marked).single_instance_view(),
            ])
            .unwrap();
        assert_eq!(view.archetype(), &archetype_of!(Marked));
        assert_eq!(view.n_component_types(), 1);
        assert_eq!(view.component_count(), 1);
        assert!(view.has_component_type::<Marked>());

        let view: ArchetypeComponentView<'_> = [(&BYTE).view()].try_into().unwrap();
        assert_eq!(view.archetype(), &archetype_of!(Byte));
        assert_eq!(view.n_component_types(), 1);
        assert_eq!(view.component_count(), 1);
        assert!(view.has_component_type::<Byte>());
        assert_eq!(view.components_of_type::<Byte>(), &[BYTE]);

        let view =
            SingleInstance::<ArchetypeComponentView<'_>>::try_from_array_of_single_instances([
                (&BYTE).single_instance_view(),
            ])
            .unwrap();
        assert_eq!(view.archetype(), &archetype_of!(Byte));
        assert_eq!(view.n_component_types(), 1);
        assert_eq!(view.component_count(), 1);
        assert!(view.has_component_type::<Byte>());
        assert_eq!(view.components_of_type::<Byte>(), &[BYTE]);

        let view: ArchetypeComponentView<'_> = [(&BYTE).view(), (&POS).view()].try_into().unwrap();
        assert_eq!(view.archetype(), &archetype_of!(Byte, Position));
        assert_eq!(view.n_component_types(), 2);
        assert_eq!(view.component_count(), 1);
        assert!(view.has_component_type::<Byte>());
        assert!(view.has_component_type::<Position>());
        assert_eq!(view.components_of_type::<Byte>(), &[BYTE]);
        assert_eq!(view.components_of_type::<Position>(), &[POS]);

        let view =
            SingleInstance::<ArchetypeComponentView<'_>>::try_from_array_of_single_instances([
                (&BYTE).single_instance_view(),
                (&POS).single_instance_view(),
            ])
            .unwrap();
        assert_eq!(view.archetype(), &archetype_of!(Byte, Position));
        assert_eq!(view.n_component_types(), 2);
        assert_eq!(view.component_count(), 1);
        assert!(view.has_component_type::<Byte>());
        assert!(view.has_component_type::<Position>());
        assert_eq!(view.components_of_type::<Byte>(), &[BYTE]);
        assert_eq!(view.components_of_type::<Position>(), &[POS]);

        let view: ArchetypeComponentView<'_> = [(&BYTE).view(), (&POS).view(), (&RECT).view()]
            .try_into()
            .unwrap();
        assert_eq!(view.archetype(), &archetype_of!(Byte, Position, Rectangle));
        assert_eq!(view.n_component_types(), 3);
        assert_eq!(view.component_count(), 1);
        assert!(view.has_component_type::<Byte>());
        assert!(view.has_component_type::<Position>());
        assert!(view.has_component_type::<Rectangle>());
        assert_eq!(view.components_of_type::<Byte>(), &[BYTE]);
        assert_eq!(view.components_of_type::<Position>(), &[POS]);
        assert_eq!(view.components_of_type::<Rectangle>(), &[RECT]);

        let view =
            SingleInstance::<ArchetypeComponentView<'_>>::try_from_array_of_single_instances([
                (&BYTE).single_instance_view(),
                (&POS).single_instance_view(),
                (&RECT).single_instance_view(),
            ])
            .unwrap();
        assert_eq!(view.archetype(), &archetype_of!(Byte, Position, Rectangle));
        assert_eq!(view.n_component_types(), 3);
        assert_eq!(view.component_count(), 1);
        assert!(view.has_component_type::<Byte>());
        assert!(view.has_component_type::<Position>());
        assert!(view.has_component_type::<Rectangle>());
        assert_eq!(view.components_of_type::<Byte>(), &[BYTE]);
        assert_eq!(view.components_of_type::<Position>(), &[POS]);
        assert_eq!(view.components_of_type::<Rectangle>(), &[RECT]);
    }

    #[test]
    fn valid_conversions_of_comp_slice_view_arrays_to_archetype_views_succeed() {
        let view: ArchetypeComponentView<'_> = [(&[Marked]).view()].try_into().unwrap();
        assert_eq!(view.archetype(), &archetype_of!(Marked));
        assert_eq!(view.n_component_types(), 1);
        assert_eq!(view.component_count(), 1);
        assert!(view.has_component_type::<Marked>());

        let view: ArchetypeComponentView<'_> = [(&[Marked, Marked]).view()].try_into().unwrap();
        assert_eq!(view.archetype(), &archetype_of!(Marked));
        assert_eq!(view.n_component_types(), 1);
        assert_eq!(view.component_count(), 2);
        assert!(view.has_component_type::<Marked>());

        let view: ArchetypeComponentView<'_> = [(&[BYTE]).view()].try_into().unwrap();
        assert_eq!(view.archetype(), &archetype_of!(Byte));
        assert_eq!(view.n_component_types(), 1);
        assert_eq!(view.component_count(), 1);
        assert!(view.has_component_type::<Byte>());
        assert_eq!(view.components_of_type::<Byte>(), &[BYTE]);

        let view: ArchetypeComponentView<'_> = [(&[BYTE, BYTE2]).view()].try_into().unwrap();
        assert_eq!(view.archetype(), &archetype_of!(Byte));
        assert_eq!(view.n_component_types(), 1);
        assert_eq!(view.component_count(), 2);
        assert!(view.has_component_type::<Byte>());
        assert_eq!(view.components_of_type::<Byte>(), &[BYTE, BYTE2]);

        let view: ArchetypeComponentView<'_> =
            [(&[BYTE]).view(), (&[POS]).view()].try_into().unwrap();
        assert_eq!(view.archetype(), &archetype_of!(Byte, Position));
        assert_eq!(view.n_component_types(), 2);
        assert_eq!(view.component_count(), 1);
        assert!(view.has_component_type::<Byte>());
        assert!(view.has_component_type::<Position>());
        assert_eq!(view.components_of_type::<Byte>(), &[BYTE]);
        assert_eq!(view.components_of_type::<Position>(), &[POS]);

        let view: ArchetypeComponentView<'_> = [(&[BYTE, BYTE2]).view(), (&[POS, POS2]).view()]
            .try_into()
            .unwrap();
        assert_eq!(view.archetype(), &archetype_of!(Byte, Position));
        assert_eq!(view.n_component_types(), 2);
        assert_eq!(view.component_count(), 2);
        assert!(view.has_component_type::<Byte>());
        assert!(view.has_component_type::<Position>());
        assert_eq!(view.components_of_type::<Byte>(), &[BYTE, BYTE2]);
        assert_eq!(view.components_of_type::<Position>(), &[POS, POS2]);
    }

    #[test]
    fn valid_conversions_of_comp_slice_view_vecs_to_archetype_views_succeed() {
        let view: ArchetypeComponentView<'_> = vec![(&[Marked]).view()].try_into().unwrap();
        assert_eq!(view.archetype(), &archetype_of!(Marked));
        assert_eq!(view.n_component_types(), 1);
        assert_eq!(view.component_count(), 1);
        assert!(view.has_component_type::<Marked>());

        let view: ArchetypeComponentView<'_> = vec![(&[Marked, Marked]).view()].try_into().unwrap();
        assert_eq!(view.archetype(), &archetype_of!(Marked));
        assert_eq!(view.n_component_types(), 1);
        assert_eq!(view.component_count(), 2);
        assert!(view.has_component_type::<Marked>());

        let view: ArchetypeComponentView<'_> = vec![(&[BYTE]).view()].try_into().unwrap();
        assert_eq!(view.archetype(), &archetype_of!(Byte));
        assert_eq!(view.n_component_types(), 1);
        assert_eq!(view.component_count(), 1);
        assert!(view.has_component_type::<Byte>());
        assert_eq!(view.components_of_type::<Byte>(), &[BYTE]);

        let view: ArchetypeComponentView<'_> = vec![(&[BYTE, BYTE2]).view()].try_into().unwrap();
        assert_eq!(view.archetype(), &archetype_of!(Byte));
        assert_eq!(view.n_component_types(), 1);
        assert_eq!(view.component_count(), 2);
        assert!(view.has_component_type::<Byte>());
        assert_eq!(view.components_of_type::<Byte>(), &[BYTE, BYTE2]);

        let view: ArchetypeComponentView<'_> =
            vec![(&[BYTE]).view(), (&[POS]).view()].try_into().unwrap();
        assert_eq!(view.archetype(), &archetype_of!(Byte, Position));
        assert_eq!(view.n_component_types(), 2);
        assert_eq!(view.component_count(), 1);
        assert!(view.has_component_type::<Byte>());
        assert!(view.has_component_type::<Position>());
        assert_eq!(view.components_of_type::<Byte>(), &[BYTE]);
        assert_eq!(view.components_of_type::<Position>(), &[POS]);

        let view: ArchetypeComponentView<'_> = vec![(&[BYTE, BYTE2]).view(), (&[POS, POS2]).view()]
            .try_into()
            .unwrap();
        assert_eq!(view.archetype(), &archetype_of!(Byte, Position));
        assert_eq!(view.n_component_types(), 2);
        assert_eq!(view.component_count(), 2);
        assert!(view.has_component_type::<Byte>());
        assert!(view.has_component_type::<Position>());
        assert_eq!(view.components_of_type::<Byte>(), &[BYTE, BYTE2]);
        assert_eq!(view.components_of_type::<Position>(), &[POS, POS2]);
    }

    #[test]
    fn order_of_comps_for_archetype_view_does_not_matter() {
        let view_1: ArchetypeComponentView<'_> = (&BYTE, &POS, &RECT).try_into().unwrap();
        let view_2: ArchetypeComponentView<'_> = (&POS, &BYTE, &RECT).try_into().unwrap();
        let view_3: ArchetypeComponentView<'_> = (&RECT, &BYTE, &POS).try_into().unwrap();
        assert_eq!(view_2.archetype, view_1.archetype);
        assert_eq!(view_3.archetype, view_1.archetype);
    }

    #[test]
    #[should_panic]
    fn conversion_of_two_comp_tuple_with_two_equal_comps_to_archetype_view_fails() {
        let _: ArchetypeComponentView<'_> = (&POS, &POS).try_into().unwrap();
    }

    #[test]
    #[should_panic]
    fn conversion_of_two_comp_tuple_with_two_equal_comp_slices_to_archetype_view_fails() {
        let _: ArchetypeComponentView<'_> = (&[POS, POS], &[POS, POS]).try_into().unwrap();
    }

    #[test]
    #[should_panic]
    fn conversion_of_three_comp_tuple_with_two_equal_comps_to_archetype_view_fails() {
        let _: ArchetypeComponentView<'_> = (&POS, &BYTE, &POS).try_into().unwrap();
    }

    #[test]
    #[should_panic]
    fn conversion_of_three_comp_tuple_with_two_equal_comp_slices_to_archetype_view_fails() {
        let _: ArchetypeComponentView<'_> = (&[POS, POS], &[BYTE, BYTE], &[POS, POS])
            .try_into()
            .unwrap();
    }

    #[test]
    fn valid_conversions_of_comp_tuples_to_archetype_views_succeed() {
        let view: ArchetypeComponentView<'_> = (&Marked).into();
        assert_eq!(view.archetype(), &archetype_of!(Marked));
        assert_eq!(view.n_component_types(), 1);
        assert_eq!(view.component_count(), 1);
        assert!(view.has_component_type::<Marked>());

        let view: SingleInstance<ArchetypeComponentView<'_>> = (&Marked).into();
        assert_eq!(view.archetype(), &archetype_of!(Marked));
        assert_eq!(view.n_component_types(), 1);
        assert_eq!(view.component_count(), 1);
        assert!(view.has_component_type::<Marked>());

        let view: ArchetypeComponentView<'_> = (&BYTE).into();
        assert_eq!(view.archetype(), &archetype_of!(Byte));
        assert_eq!(view.n_component_types(), 1);
        assert_eq!(view.component_count(), 1);
        assert!(view.has_component_type::<Byte>());
        assert_eq!(view.components_of_type::<Byte>(), &[BYTE]);

        let view: SingleInstance<ArchetypeComponentView<'_>> = (&BYTE).into();
        assert_eq!(view.archetype(), &archetype_of!(Byte));
        assert_eq!(view.n_component_types(), 1);
        assert_eq!(view.component_count(), 1);
        assert!(view.has_component_type::<Byte>());
        assert_eq!(view.components_of_type::<Byte>(), &[BYTE]);

        let view: ArchetypeComponentView<'_> = (&BYTE, &POS).try_into().unwrap();
        assert_eq!(view.archetype(), &archetype_of!(Byte, Position));
        assert_eq!(view.n_component_types(), 2);
        assert_eq!(view.component_count(), 1);
        assert!(view.has_component_type::<Byte>());
        assert!(view.has_component_type::<Position>());
        assert_eq!(view.components_of_type::<Byte>(), &[BYTE]);
        assert_eq!(view.components_of_type::<Position>(), &[POS]);

        let view: SingleInstance<ArchetypeComponentView<'_>> = (&BYTE, &POS).try_into().unwrap();
        assert_eq!(view.archetype(), &archetype_of!(Byte, Position));
        assert_eq!(view.n_component_types(), 2);
        assert_eq!(view.component_count(), 1);
        assert!(view.has_component_type::<Byte>());
        assert!(view.has_component_type::<Position>());
        assert_eq!(view.components_of_type::<Byte>(), &[BYTE]);
        assert_eq!(view.components_of_type::<Position>(), &[POS]);

        let view: ArchetypeComponentView<'_> = (&BYTE, &POS, &RECT).try_into().unwrap();
        assert_eq!(view.archetype(), &archetype_of!(Byte, Position, Rectangle));
        assert_eq!(view.n_component_types(), 3);
        assert_eq!(view.component_count(), 1);
        assert!(view.has_component_type::<Byte>());
        assert!(view.has_component_type::<Position>());
        assert!(view.has_component_type::<Rectangle>());
        assert_eq!(view.components_of_type::<Byte>(), &[BYTE]);
        assert_eq!(view.components_of_type::<Position>(), &[POS]);
        assert_eq!(view.components_of_type::<Rectangle>(), &[RECT]);

        let view: SingleInstance<ArchetypeComponentView<'_>> =
            (&BYTE, &POS, &RECT).try_into().unwrap();
        assert_eq!(view.archetype(), &archetype_of!(Byte, Position, Rectangle));
        assert_eq!(view.n_component_types(), 3);
        assert_eq!(view.component_count(), 1);
        assert!(view.has_component_type::<Byte>());
        assert!(view.has_component_type::<Position>());
        assert!(view.has_component_type::<Rectangle>());
        assert_eq!(view.components_of_type::<Byte>(), &[BYTE]);
        assert_eq!(view.components_of_type::<Position>(), &[POS]);
        assert_eq!(view.components_of_type::<Rectangle>(), &[RECT]);
    }

    #[test]
    fn valid_conversions_of_comp_slice_tuples_to_archetype_views_succeed() {
        let view: ArchetypeComponentView<'_> = (&[Marked]).into();
        assert_eq!(view.archetype(), &archetype_of!(Marked));
        assert_eq!(view.n_component_types(), 1);
        assert_eq!(view.component_count(), 1);
        assert!(view.has_component_type::<Marked>());

        let view: ArchetypeComponentView<'_> = (&[Marked, Marked]).into();
        assert_eq!(view.archetype(), &archetype_of!(Marked));
        assert_eq!(view.n_component_types(), 1);
        assert_eq!(view.component_count(), 2);
        assert!(view.has_component_type::<Marked>());

        let view: ArchetypeComponentView<'_> = (&[BYTE]).into();
        assert_eq!(view.archetype(), &archetype_of!(Byte));
        assert_eq!(view.n_component_types(), 1);
        assert_eq!(view.component_count(), 1);
        assert!(view.has_component_type::<Byte>());
        assert_eq!(view.components_of_type::<Byte>(), &[BYTE]);

        let view: ArchetypeComponentView<'_> = (&[BYTE, BYTE2]).into();
        assert_eq!(view.archetype(), &archetype_of!(Byte));
        assert_eq!(view.n_component_types(), 1);
        assert_eq!(view.component_count(), 2);
        assert!(view.has_component_type::<Byte>());
        assert_eq!(view.components_of_type::<Byte>(), &[BYTE, BYTE2]);

        let view: ArchetypeComponentView<'_> = (&[BYTE], &[POS]).try_into().unwrap();
        assert_eq!(view.archetype(), &archetype_of!(Byte, Position));
        assert_eq!(view.n_component_types(), 2);
        assert_eq!(view.component_count(), 1);
        assert!(view.has_component_type::<Byte>());
        assert!(view.has_component_type::<Position>());
        assert_eq!(view.components_of_type::<Byte>(), &[BYTE]);
        assert_eq!(view.components_of_type::<Position>(), &[POS]);

        let view: ArchetypeComponentView<'_> = (&[BYTE, BYTE2], &[POS, POS2]).try_into().unwrap();
        assert_eq!(view.archetype(), &archetype_of!(Byte, Position));
        assert_eq!(view.n_component_types(), 2);
        assert_eq!(view.component_count(), 2);
        assert!(view.has_component_type::<Byte>());
        assert!(view.has_component_type::<Position>());
        assert_eq!(view.components_of_type::<Byte>(), &[BYTE, BYTE2]);
        assert_eq!(view.components_of_type::<Position>(), &[POS, POS2]);

        let view: ArchetypeComponentView<'_> = (&[BYTE], &[POS], &[RECT]).try_into().unwrap();
        assert_eq!(view.archetype(), &archetype_of!(Byte, Position, Rectangle));
        assert_eq!(view.n_component_types(), 3);
        assert_eq!(view.component_count(), 1);
        assert!(view.has_component_type::<Byte>());
        assert!(view.has_component_type::<Position>());
        assert!(view.has_component_type::<Rectangle>());
        assert_eq!(view.components_of_type::<Byte>(), &[BYTE]);
        assert_eq!(view.components_of_type::<Position>(), &[POS]);
        assert_eq!(view.components_of_type::<Rectangle>(), &[RECT]);

        let view: ArchetypeComponentView<'_> = (&[BYTE, BYTE2], &[POS, POS2], &[RECT, RECT])
            .try_into()
            .unwrap();
        assert_eq!(view.archetype(), &archetype_of!(Byte, Position, Rectangle));
        assert_eq!(view.n_component_types(), 3);
        assert_eq!(view.component_count(), 2);
        assert!(view.has_component_type::<Byte>());
        assert!(view.has_component_type::<Position>());
        assert!(view.has_component_type::<Rectangle>());
        assert_eq!(view.components_of_type::<Byte>(), &[BYTE, BYTE2]);
        assert_eq!(view.components_of_type::<Position>(), &[POS, POS2]);
        assert_eq!(view.components_of_type::<Rectangle>(), &[RECT, RECT]);
    }

    #[test]
    #[should_panic]
    fn adding_existing_comp_array_to_archetype_view_fails() {
        let mut view: ArchetypeComponentView<'_> = (&BYTE, &POS, &RECT).try_into().unwrap();
        view.add_new_component_type((&POS).view()).unwrap();
    }

    #[test]
    #[should_panic]
    fn adding_comp_array_with_different_count_to_archetype_view_fails() {
        let mut view: ArchetypeComponentView<'_> = (&BYTE, &POS).try_into().unwrap();
        view.add_new_component_type((&[RECT, RECT]).view()).unwrap();
    }

    #[test]
    fn adding_individual_single_comp_arrays_to_archetype_view_works() {
        let mut view = ArchetypeComponentView::empty();
        view.add_new_component_type((&BYTE).view()).unwrap();
        assert_eq!(view.archetype(), &archetype_of!(Byte));
        assert_eq!(view.n_component_types(), 1);
        assert_eq!(view.component_count(), 1);
        assert!(view.has_component_type::<Byte>());
        assert_eq!(view.components_of_type::<Byte>(), &[BYTE]);

        view.add_new_component_type((&POS).view()).unwrap();
        assert_eq!(view.archetype(), &archetype_of!(Byte, Position));
        assert_eq!(view.n_component_types(), 2);
        assert_eq!(view.component_count(), 1);
        assert!(view.has_component_type::<Byte>());
        assert!(view.has_component_type::<Position>());
        assert_eq!(view.components_of_type::<Byte>(), &[BYTE]);
        assert_eq!(view.components_of_type::<Position>(), &[POS]);

        view.add_new_component_type((&RECT).view()).unwrap();
        assert_eq!(view.archetype(), &archetype_of!(Byte, Position, Rectangle));
        assert_eq!(view.n_component_types(), 3);
        assert_eq!(view.component_count(), 1);
        assert!(view.has_component_type::<Byte>());
        assert!(view.has_component_type::<Position>());
        assert!(view.has_component_type::<Rectangle>());
        assert_eq!(view.components_of_type::<Byte>(), &[BYTE]);
        assert_eq!(view.components_of_type::<Position>(), &[POS]);
        assert_eq!(view.components_of_type::<Rectangle>(), &[RECT]);
    }

    #[test]
    fn adding_individual_multi_comp_arrays_to_archetype_view_works() {
        let mut view = ArchetypeComponentView::empty();
        view.add_new_component_type((&[BYTE, BYTE2]).view())
            .unwrap();
        assert_eq!(view.archetype(), &archetype_of!(Byte));
        assert_eq!(view.n_component_types(), 1);
        assert_eq!(view.component_count(), 2);
        assert!(view.has_component_type::<Byte>());
        assert_eq!(view.components_of_type::<Byte>(), &[BYTE, BYTE2]);

        view.add_new_component_type((&[POS, POS2]).view()).unwrap();
        assert_eq!(view.archetype(), &archetype_of!(Byte, Position));
        assert_eq!(view.n_component_types(), 2);
        assert_eq!(view.component_count(), 2);
        assert!(view.has_component_type::<Byte>());
        assert!(view.has_component_type::<Position>());
        assert_eq!(view.components_of_type::<Byte>(), &[BYTE, BYTE2]);
        assert_eq!(view.components_of_type::<Position>(), &[POS, POS2]);

        view.add_new_component_type((&[RECT, RECT]).view()).unwrap();
        assert_eq!(view.archetype(), &archetype_of!(Byte, Position, Rectangle));
        assert_eq!(view.n_component_types(), 3);
        assert_eq!(view.component_count(), 2);
        assert!(view.has_component_type::<Byte>());
        assert!(view.has_component_type::<Position>());
        assert!(view.has_component_type::<Rectangle>());
        assert_eq!(view.components_of_type::<Byte>(), &[BYTE, BYTE2]);
        assert_eq!(view.components_of_type::<Position>(), &[POS, POS2]);
        assert_eq!(view.components_of_type::<Rectangle>(), &[RECT, RECT]);
    }

    #[test]
    #[should_panic]
    fn adding_and_not_overwriting_comp_array_with_different_count_in_archetype_view_fails() {
        let mut view: ArchetypeComponentView<'_> = (&BYTE, &POS).try_into().unwrap();
        view.add_or_overwrite_component_types([(&[RECT, RECT]).view()])
            .unwrap();
    }

    #[test]
    #[should_panic]
    fn overwriting_comp_array_with_different_count_in_archetype_view_fails() {
        let mut view: ArchetypeComponentView<'_> = (&BYTE, &RECT).try_into().unwrap();
        view.add_or_overwrite_component_types([(&[RECT, RECT]).view()])
            .unwrap();
    }

    #[test]
    #[should_panic]
    fn including_multiple_comp_arrays_with_different_counts_in_archetype_view_fails() {
        let mut view: ArchetypeComponentView<'_> = (&BYTE).into();
        view.add_or_overwrite_component_types([(&[RECT, RECT]).view(), (&POS).view()])
            .unwrap();
    }

    #[test]
    fn adding_and_not_overwriting_individual_single_comp_arrays_in_archetype_view_works() {
        let mut view = ArchetypeComponentView::empty();
        view.add_or_overwrite_component_types([(&BYTE).view()])
            .unwrap();
        assert_eq!(view.archetype(), &archetype_of!(Byte));
        assert_eq!(view.n_component_types(), 1);
        assert_eq!(view.component_count(), 1);
        assert!(view.has_component_type::<Byte>());
        assert_eq!(view.components_of_type::<Byte>(), &[BYTE]);

        view.add_or_overwrite_component_types([(&POS).view()])
            .unwrap();
        assert_eq!(view.archetype(), &archetype_of!(Byte, Position));
        assert_eq!(view.n_component_types(), 2);
        assert_eq!(view.component_count(), 1);
        assert!(view.has_component_type::<Byte>());
        assert!(view.has_component_type::<Position>());
        assert_eq!(view.components_of_type::<Byte>(), &[BYTE]);
        assert_eq!(view.components_of_type::<Position>(), &[POS]);

        view.add_or_overwrite_component_types([(&RECT).view()])
            .unwrap();
        assert_eq!(view.archetype(), &archetype_of!(Byte, Position, Rectangle));
        assert_eq!(view.n_component_types(), 3);
        assert_eq!(view.component_count(), 1);
        assert!(view.has_component_type::<Byte>());
        assert!(view.has_component_type::<Position>());
        assert!(view.has_component_type::<Rectangle>());
        assert_eq!(view.components_of_type::<Byte>(), &[BYTE]);
        assert_eq!(view.components_of_type::<Position>(), &[POS]);
        assert_eq!(view.components_of_type::<Rectangle>(), &[RECT]);
    }

    #[test]
    fn adding_and_not_overwriting_individual_two_comp_arrays_in_archetype_view_works() {
        let mut view = ArchetypeComponentView::empty();
        view.add_or_overwrite_component_types([(&[BYTE, BYTE2]).view()])
            .unwrap();
        assert_eq!(view.archetype(), &archetype_of!(Byte));
        assert_eq!(view.n_component_types(), 1);
        assert_eq!(view.component_count(), 2);
        assert!(view.has_component_type::<Byte>());
        assert_eq!(view.components_of_type::<Byte>(), &[BYTE, BYTE2]);

        view.add_or_overwrite_component_types([(&[POS, POS2]).view()])
            .unwrap();
        assert_eq!(view.archetype(), &archetype_of!(Byte, Position));
        assert_eq!(view.n_component_types(), 2);
        assert_eq!(view.component_count(), 2);
        assert!(view.has_component_type::<Byte>());
        assert!(view.has_component_type::<Position>());
        assert_eq!(view.components_of_type::<Byte>(), &[BYTE, BYTE2]);
        assert_eq!(view.components_of_type::<Position>(), &[POS, POS2]);
    }

    #[test]
    fn adding_and_not_overwriting_multiple_single_comp_arrays_in_archetype_view_works() {
        let mut view = ArchetypeComponentView::empty();
        view.add_or_overwrite_component_types([(&POS).view(), (&BYTE).view(), (&RECT).view()])
            .unwrap();
        assert_eq!(view.archetype(), &archetype_of!(Byte, Position, Rectangle));
        assert_eq!(view.n_component_types(), 3);
        assert_eq!(view.component_count(), 1);
        assert!(view.has_component_type::<Byte>());
        assert!(view.has_component_type::<Position>());
        assert!(view.has_component_type::<Rectangle>());
        assert_eq!(view.components_of_type::<Byte>(), &[BYTE]);
        assert_eq!(view.components_of_type::<Position>(), &[POS]);
        assert_eq!(view.components_of_type::<Rectangle>(), &[RECT]);
    }

    #[test]
    fn overwriting_individual_single_comp_array_in_archetype_view_works() {
        let mut view: ArchetypeComponentView<'_> = (&POS, &BYTE).try_into().unwrap();

        view.add_or_overwrite_component_types([(&BYTE2).view()])
            .unwrap();
        assert_eq!(view.archetype(), &archetype_of!(Byte, Position));
        assert_eq!(view.n_component_types(), 2);
        assert_eq!(view.component_count(), 1);
        assert!(view.has_component_type::<Byte>());
        assert!(view.has_component_type::<Position>());
        assert_eq!(view.components_of_type::<Byte>(), &[BYTE2]);
        assert_eq!(view.components_of_type::<Position>(), &[POS]);
    }

    #[test]
    fn overwriting_individual_two_comp_array_in_archetype_view_works() {
        let mut view: ArchetypeComponentView<'_> =
            (&[BYTE, BYTE2], &[POS, POS2]).try_into().unwrap();

        view.add_or_overwrite_component_types([(&[BYTE2, BYTE]).view()])
            .unwrap();
        assert_eq!(view.archetype(), &archetype_of!(Byte, Position));
        assert_eq!(view.n_component_types(), 2);
        assert_eq!(view.component_count(), 2);
        assert!(view.has_component_type::<Byte>());
        assert!(view.has_component_type::<Position>());
        assert_eq!(view.components_of_type::<Byte>(), &[BYTE2, BYTE]);
        assert_eq!(view.components_of_type::<Position>(), &[POS, POS2]);
    }

    #[test]
    fn overwriting_multiple_single_comp_arrays_in_archetype_view_works() {
        let mut view: ArchetypeComponentView<'_> = (&POS, &BYTE).try_into().unwrap();

        view.add_or_overwrite_component_types([(&BYTE2).view(), (&POS2).view()])
            .unwrap();
        assert_eq!(view.archetype(), &archetype_of!(Byte, Position));
        assert_eq!(view.n_component_types(), 2);
        assert_eq!(view.component_count(), 1);
        assert!(view.has_component_type::<Byte>());
        assert!(view.has_component_type::<Position>());
        assert_eq!(view.components_of_type::<Byte>(), &[BYTE2]);
        assert_eq!(view.components_of_type::<Position>(), &[POS2]);
    }

    #[test]
    fn overwriting_multiple_two_comp_arrays_in_archetype_view_works() {
        let mut view: ArchetypeComponentView<'_> =
            (&[POS, POS2], &[BYTE, BYTE2]).try_into().unwrap();

        view.add_or_overwrite_component_types([(&[BYTE2, BYTE]).view(), (&[POS2, POS]).view()])
            .unwrap();
        assert_eq!(view.archetype(), &archetype_of!(Byte, Position));
        assert_eq!(view.n_component_types(), 2);
        assert_eq!(view.component_count(), 2);
        assert!(view.has_component_type::<Byte>());
        assert!(view.has_component_type::<Position>());
        assert_eq!(view.components_of_type::<Byte>(), &[BYTE2, BYTE]);
        assert_eq!(view.components_of_type::<Position>(), &[POS2, POS]);
    }

    #[test]
    fn adding_and_overwriting_multiple_single_comp_arrays_in_archetype_view_works() {
        let mut view: ArchetypeComponentView<'_> = (&POS, &RECT).try_into().unwrap();

        view.add_or_overwrite_component_types([(&BYTE).view(), (&POS2).view()])
            .unwrap();
        assert_eq!(view.archetype(), &archetype_of!(Byte, Position, Rectangle));
        assert_eq!(view.n_component_types(), 3);
        assert_eq!(view.component_count(), 1);
        assert!(view.has_component_type::<Byte>());
        assert!(view.has_component_type::<Position>());
        assert!(view.has_component_type::<Rectangle>());
        assert_eq!(view.components_of_type::<Byte>(), &[BYTE]);
        assert_eq!(view.components_of_type::<Position>(), &[POS2]);
        assert_eq!(view.components_of_type::<Rectangle>(), &[RECT]);
    }

    #[test]
    fn adding_and_overwriting_multiple_two_comp_arrays_in_archetype_view_works() {
        let mut view: ArchetypeComponentView<'_> =
            (&[POS, POS2], &[RECT, RECT]).try_into().unwrap();

        view.add_or_overwrite_component_types([(&[BYTE, BYTE2]).view(), (&[POS2, POS]).view()])
            .unwrap();
        assert_eq!(view.archetype(), &archetype_of!(Byte, Position, Rectangle));
        assert_eq!(view.n_component_types(), 3);
        assert_eq!(view.component_count(), 2);
        assert!(view.has_component_type::<Byte>());
        assert!(view.has_component_type::<Position>());
        assert!(view.has_component_type::<Rectangle>());
        assert_eq!(view.components_of_type::<Byte>(), &[BYTE, BYTE2]);
        assert_eq!(view.components_of_type::<Position>(), &[POS2, POS]);
        assert_eq!(view.components_of_type::<Rectangle>(), &[RECT, RECT]);
    }

    #[test]
    fn overwriting_multiple_single_comp_arrays_of_same_type_in_archetype_view_works() {
        let mut view: ArchetypeComponentView<'_> = (&BYTE).into();

        view.add_or_overwrite_component_types([(&BYTE2).view(), (&BYTE).view()])
            .unwrap();
        assert_eq!(view.archetype(), &archetype_of!(Byte));
        assert_eq!(view.n_component_types(), 1);
        assert_eq!(view.component_count(), 1);
        assert!(view.has_component_type::<Byte>());
        assert_eq!(view.components_of_type::<Byte>(), &[BYTE]);
    }

    #[test]
    #[should_panic]
    fn removing_missing_component_from_archetype_view_fails() {
        let mut view: ArchetypeComponentView<'_> = (&BYTE, &RECT).try_into().unwrap();
        view.remove_component_type_with_id(Position::component_id())
            .unwrap();
    }

    #[test]
    #[should_panic]
    fn removing_component_from_empty_archetype_view_fails() {
        let mut view = ArchetypeComponentView::empty();
        view.remove_component_type_with_id(Position::component_id())
            .unwrap();
    }

    #[test]
    fn removing_components_from_archetype_view_works() {
        let mut view: ArchetypeComponentView<'_> = (&BYTE, &POS, &RECT).try_into().unwrap();
        view.remove_component_type_with_id(Byte::component_id())
            .unwrap();
        assert_eq!(view.archetype(), &archetype_of!(Position, Rectangle));
        assert_eq!(view.n_component_types(), 2);
        assert!(view.has_component_type::<Position>());
        assert!(view.has_component_type::<Rectangle>());

        view.remove_component_type_with_id(Rectangle::component_id())
            .unwrap();
        assert_eq!(view.archetype(), &archetype_of!(Position));
        assert_eq!(view.n_component_types(), 1);
        assert!(view.has_component_type::<Position>());

        view.remove_component_type_with_id(Position::component_id())
            .unwrap();
        assert_eq!(view.archetype(), &archetype_of!());
        assert_eq!(view.n_component_types(), 0);
    }

    #[test]
    #[should_panic]
    fn removing_multiple_components_from_archetype_view_when_one_is_missing_fails() {
        let mut view: ArchetypeComponentView<'_> = (&BYTE, &RECT).try_into().unwrap();
        view.remove_component_types_with_ids([Byte::component_id(), Position::component_id()])
            .unwrap();
    }

    #[test]
    #[should_panic]
    fn removing_multiple_of_the_same_component_from_archetype_view_fails() {
        let mut view: ArchetypeComponentView<'_> = (&BYTE, &RECT).try_into().unwrap();
        view.remove_component_types_with_ids([Byte::component_id(), Byte::component_id()])
            .unwrap();
    }

    #[test]
    #[should_panic]
    fn removing_multiple_components_from_empty_archetype_view_fails() {
        let mut view = ArchetypeComponentView::empty();
        view.remove_component_types_with_ids([Byte::component_id(), Position::component_id()])
            .unwrap();
    }

    #[test]
    fn removing_multiple_components_from_archetype_view_works() {
        let mut view: ArchetypeComponentView<'_> = (&BYTE, &POS, &RECT).try_into().unwrap();
        view.remove_component_types_with_ids([Byte::component_id(), Rectangle::component_id()])
            .unwrap();
        assert_eq!(view.archetype(), &archetype_of!(Position));
        assert_eq!(view.n_component_types(), 1);
        assert!(view.has_component_type::<Position>());

        view.remove_component_types_with_ids([Position::component_id()])
            .unwrap();
        assert_eq!(view.archetype(), &archetype_of!());
        assert_eq!(view.n_component_types(), 0);
    }

    #[test]
    fn duplicating_single_instance_archetype_storage_works() {
        let single_instance_storage =
            ArchetypeComponentStorage::try_from_single_instance_view((&BYTE, &POS, &RECT)).unwrap();

        let storage = single_instance_storage.duplicate_instance(3);

        assert_eq!(
            storage.archetype(),
            &archetype_of!(Byte, Position, Rectangle)
        );
        assert_eq!(storage.n_component_types(), 3);
        assert_eq!(storage.component_count(), 3);
        assert!(storage.has_component_type::<Byte>());
        assert!(storage.has_component_type::<Position>());
        assert!(storage.has_component_type::<Rectangle>());
        assert_eq!(storage.components_of_type::<Byte>(), &[BYTE, BYTE, BYTE]);
        assert_eq!(storage.components_of_type::<Position>(), &[POS, POS, POS]);
        assert_eq!(
            storage.components_of_type::<Rectangle>(),
            &[RECT, RECT, RECT]
        );
    }

    #[test]
    fn combining_single_instance_archetype_storage_with_single_instance_archetype_view_works() {
        let single_instance_storage =
            ArchetypeComponentStorage::try_from_single_instance_view((&BYTE, &POS)).unwrap();

        let combined_storage = single_instance_storage
            .combined_with((&RECT, &BYTE2))
            .unwrap();

        assert_eq!(
            combined_storage.archetype(),
            &archetype_of!(Byte, Position, Rectangle)
        );
        assert_eq!(combined_storage.n_component_types(), 3);
        assert_eq!(combined_storage.component_count(), 1);
        assert!(combined_storage.has_component_type::<Byte>());
        assert!(combined_storage.has_component_type::<Position>());
        assert!(combined_storage.has_component_type::<Rectangle>());
        assert_eq!(combined_storage.components_of_type::<Byte>(), &[BYTE2]);
        assert_eq!(combined_storage.components_of_type::<Position>(), &[POS]);
        assert_eq!(combined_storage.components_of_type::<Rectangle>(), &[RECT]);
    }

    #[test]
    fn combining_single_instance_archetype_storage_with_multi_instance_archetype_view_works() {
        let single_instance_storage =
            ArchetypeComponentStorage::try_from_single_instance_view((&BYTE, &POS)).unwrap();

        let combined_storage = single_instance_storage
            .combined_with((&[RECT, RECT], &[BYTE2, BYTE2]))
            .unwrap();

        assert_eq!(
            combined_storage.archetype(),
            &archetype_of!(Byte, Position, Rectangle)
        );
        assert_eq!(combined_storage.n_component_types(), 3);
        assert_eq!(combined_storage.component_count(), 2);
        assert!(combined_storage.has_component_type::<Byte>());
        assert!(combined_storage.has_component_type::<Position>());
        assert!(combined_storage.has_component_type::<Rectangle>());
        assert_eq!(
            combined_storage.components_of_type::<Byte>(),
            &[BYTE2, BYTE2]
        );
        assert_eq!(
            combined_storage.components_of_type::<Position>(),
            &[POS, POS]
        );
        assert_eq!(
            combined_storage.components_of_type::<Rectangle>(),
            &[RECT, RECT]
        );
    }

    #[test]
    fn constructing_table_works() {
        let entity_0 = EntityID(0);
        let entity_42 = EntityID(42);
        let entity_10 = EntityID(10);

        let table = ArchetypeTable::new_with_entities([entity_0], (&BYTE).into());
        assert!(table.has_entity(entity_0));
        assert_eq!(table.entity(entity_0).component::<Byte>(), &BYTE);

        let table =
            ArchetypeTable::new_with_entities([entity_42], (&RECT, &POS).try_into().unwrap());
        assert!(table.has_entity(entity_42));
        let entity = table.entity(entity_42);
        assert_eq!(entity.component::<Position>(), &POS);
        assert_eq!(entity.component::<Rectangle>(), &RECT);

        let table = ArchetypeTable::new_with_entities(
            [entity_10],
            (&BYTE, &RECT, &POS).try_into().unwrap(),
        );
        assert!(table.has_entity(entity_10));
        let entity = table.entity(entity_10);
        assert_eq!(entity.component::<Byte>(), &BYTE);
        assert_eq!(entity.component::<Position>(), &POS);
        assert_eq!(entity.component::<Rectangle>(), &RECT);
    }

    #[test]
    fn getting_iter_over_all_entities_works() {
        let entity_0 = EntityID(0);
        let entity_1 = EntityID(1);
        let mut inserted_entities: HashSet<_> = [entity_0, entity_1].into_iter().collect();

        let mut table =
            ArchetypeTable::new_with_entities([entity_0], (&RECT, &POS).try_into().unwrap());
        table.add_entities([entity_1], (&RECT, &POS).try_into().unwrap());

        let mut entities = table.all_entities();

        let entity = entities.next().unwrap();
        assert_eq!(entity.archetype_id(), table.archetype().id());
        assert!(inserted_entities.remove(&entity.id()));

        let entity = entities.next().unwrap();
        assert_eq!(entity.archetype_id(), table.archetype().id());
        assert!(inserted_entities.remove(&entity.id()));

        assert!(entities.next().is_none());
    }

    #[test]
    fn adding_entity_to_table_works() {
        let entity_0 = EntityID(0);
        let entity_1 = EntityID(1);
        let entity_3 = EntityID(3);
        let entity_7 = EntityID(7);

        let mut table = ArchetypeTable::new_with_entities([entity_0], (&BYTE).into());
        table.add_entities([entity_1], (&BYTE).into());
        assert!(table.has_entity(entity_0));
        assert_eq!(table.entity(entity_0).component::<Byte>(), &BYTE);
        assert!(table.has_entity(entity_1));
        assert_eq!(table.entity(entity_1).component::<Byte>(), &BYTE);

        let mut table =
            ArchetypeTable::new_with_entities([entity_3], (&RECT, &POS).try_into().unwrap());
        table.add_entities([entity_7], (&RECT, &POS).try_into().unwrap());
        assert!(table.has_entity(entity_3));
        let entity = table.entity(entity_3);
        assert_eq!(entity.component::<Position>(), &POS);
        assert_eq!(entity.component::<Rectangle>(), &RECT);
        assert!(table.has_entity(entity_7));
        let entity = table.entity(entity_7);
        assert_eq!(entity.component::<Position>(), &POS);
        assert_eq!(entity.component::<Rectangle>(), &RECT);
    }

    #[test]
    fn adding_entities_with_components_in_different_orders_to_table_works() {
        let entity_0 = EntityID(0);
        let entity_1 = EntityID(1);

        let mut table =
            ArchetypeTable::new_with_entities([entity_0], (&BYTE, &POS).try_into().unwrap());

        table.add_entities([entity_1], (&POS, &BYTE).try_into().unwrap());
    }

    #[test]
    #[should_panic]
    fn adding_existing_entity_to_table_fails() {
        let entity_0 = EntityID(0);
        let mut table = ArchetypeTable::new_with_entities([entity_0], (&BYTE).into());
        table.add_entities([entity_0], (&BYTE).into());
    }

    #[test]
    fn removing_entity_from_table_works() {
        let entity_0 = EntityID(0);
        let entity_1 = EntityID(1);

        let mut table =
            ArchetypeTable::new_with_entities([entity_0], (&RECT, &POS).try_into().unwrap());
        table.add_entities([entity_1], (&RECT, &POS).try_into().unwrap());

        table.remove_entity(entity_0).unwrap();
        assert!(!table.has_entity(entity_0));
        assert!(table.has_entity(entity_1));

        table.remove_entity(entity_1).unwrap();
        assert!(table.is_empty());
    }

    #[test]
    #[should_panic]
    fn removing_missing_entity_from_table_fails() {
        let mut table =
            ArchetypeTable::new_with_entities([EntityID(0)], (&RECT, &POS).try_into().unwrap());
        table.remove_entity(EntityID(1)).unwrap();
    }

    #[test]
    #[should_panic]
    fn removing_entity_from_empty_table_fails() {
        let entity_0 = EntityID(0);
        let mut table =
            ArchetypeTable::new_with_entities([entity_0], (&RECT, &POS).try_into().unwrap());
        table.remove_entity(entity_0).unwrap();
        table.remove_entity(entity_0).unwrap();
    }

    #[test]
    fn removing_all_entities_from_single_entity_table_works() {
        let entity_0 = EntityID(0);
        let mut table = ArchetypeTable::new_with_entities([entity_0], (&BYTE).into());

        table.remove_all_entities();

        assert!(table.is_empty());
        assert!(!table.has_entity(entity_0));
        assert_eq!(
            table
                .component_storage(Byte::component_id())
                .read()
                .unwrap()
                .component_count(),
            0
        );
    }

    #[test]
    fn removing_all_entities_from_multi_entity_table_works() {
        let entity_0 = EntityID(0);
        let entity_1 = EntityID(1);

        let mut table =
            ArchetypeTable::new_with_entities([entity_0], (&RECT, &POS).try_into().unwrap());
        table.add_entities([entity_1], (&RECT, &POS).try_into().unwrap());

        table.remove_all_entities();

        assert!(table.is_empty());
        assert!(!table.has_entity(entity_0));
        assert!(!table.has_entity(entity_1));
        assert_eq!(
            table
                .component_storage(Rectangle::component_id())
                .read()
                .unwrap()
                .component_count(),
            0
        );
        assert_eq!(
            table
                .component_storage(Position::component_id())
                .read()
                .unwrap()
                .component_count(),
            0
        );
    }

    #[test]
    fn reusing_table_after_removing_all_entities_works() {
        let entity_0 = EntityID(0);
        let entity_1 = EntityID(1);

        let mut table = ArchetypeTable::new_with_entities([entity_0], (&BYTE).into());
        table.remove_all_entities();

        table.add_entities([entity_1], (&BYTE2).into());

        assert!(!table.has_entity(entity_0));
        assert!(table.has_entity(entity_1));
        assert_eq!(table.entity(entity_1).component::<Byte>(), &BYTE2);
    }
}
