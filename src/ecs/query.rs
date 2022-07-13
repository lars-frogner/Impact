//! Querying for sets of ECS components.

use super::{
    archetype::{Archetype, ArchetypeTable},
    component::{Component, ComponentStorage},
    world::World,
};
use anyhow::Result;
use std::{
    hash::Hash,
    marker::PhantomData,
    sync::{RwLock, RwLockReadGuard, RwLockWriteGuard},
};

/// Represents types that can be used to create a
/// [`ComponentQuery`].
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
    /// multiple component of the same type.
    fn query(world: &'w mut World) -> Result<ComponentQuery<C, Self::Guards>> {
        let archetype = Self::determine_archetype()?;
        let tables = world.find_tables_containing_archetype(archetype.id())?;
        let guards: Vec<_> = tables
            .into_iter()
            .map(Self::access_archetype_table)
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
    fn access_archetype_table(table: &'w ArchetypeTable) -> Self::Guards;
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

#[derive(Debug)]
pub struct ComponentQuery<C, G> {
    guards: Vec<G>,
    _query_type: PhantomData<C>,
}

pub trait StorageAccess<'w, 'g, C> {
    type Guard: AccessGuard<'w, 'g, C>;

    fn access(storage: &'w RwLock<ComponentStorage>) -> Self::Guard;
}

pub trait AccessGuardGroup<'w, 'g, C> {
    type OutputItem;
    type OutputIter: Iterator<Item = Self::OutputItem>;

    fn iter_mut(guards: &'g mut Self) -> Self::OutputIter;
}

pub trait AccessGuard<'w, 'g, C> {
    type OutputItem;
    type OutputIter: Iterator<Item = Self::OutputItem>;

    fn iter_mut(&'g mut self) -> Self::OutputIter;
}

#[derive(Debug)]
pub struct ReadGuard<'w, C> {
    guard: RwLockReadGuard<'w, ComponentStorage>,
    _component_type: PhantomData<C>,
}

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

    pub fn iter_mut(
        &'g mut self,
    ) -> impl Iterator<Item = <G as AccessGuardGroup<'w, 'g, C>>::OutputItem> {
        self.guards.iter_mut().flat_map(AccessGuardGroup::iter_mut)
    }
}

impl<'w, 'g, C1, C2, S1, S2> IntoComponentQuery<'w, 'g, (C1, C2)> for (S1, S2)
where
    S1: StorageAccess<'w, 'g, C1>,
    S2: StorageAccess<'w, 'g, C2>,
    C1: Component,
    C2: Component,
{
    type Guards = (S1::Guard, S2::Guard);

    fn determine_archetype() -> Result<Archetype> {
        Archetype::new_from_component_id_arr([C1::component_id(), C2::component_id()])
    }

    fn access_archetype_table(table: &'w ArchetypeTable) -> Self::Guards {
        (
            table.access_component_storage::<'w, 'g, C1, S1>(),
            table.access_component_storage::<'w, 'g, C2, S2>(),
        )
    }
}

impl<'w, 'g, C1, C2, G1, G2> AccessGuardGroup<'w, 'g, (C1, C2)> for (G1, G2)
where
    G1: AccessGuard<'w, 'g, C1>,
    G2: AccessGuard<'w, 'g, C2>,
{
    type OutputItem = (G1::OutputItem, G2::OutputItem);
    type OutputIter = std::iter::Zip<G1::OutputIter, G2::OutputIter>;

    fn iter_mut((guard_1, guard_2): &'g mut Self) -> Self::OutputIter {
        guard_1.iter_mut().zip(guard_2.iter_mut())
    }
}

impl<'w, 'g, C> StorageAccess<'w, 'g, C> for Read<C>
where
    C: 'w + Component,
{
    type Guard = ReadGuard<'w, C>;

    fn access(storage: &'w RwLock<ComponentStorage>) -> Self::Guard {
        Self::Guard::new(storage.read().unwrap())
    }
}

impl<'w, 'g, C> StorageAccess<'w, 'g, C> for Write<C>
where
    C: 'w + Component,
{
    type Guard = WriteGuard<'w, C>;

    fn access(storage: &'w RwLock<ComponentStorage>) -> Self::Guard {
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

impl<'w, 'g, C> AccessGuard<'w, 'g, C> for ReadGuard<'w, C>
where
    C: 'w + Component,
{
    type OutputItem = &'g C;
    type OutputIter = std::slice::Iter<'g, C>;

    fn iter_mut(&'g mut self) -> Self::OutputIter {
        self.guard.slice().iter()
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

impl<'w, 'g, C> AccessGuard<'w, 'g, C> for WriteGuard<'w, C>
where
    C: 'w + Component,
{
    type OutputItem = &'g mut C;
    type OutputIter = std::slice::IterMut<'g, C>;

    fn iter_mut(&'g mut self) -> Self::OutputIter {
        self.guard.slice_mut().iter_mut()
    }
}

// impl<'a, C> ArchetypeQueryBuilder<'a> for C
// where
//     C: Component,
// {
//     type OutputItem = &'a C;
//     type OutputIter = std::slice::Iter<'a, C>;

//     fn determine_archetype() -> Archetype {
//         Archetype::new_from_component_id_arr([C::component_id()])
//     }

//     fn get_component_from_table(table: &'a ArchetypeTable) -> Self::OutputIter {
//         table.get_component().iter()
//     }
// }

// impl<'a, C> StorageAccess<'a, C> for Write<C>
// where
//     C: 'a + Component,
// {
//     type OutputItem = &'a mut C;
//     type OutputIter = std::slice::IterMut<'a, C>;

//     fn access(storage: &'a ComponentStorage) -> Self::OutputIter {
//         storage.slice().iter()
//     }
// }

// impl<'a, C1, C2, C3, S1, S2, S3> ArchetypeQueryBuilder<'a, (C1, C2, C3)> for (S1, S2, S3)
// where
//     S1: StorageAccess<'a, C1>,
//     S2: StorageAccess<'a, C2>,
//     S3: StorageAccess<'a, C3>,
//     C1: Component,
//     C2: Component,
//     C3: Component,
// {
//     type OutputItem = ((&'a C1, &'a C2), &'a C3);
//     type OutputIter = std::iter::Zip<
//         std::iter::Zip<std::slice::Iter<'a, C1>, std::slice::Iter<'a, C2>>,
//         std::slice::Iter<'a, C3>,
//     >;

//     fn determine_archetype() -> Archetype {
//         Archetype::new_from_component_id_arr([
//             C1::component_id(),
//             C2::component_id(),
//             C3::component_id(),
//         ])
//     }

//     fn get_component_from_table(table: &'a ArchetypeTable) -> Self::OutputIter {
//         table
//             .get_component::<C1>()
//             .iter()
//             .zip(table.get_component::<C2>())
//             .zip(table.get_component::<C3>())
//     }
// }

// impl<'a, C> ArchetypeQuery<'a> for C
// where
//     C: Component,
// {
//     type Output = &'a [C];

//     fn get_components(table: &'a ArchetypeTable) -> Self::Output {
//         table.get_component()
//     }
// }

// impl<'a, C1, C2> ArchetypeQuery<'a> for (C1, C2)
// where
//     C1: Component,
//     C2: Component,
// {
//     type Output = std::iter::Flatten<std::iter::Map<<I as std::iter::IntoIterator>::IntoIter>>; //(&'a [C1], &'a [C2]);

//     fn archetype() -> Archetype {
//         Archetype::new_from_component_id_arr([C1::component_id(), C2::component_id()])
//     }

//     fn get_components<I>(tables: I) -> Self::Output
//     where
//         I: IntoIterator<Item = &'a ArchetypeTable>,
//     {
//         tables
//             .into_iter()
//             .map(|table| {
//                 table
//                     .get_component::<C1>()
//                     .into_iter()
//                     .zip(table.get_component::<C2>().into_iter())
//             })
//             .flatten()
//     }
// }
