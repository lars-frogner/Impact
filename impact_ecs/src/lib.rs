//! Implementation of an [Entity Component System](https://en.wikipedia.org/wiki/Entity_component_system) engine.

pub mod archetype;
pub mod component;
pub mod util;
pub mod world;

/// Derive macro generating an impl of the trait Component.
pub use impact_ecs_macros::Component;

/// Macro for querying for a specific set of component types.
///
/// ```ignore
/// query!(world, |comp_a: &CompA, comp_b: &mut CompB, ..| {
///     // Do something with `comp_a`, `comp_b`, ..
/// });
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
/// # #[derive(Clone, Copy, Zeroable, Pod, Component)]
/// # struct Distance(f32);
/// # #[repr(C)]
/// # #[derive(Clone, Copy, Zeroable, Pod, Component)]
/// # struct Speed(f32);
/// #
/// let mut world = World::new();
/// let entity = world.create_entity((&Distance(0.0), &Speed(10.0)))?;
///
/// query!(world, |distance: &mut Distance, speed: &Speed| {
///     distance.0 += speed.0 * 0.1;
/// });
/// #
/// # Ok::<(), Error>(())
/// ```
///
/// # Concurrency
///
/// When `query` is invoked, it loops through each
/// [`ArchetypeTable`](archetype::ArchetypeTable) containing
/// matching components and acquires its [`RwLock`] for shared
/// access. This prevents concurrent changes to the table
/// structure while the lock is held. Next, the `RwLock`
/// guarding each of the table's
/// [`ComponentStorage`](component::ComponentStorage)s
/// matching the query is then acquired either for either shared
/// or exclusive access depending on whether an immutable or
/// mutable reference was used in front of the component type
/// in the provided closure. The locks on the table and component
/// storages are all released as soon as we move on to the next
/// table.
pub use impact_ecs_macros::query;
