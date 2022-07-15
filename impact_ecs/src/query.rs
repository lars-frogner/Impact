//! Querying for sets of ECS components.

use super::{
    archetype::{Archetype, ArchetypeTable},
    component::{Component, ComponentStorage},
    world::World,
};
use anyhow::Result;
use paste::paste;
use std::iter::Zip;
use std::{
    hash::Hash,
    marker::PhantomData,
    sync::{RwLock, RwLockReadGuard, RwLockWriteGuard},
};

/// A query for a specific set of component types. Any
/// [`Entity`](super::world::Entity) in the [`World`] that has
/// components of all types specified in the query is considered
/// a match. The `ComponentQuery` can then provide an iterator
/// over each matching `Entity`'s relevant component data (see
/// [`ComponentQuery::iter_mut`]).
///
/// To create a query, we need to construct a type implementing
/// the [`IntoComponentQuery`] trait, and call the static method
/// [`query`](IntoComponentQuery::query) on that type. The trait
/// is implemented for any tuple containing [`Component`] types
/// wrapped in one the [`StorageAccess`] types; [`Read`] and [`Write`].
///
/// # Examples
/// ```
/// # use impact_ecs::{
/// #    query::{Read, Write, IntoComponentQuery},
/// #    world::World
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
/// let entity = world.create_entity((&Distance(0.0), &Speed(10.0)))?;
///
/// let mut query = <(Write<Distance>, Read<Speed>)>::query(&mut world)?;
/// for (distance, speed) in query.iter_mut() {
///     distance.0 += speed.0 * 0.1;
/// }
/// #
/// # Ok::<(), Error>(())
/// ```
///
/// # Concurrency
///
/// When a `ComponentQuery` is constructed, it acquires the
/// [`RwLock`] protecting each component storage covered by
/// the query, either for exclusive access (if the [`Write`]
/// marker type was used with the component) or shared access
/// (if the [`Read`] marker type was used). The locks are held
/// until the `ComponentQuery` is dropped.
#[derive(Debug)]
pub struct ComponentQuery<C, G> {
    guards: Vec<G>,
    _query_type: PhantomData<C>,
}

/// Represents types that can be used to create a
/// [`ComponentQuery`].
///
/// # Lifetimes
/// - `'w` is the lifetime of the [`World`] owning all the
/// component data we are trying to access.
/// - `'g` is the lifetime of the [`AccessGuard`]s created
/// to hold the [`RwLock`]s on the [`ComponentStorage`]s.
pub trait IntoComponentQuery<'w, 'g, C> {
    /// The type of [`AccessGuardGroup`] that will provide
    /// access to the data requested by the query.
    type Guards: AccessGuardGroup<'w, 'g, C>;

    /// Constructs a [`ComponentQuery`] based on the type of
    /// `Self`, which provides access to all [`Component`] data
    /// in the given [`World`] matching the query.
    ///
    /// # Errors
    /// Returns an error if the set of requested components
    /// has no valid archetype, which happens if there are
    /// multiple components of the same type.
    fn query(world: &'w mut World) -> Result<ComponentQuery<C, Self::Guards>> {
        let archetype = Self::determine_archetype()?;
        let tables = world.find_tables_containing_archetype(archetype);
        let guards: Vec<_> = tables
            .into_iter()
            .map(|table| {
                // Since we know the components are present in the table we just unwrap the result
                Self::access_archetype_table(table).unwrap()
            })
            .collect();
        Ok(ComponentQuery::new(guards))
    }

    /// Uses the type information of `Self` to determine
    /// which [`Archetype`] the set of components queried
    /// for corresponds to.
    ///
    /// # Errors
    /// Returns an error if the set of components has no valid
    /// archetype, which happens if there are multiple component
    /// of the same type.
    fn determine_archetype() -> Result<Archetype>;

