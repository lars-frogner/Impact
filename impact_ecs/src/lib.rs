//! Implementation of an [Entity Component System](https://en.wikipedia.org/wiki/Entity_component_system) engine.

#![warn(missing_debug_implementations)]
#![warn(rust_2018_idioms)]
#![warn(clippy::cast_lossless)]

pub mod archetype;
pub mod component;
pub(crate) mod util;
pub mod world;

/// Derive macro generating an impl of the trait
/// [`Component`](component::Component).
pub use impact_ecs_macros::Component;

///
pub use impact_ecs_macros::prepare;

/// Macro for querying for a specific set of component types.
///
/// ```ignore
/// query!(
///     world,
///     // Call closure for entities that have both `Comp1` and `Comp2`
///     |entity: Entity, comp_1: &Comp1, comp_2: &mut Comp2| {
///         // Do something with `entity`, `comp_1` and `comp_2`
///     },
///     // Require additionaly that included entities have `MarkerComp1`
///     // and `MarkerComp2` (optional)
///     [MarkerComp1, MarkerComp2]
///     // Exclude any entity that has `Comp3` or `Comp4` (optional)
///     ![Comp3, Comp4]
/// );
/// ```
///
/// The macro takes as input the [`World`](world::World) to query
/// followed by a closure definition whose type signature specifies
/// the set of [`Component`](component::Component) types to find
/// matching instances of as well as whether immutable or mutable
/// access to each component type is required. The type of each closure
/// argument must be annotated, and has to be an immutable or mutable
/// reference to a type implementing the `Component` trait. The exception
/// is the first closure argument, which may be annotated with the
/// [`Entity`](world::Entity) type, in which case the matching `Entity`
/// will be passed to the closure along with the component instances.
/// The body of the closure specifies what to do with each set of
/// matching component instances. The closure will be called once
/// for each `Entity` that has components of all types specified.
///
/// Optionally, an array of additionaly required component types can be
/// included as an argument to the macro. Only entities that also have the
/// listed components will be included. The primary use of specifying a
/// required component here instead of in the closure signature is for
/// zero-sized marker components, which are not allowed in the closure
/// signature.
///
/// Another option is to include an array of disallowed component types
/// as an argument to the macro. The array must be prefixed with `!`.
/// If an entity has all of the required components, but also has a
/// component specified in the dissalowed component list, it will not
/// be included.
///
/// # Examples
/// ```
/// # use impact_ecs::{
/// #     world::{World, Entity}
/// # };
/// # use impact_ecs_macros::{
/// #     ComponentDoctest as Component,
/// #     query_doctest as query,
/// # };
/// # use bytemuck::{Zeroable, Pod};
/// # use anyhow::Error;
/// # use std::collections::HashSet;
/// #
/// # #[repr(C)]
/// # #[derive(Clone, Copy, Debug, PartialEq, Zeroable, Pod, Component)]
/// # struct Distance(f32);
/// # #[repr(C)]
/// # #[derive(Clone, Copy, Zeroable, Pod, Component)]
/// # struct Speed(f32);
/// # #[repr(C)]
/// # #[derive(Clone, Copy, Zeroable, Pod, Component)]
/// # struct Mass(f32);
/// # #[repr(C)]
/// # #[derive(Clone, Copy, Zeroable, Pod, Component)]
/// # struct Active;
/// # #[repr(C)]
/// # #[derive(Clone, Copy, Zeroable, Pod, Component)]
/// # struct Stuck;
/// #
/// let mut world = World::new();
/// let entity_1 = world.create_entity((&Mass(1.0), &Distance(0.0), &Speed(10.0), &Active))?;
/// let entity_2 = world.create_entity((&Mass(1.0), &Distance(0.0), &Speed(10.0)))?;
/// let entity_3 = world.create_entity((&Mass(1.0), &Distance(0.0), &Speed(10.0), &Active, &Stuck))?;
///
/// let mut matched_entities = HashSet::new();
///
/// query!(
///     world,
///     |entity: Entity, distance: &mut Distance, speed: &Speed| {
///         matched_entities.insert(entity);
///         distance.0 += speed.0;
///     },
///     [Active],
///     ![Stuck]
/// );
///
/// // `entity_1` has moved
/// assert_eq!(
///     world.entity(&entity_1).component::<Distance>().access(),
///     &Distance(10.0)
/// );
/// // `entity_2` has not moved, since it is not active
/// assert_eq!(
///     world.entity(&entity_2).component::<Distance>().access(),
///     &Distance(0.0)
/// );
/// // `entity_3` has not moved, since it is stuck
/// assert_eq!(
///     world.entity(&entity_3).component::<Distance>().access(),
///     &Distance(0.0)
/// );
/// // Proof that only `entity_1` matched the query
/// assert_eq!(matched_entities.len(), 1);
/// assert!(matched_entities.contains(&entity_1));
/// #
/// # Ok::<(), Error>(())
/// ```
///
/// # Concurrency
///
/// When `query` is invoked, it loops through each
/// [`ArchetypeTable`](archetype::ArchetypeTable) containing
/// matching components and acquires its [`RwLock`](std::sync::RwLock)
/// for shared access. This prevents concurrent changes to the table
/// structure while the lock is held. Next, the `RwLock` guarding each
/// of the table's [`ComponentStorage`](component::ComponentStorage)s
/// matching the query is acquired for either shared or
/// exclusive access depending on whether an immutable or
/// mutable reference was used in front of the component type
/// in the provided closure. The locks on the table and component
/// storages are all released as soon as we move on to the next
/// table.
pub use impact_ecs_macros::query;
