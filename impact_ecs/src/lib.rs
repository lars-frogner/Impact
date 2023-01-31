//! Implementation of an [Entity Component System](https://en.wikipedia.org/wiki/Entity_component_system) engine.

#![warn(
    clippy::all,
    clippy::await_holding_lock,
    clippy::cast_lossless,
    clippy::char_lit_as_u8,
    clippy::checked_conversions,
    clippy::dbg_macro,
    clippy::debug_assert_with_mut_call,
    clippy::doc_markdown,
    clippy::empty_enum,
    clippy::enum_glob_use,
    clippy::exit,
    clippy::expl_impl_clone_on_copy,
    clippy::explicit_deref_methods,
    clippy::explicit_into_iter_loop,
    clippy::fallible_impl_from,
    clippy::filter_map_next,
    clippy::flat_map_option,
    clippy::float_cmp_const,
    clippy::fn_params_excessive_bools,
    clippy::from_iter_instead_of_collect,
    clippy::if_let_mutex,
    clippy::implicit_clone,
    clippy::imprecise_flops,
    clippy::inefficient_to_string,
    clippy::invalid_upcast_comparisons,
    clippy::large_digit_groups,
    clippy::large_stack_arrays,
    clippy::large_types_passed_by_value,
    clippy::let_unit_value,
    clippy::linkedlist,
    clippy::lossy_float_literal,
    clippy::macro_use_imports,
    clippy::manual_ok_or,
    clippy::map_err_ignore,
    clippy::map_flatten,
    clippy::map_unwrap_or,
    clippy::match_on_vec_items,
    clippy::match_same_arms,
    clippy::match_wild_err_arm,
    clippy::match_wildcard_for_single_variants,
    clippy::mem_forget,
    clippy::mismatched_target_os,
    clippy::missing_enforced_import_renames,
    clippy::mut_mut,
    clippy::mutex_integer,
    clippy::needless_borrow,
    clippy::needless_continue,
    clippy::needless_for_each,
    clippy::option_option,
    clippy::path_buf_push_overwrite,
    clippy::ptr_as_ptr,
    clippy::rc_mutex,
    clippy::ref_option_ref,
    clippy::rest_pat_in_fully_bound_structs,
    clippy::same_functions_in_if_condition,
    clippy::semicolon_if_nothing_returned,
    clippy::single_match_else,
    clippy::string_add_assign,
    clippy::string_add,
    clippy::string_lit_as_bytes,
    clippy::string_to_string,
    clippy::todo,
    clippy::trait_duplication_in_bounds,
    clippy::unimplemented,
    clippy::unnested_or_patterns,
    clippy::unused_self,
    clippy::useless_transmute,
    clippy::verbose_file_reads,
    clippy::zero_sized_map_values,
    future_incompatible,
    missing_debug_implementations,
    nonstandard_style,
    rust_2018_idioms
)]

pub mod archetype;
pub mod component;
pub mod world;

/// Derive macro generating an impl of the trait
/// [`Component`](component::Component).
pub use impact_ecs_macros::Component;

/// Creates a new [`Archetype`](archetype::Archetype) defined by
/// the given component types.
///
/// Providing no components still gives a valid archetype.
/// All provided types must implement the [`Component`](component::Component)
/// trait, and no type can be repeated. The order in which
/// the component types are specified does not affect the result.
///
/// ```ignore
/// archetype_of!(Comp1, Comp2, ...)
/// ```
pub use impact_ecs_macros::archetype_of;