    /// Obtains access to the relevant component data in the
    /// given [`ArchetypeTable`].
    ///
    /// # Errors
    /// Returns an error if the requested component types are
    /// not present in the table.
    fn access_archetype_table(table: &'w ArchetypeTable) -> Result<Self::Guards>;
}

/// Marker type that is wrapped around a [`Component`] type
/// in a query in order to indicate that we need read-only
/// access to the data for that component type.
#[derive(Clone, Copy, Debug, PartialEq, Hash)]
pub struct Read<C>(PhantomData<C>);

/// Marker type that is wrapped around a [`Component`] type
/// in a query in order to indicate that we need write access
/// to the data for that component type.
#[derive(Clone, Copy, Debug, PartialEq, Hash)]
pub struct Write<C>(PhantomData<C>);

/// Represents a type used to mark the kind of access to a
/// storage that is required. It is implemented by the [`Read`]
/// and [`Write`] types.
pub trait StorageAccess<'w, 'g, C> {
    /// The type of [`AccessGuard`] that will manage access
    /// to the storage.
    type Guard: AccessGuard<'w, 'g, C>;

    /// Creates an [`AccessGuard`] that will manage access
    /// to the given [`ComponentStorage`].
    ///
    /// # Panics
    /// If the [`RwLock`] is poisoned.
    fn access(storage: &'w RwLock<ComponentStorage>) -> Self::Guard;
}

/// Represents a set of [`AccessGuard`]s that together can
/// provide simulaneous access to multiple component storages.
pub trait AccessGuardGroup<'w, 'g, C> {
    type OutputItem;
    type OutputIter: Iterator<Item = Self::OutputItem>;

    /// Provides an iterator over all the components in the
    /// set of storages.
    fn iter(guards: &'g mut Self) -> Self::OutputIter;
}

/// Represents a guard that manages access to a single
/// component storage.
pub trait AccessGuard<'w, 'g, C> {
    type OutputSlice;
    type OutputItem;
    type OutputIter: Iterator<Item = Self::OutputItem>;

