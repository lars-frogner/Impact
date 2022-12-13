//! Implementation of an [Entity Component System](https://en.wikipedia.org/wiki/Entity_component_system) engine.

#![warn(missing_debug_implementations)]
#![warn(rust_2018_idioms)]
#![warn(clippy::cast_lossless)]

pub mod archetype;
pub mod component;
pub mod util;
pub mod world;

/// Derive macro generating an impl of the trait
/// [`Component`](component::Component).
pub use impact_ecs_macros::Component;

/// Macro for querying for a specific set of component types.
///
/// ```ignore
/// query!(
///     world,
///     // Call closure for each entity that has `CompA` and `CompB`
///     |comp_a: &CompA, comp_b: &mut CompB, ..| {
///         // Do something with `comp_a`, `comp_b`, ..
///     },
///     // Exclude any entity that has `CompC` or `CompD` (optional)
///     ![CompC, CompD]
/// );
/// ```
///
/// The macro takes as input the [`World`](world::World) to query
/// followed by a closure definition whose type signature specifies
/// the set of [`Component`](component::Component) types to find
/// matching instances of as well as whether immutable or mutable
/// access to each component type is required. The type of each closure
/// argument must be annotated, and has to be an immutable or mutable
/// reference to a type implementing the `Component` trait. The body of
/// the closure specifies what to do with each set of matching component
/// instances. The closure will be called once for each
/// [`Entity`](world::Entity) that has components of all types specified.
///
/// Optionally, an array of disallowed component types can be included as
/// a third argument to the macro. The array must be prefixed with `!`.
/// If an entity is a match based on the closure signature, but also has a
/// component specified in the dissalowed component list, it will not be
/// included.
///
/// # Examples
/// ```
/// # use impact_ecs::{
/// #     world::World
/// # };
/// # use impact_ecs_macros::{
/// #     ComponentDoctest as Component,
/// #     query_doctest as query,
/// # };
/// # use bytemuck::{Zeroable, Pod};
/// # use anyhow::Error;
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
/// # struct Stuck;
/// #
/// let mut world = World::new();
/// let entity_1 = world.create_entity((&Mass(1.0), &Distance(1.0), &Speed(10.0)))?;
/// let entity_2 = world.create_entity((&Mass(2.0), &Distance(0.0), &Speed(20.0), &Stuck))?;
///
/// query!(
///     world,
///     |distance: &mut Distance, speed: &Speed| {
///         distance.0 += speed.0 * 0.1;
///     },
///     ![Stuck]
/// );
///
/// assert_eq!(
///     world.entity(&entity_1).component::<Distance>().access(),
///     &Distance(2.0)
/// );
/// assert_eq!(
///     world.entity(&entity_2).component::<Distance>().access(),
///     &Distance(0.0)
/// );
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