/// Macro for performing setup on components before creating entities.
///
/// ```ignore
/// setup!(
///     {
///         // Setup to run once if criteria are matched (optional)
///         // ....
///     },
///     // Identifier for the `ArchetypeComponentStorage` to match on
///     components,
///     // Call closure for each component instance if `components` has both
///     // `Comp1` and `Comp2`
///     |comp_1: &Comp1, comp_2: &mut Comp2| -> (Comp3, Comp4) {
///         // Do something with `entity`, `comp_1` and `comp_2`
///         // ...
///         // Return instances of `comp_3` and `comp_4` to add to `components`
///         (comp_3, comp_4)
///     },
///     // Require additionaly that `components` has `MarkerComp1` and
///     // `MarkerComp2` (optional)
///     [MarkerComp1, MarkerComp2]
///     // Do not call the closure if `components` has `Comp3` or `Comp4`
///     // (optional)
///     ![Comp3, Comp4]
/// );
/// ```
///
/// The macro takes as input an
/// [`ArchetypeComponentStorage`](archetype::ArchetypeComponentStorage)
/// wrapping a set of component instances, followed by a closure
/// definition whose type signature specifies the set of
/// [`Component`](component::Component) types to look for in the set of existing
/// components as well as the component types the closure will return instances
/// of for inclusion in the `ArchetypeComponentStorage`. The type of
/// each closure argument must be annotated, and has to be an immutable reference
/// to a type implementing the `Component` trait. If the closure returns anything,
/// it must be a single value or a tuple of values implementing the `Component`
/// trait, and the return type has to be annotated in the closure signature.
///
/// The body of the closure specifies what to do with each set of matching
/// component instances present in the `ArchetypeComponentStorage`. The closure will only
/// be called if the `ArchetypeComponentStorage` has the requested component types
/// and if so it will be called once with each set of requested component instances.
/// Any instances of a new component type that the closure returns will be added under a new
/// component type in the `ArchetypeComponentStorage`. Any returned instances of an already existing
/// component type will overwrite the existing instances for that component type.
///
/// Optionally, an array of additionaly required component types can be
/// included as an argument to the macro. The closure will only be called
/// if the `ArchetypeComponentStorage` also has these component types.
/// The primary use of specifying a required component here instead of in
/// the closure signature is for zero-sized marker components, which are
/// not allowed in the closure signature.
///
/// Another option is to include an array of disallowed component types
/// as an argument to the macro. The array must be prefixed with `!`.
/// If the `ArchetypeComponentStorage` has all of the required components, but
/// also has a component type specified in the dissalowed component
/// list, the closure will not be called.
///
/// Finally, arbitrary code to run once if (and only if) the `ArchetypeComponentStorage`
/// has all of the required components can be specified inside curly
/// braces as the first argument to the macro. This code will be included in the
/// parent scope of the closure, and will go out of scope when all closure calls
/// have been executed.
///
/// # Examples
/// ```
/// # use impact_ecs::{
/// #     archetype::ArchetypeComponentStorage,
/// #     world::{World, Entity}
/// # };
/// # use impact_ecs_macros::{
/// #     ComponentDoctest as Component,
/// #     setup_doctest as setup,
/// # };
/// # use bytemuck::{Zeroable, Pod};
/// # use anyhow::Error;
/// # use std::collections::HashSet;
/// #
/// # #[repr(C)]
/// # #[derive(Clone, Copy, Zeroable, Pod, Component)]
/// # struct Flux(f32);
/// # #[repr(C)]
/// # #[derive(Clone, Copy, Zeroable, Pod, Component)]
/// # struct Area(f32);
/// # #[repr(C)]
/// # #[derive(Clone, Copy, Debug, PartialEq, Zeroable, Pod, Component)]
/// # struct Luminosity(f32);
/// # #[repr(C)]
/// # #[derive(Clone, Copy, Zeroable, Pod, Component)]
/// # struct Light;
/// # #[repr(C)]
/// # #[derive(Clone, Copy, Zeroable, Pod, Component)]
/// # struct Disabled;
/// #
/// fn setup_area_lights(components: &mut ArchetypeComponentStorage, contains_area_lights: &mut bool) {
///     setup!(
///         {
///             *contains_area_lights = true;
///         },
///         components,
///         |flux: &Flux, area: &Area| -> Luminosity {
///             Luminosity(flux.0 * area.0)
///         },
///         [Light],
///         ![Disabled]
///     );
/// }
///
/// let mut world = World::new();
/// let mut components = ArchetypeComponentStorage::try_from_view(
///     (&[Light, Light], &[Flux(1.0), Flux(5.0)], &[Area(2.0), Area(2.0)])
/// )?;
/// let mut contains_area_lights = false;
///
/// setup_area_lights(&mut components, &mut contains_area_lights);
///
/// let entities = world.create_entities(components)?;
///
/// assert!(contains_area_lights);
/// assert_eq!(
///     world.entity(&entities[0]).component::<Luminosity>().access(),
///     &Luminosity(2.0)
/// );
/// assert_eq!(
///     world.entity(&entities[1]).component::<Luminosity>().access(),
///     &Luminosity(10.0)
/// );
/// #
/// # Ok::<(), Error>(())
/// ```
///
pub use impact_ecs_macros::setup;

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
///
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