    /// Provides an slice with the components in the
    /// storage.
    fn slice(&'g mut self) -> Self::OutputSlice;

    /// Provides an iterator over the components in the
    /// storage.
    fn iter(&'g mut self) -> Self::OutputIter;
}

/// A guard that holds a [`RwLockReadGuard`] with a
/// reference to a component storage, and thus can
/// provide read-only access to the storage. The lock
/// is released when the guard is dropped, which happens
/// when the [`ComponentQuery`] owning the guard is dropped.
#[derive(Debug)]
pub struct ReadGuard<'w, C> {
    guard: RwLockReadGuard<'w, ComponentStorage>,
    _component_type: PhantomData<C>,
}

/// A guard that holds a [`RwLockWriteGuard`] with a
/// reference to a component storage, and thus can
/// provide write access to the storage. The lock
/// is released when the guard is dropped, which happens
/// when the [`ComponentQuery`] owning the guard is dropped.
#[derive(Debug)]
pub struct WriteGuard<'w, C> {
    guard: RwLockWriteGuard<'w, ComponentStorage>,
    _component_type: PhantomData<C>,
}

impl<'w, 'g, C, G> ComponentQuery<C, G>
where
    G: AccessGuardGroup<'w, 'g, C>,
{
    fn new(guards: Vec<G>) -> Self {
        Self {
            guards,
            _query_type: PhantomData,
        }
    }

    /// Provides an iterator over the components requested
    /// by the query. Each item of the iterator is a tuple
    /// with references to values of the component types
    /// specified in the type used to construct the query,
    /// with the component types occurring in the same order.
    /// The references are either shared or exclusive depending
    /// on whether the [`Read`] or [`Write`] marker type was
    /// used around the component type.
    ///
    /// Since the iterator is created by [`zip`](Iterator::zip)-ing
    /// together each individual iterator over components of a given
    /// type, the output item is a nested tuple, e.g.
    /// `((comp_1, comp_2), comp_3)`.
    pub fn iter_mut(
        &'g mut self,
    ) -> impl Iterator<Item = <G as AccessGuardGroup<'w, 'g, C>>::OutputItem> {
        self.guards.iter_mut().flat_map(AccessGuardGroup::iter)
    }
}

impl<'w, 'g, C> StorageAccess<'w, 'g, C> for Read<C>
where
    C: 'w + Component,
{
    type Guard = ReadGuard<'w, C>;

    fn access(storage: &'w RwLock<ComponentStorage>) -> Self::Guard {
        // Acquire the lock on the storage for read access
        Self::Guard::new(storage.read().unwrap())
    }
}

impl<'w, 'g, C> StorageAccess<'w, 'g, C> for Write<C>
where
    C: 'w + Component,
{
    type Guard = WriteGuard<'w, C>;

    fn access(storage: &'w RwLock<ComponentStorage>) -> Self::Guard {
        // Acquire the lock on the storage for write access
        Self::Guard::new(storage.write().unwrap())
    }
}

impl<'w, C> ReadGuard<'w, C>
where
    C: Component,
{
    fn new(guard: RwLockReadGuard<'w, ComponentStorage>) -> Self {
        Self {
            guard,
            _component_type: PhantomData,
        }
    }
}

// A `ReadGuard` will provide an immutable slice iterator
// over the components in the storage it protects
impl<'w, 'g, C> AccessGuard<'w, 'g, C> for ReadGuard<'w, C>
where
    C: 'w + Component,
{
    type OutputSlice = &'g [C];
    type OutputItem = &'g C;
    type OutputIter = std::slice::Iter<'g, C>;

    fn slice(&'g mut self) -> Self::OutputSlice {
        self.guard.slice()
    }

    fn iter(&'g mut self) -> Self::OutputIter {
        self.slice().iter()
    }
}

impl<'w, C> WriteGuard<'w, C>
where
    C: Component,
{
    fn new(guard: RwLockWriteGuard<'w, ComponentStorage>) -> Self {
        Self {
            guard,
            _component_type: PhantomData,
        }
    }
}

// A `WriteGuard` will provide a mutable slice iterator
// over the components in the storage it protects
impl<'w, 'g, C> AccessGuard<'w, 'g, C> for WriteGuard<'w, C>
where
    C: 'w + Component,
{
    type OutputSlice = &'g mut [C];
    type OutputItem = &'g mut C;
    type OutputIter = std::slice::IterMut<'g, C>;

    fn slice(&'g mut self) -> Self::OutputSlice {
        self.guard.slice_mut()
    }

    fn iter(&'g mut self) -> Self::OutputIter {
        self.slice().iter_mut()
    }
}

/// Macro for generating implementations of [`IntoComponentQuery`] for
/// relevant groups of types. The implementations need a type parameter
/// for each [`Component`] type (e.g. `Position`) as well as for the
/// wrapping [`StorageAccess`] type (e.g. `Read<Position>`).
macro_rules! impl_IntoComponentQuery {
    // For a single access-wrapped component type
    (component = $c:ident, access = $a:ident) => {
        impl<'w, 'g, $c, $a> IntoComponentQuery<'w, 'g, $c> for $a
        where
            $a: StorageAccess<'w, 'g, $c>,
            $c: Component,
        {
            type Guards = $a::Guard;

            fn determine_archetype() -> Result<Archetype> {
                Archetype::new_from_component_id_arr([$c::component_id()])
            }

            fn access_archetype_table(table: &'w ArchetypeTable) -> Result<Self::Guards> {
                table.access_component_storage::<'w, 'g, $c, $a>()
            }
        }
    };
    // For a tuple of access-wrapped component types
    (component_tuple = ($($c:ident),*), access_tuple = ($($a:ident),*)) => {
        impl<'w, 'g, $($c),*, $($a),*> IntoComponentQuery<'w, 'g, ($($c),*)> for ($($a),*)
        where
            $($a: StorageAccess<'w, 'g, $c>,)*
            $($c: Component,)*
        {
            type Guards = ($($a::Guard),*);

            fn determine_archetype() -> Result<Archetype> {
                Archetype::new_from_component_id_arr([$($c::component_id()),*])
            }

            fn access_archetype_table(table: &'w ArchetypeTable) -> Result<Self::Guards> {
                Ok(($(table.access_component_storage::<'w, 'g, $c, $a>()?),*))
            }
        }
    };
}

/// Macro for generating implementations of [`AccessGuardGroup`] for
/// the groups of [`AccessGuard`]s specified in the `Guards` associated
/// type in the [`impl_IntoComponentQuery`] macro.
macro_rules! impl_AccessGuardGroup {
    (component = $c:ident, guard = $g:ident) => {
        impl<'w, 'g, $c, $g> AccessGuardGroup<'w, 'g, $c> for $g
        where
            $g: AccessGuard<'w, 'g, $c>,
        {
            type OutputItem = $g::OutputItem;
            type OutputIter = $g::OutputIter;

            fn iter(guard: &'g mut Self) -> Self::OutputIter {
                guard.iter()
            }
        }
    };
    (
        component_tuple = ($c1:ident, $($c:ident),*),
        guard_tuple = ($g1:ident, $($g:ident),*),
        output_item = $output_item:tt,
        output_iter = $($output_iter:tt)*
    ) => {
        impl<'w, 'g, $c1, $($c),*, $g1, $($g),*> AccessGuardGroup<'w, 'g, ($c1, $($c),*)> for ($g1, $($g),*)
        where
            $g1: AccessGuard<'w, 'g, $c1>,
            $($g: AccessGuard<'w, 'g, $c>,)*
        {
            type OutputItem = $output_item;
            type OutputIter = $($output_iter)*;

            #[allow(non_snake_case)]
            fn iter((paste! { [<guard_ $g1>] }, $(paste! { [<guard_ $g>] }),*): &'g mut Self) -> Self::OutputIter {
                paste! { [<guard_ $g1>] }.iter()$(.zip(paste! { [<guard_ $g>] }.iter()))*
            }
        }
    };
}

// Enable queries like `<Read<Velocity>>::query()`
impl_AccessGuardGroup!(component = C, guard = G);
impl_IntoComponentQuery!(component = C, access = A);

// Enable queries like `<(Read<Velocity>, Write<Position>)>::query()`
impl_AccessGuardGroup!(
    component_tuple = (C1, C2),
    guard_tuple = (G1, G2),
    output_item = (G1::OutputItem, G2::OutputItem),
    output_iter = Zip<G1::OutputIter, G2::OutputIter>
);
impl_IntoComponentQuery!(component_tuple = (C1, C2), access_tuple = (A1, A2));

// Enable queries like `<(Read<Velocity>, Write<Position>, Read<Mass>)>::query()`
impl_AccessGuardGroup!(
    component_tuple = (C1, C2, C3),
    guard_tuple = (G1, G2, G3),
    // Items from the query iterator are nested tuples..
    output_item = ((G1::OutputItem, G2::OutputItem), G3::OutputItem),
    // .. because the iterator is created by zipping
    output_iter = Zip<Zip<G1::OutputIter, G2::OutputIter>, G3::OutputIter>
);
impl_IntoComponentQuery!(component_tuple = (C1, C2, C3), access_tuple = (A1, A2, A3));

// etc.
impl_AccessGuardGroup!(
    component_tuple = (C1, C2, C3, C4),
    guard_tuple = (G1, G2, G3, G4),
    output_item = (((G1::OutputItem, G2::OutputItem), G3::OutputItem), G4::OutputItem),
    output_iter = Zip<Zip<Zip<G1::OutputIter, G2::OutputIter>, G3::OutputIter>, G4::OutputIter>
);
impl_IntoComponentQuery!(
    component_tuple = (C1, C2, C3, C4),
    access_tuple = (A1, A2, A3, A4)
);

impl_AccessGuardGroup!(
    component_tuple = (C1, C2, C3, C4, C5),
    guard_tuple = (G1, G2, G3, G4, G5),
    output_item = ((((G1::OutputItem, G2::OutputItem), G3::OutputItem), G4::OutputItem), G5::OutputItem),
    output_iter = Zip<Zip<Zip<Zip<G1::OutputIter, G2::OutputIter>, G3::OutputIter>, G4::OutputIter>, G5::OutputIter>
);
impl_IntoComponentQuery!(
    component_tuple = (C1, C2, C3, C4, C5),
    access_tuple = (A1, A2, A3, A4, A5)
);

#[cfg(test)]
mod test {
    use super::*;
}
