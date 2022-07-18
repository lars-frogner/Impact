//! Procedural macros for the `impact_ecs` crate.

mod component;
mod query;

use lazy_static::lazy_static;
use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use proc_macro_crate::{self, FoundCrate};
use syn::{parse_macro_input, DeriveInput};

/// Derive macro generating an impl of the trait Component.
#[proc_macro_derive(Component)]
pub fn derive_component(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    component::impl_component(input, &crate_root_ident()).into()
}

/// For use in doctests, where `crate` doesn't work as root identifier.
#[proc_macro_derive(ComponentDoctest)]
pub fn derive_component_doctest(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    component::impl_component(input, &crate_root_ident_doctest()).into()
}

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
/// ```ignore
/// # use impact_ecs::{
/// #     world::World
/// # };
/// # use impact_ecs_derive::{
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
#[proc_macro]
pub fn query(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as query::QueryInput);
    query::query(input, &crate_root_ident())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// For use in doctests, where `crate` doesn't work as root identifier.
#[proc_macro]
pub fn query_doctest(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as query::QueryInput);
    query::query(input, &crate_root_ident_doctest())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

const CRATE_NAME: &str = "impact_ecs";

lazy_static! {
    static ref CRATE_IMPORT_ROOT: String = determine_crate_import_root();
}

/// Determines whether to use `crate` or the actual crate name as root
/// for `use` statements.
fn determine_crate_import_root() -> String {
    let found_crate =
        proc_macro_crate::crate_name(CRATE_NAME).expect("impact_ecs not found in Cargo.toml");
    match found_crate {
        FoundCrate::Itself => "crate".to_string(),
        FoundCrate::Name(name) => name,
    }
}

fn crate_root_ident() -> Ident {
    Ident::new(CRATE_IMPORT_ROOT.as_str(), Span::call_site())
}

/// For doctests, the actual crate name rather than `crate` must be used
/// as root identifier.
fn crate_root_ident_doctest() -> Ident {
    Ident::new(CRATE_NAME, Span::call_site())
}
