//! Procedural macros for the `impact_ecs` crate.

mod archetype;
mod component;
mod query;
mod querying_util;
mod setup;

use lazy_static::lazy_static;
use proc_macro::TokenStream;
use proc_macro_crate::{self, FoundCrate};
use proc_macro2::{Ident, Span};
use syn::{DeriveInput, parse_macro_input};

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

#[proc_macro]
pub fn archetype_of(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as archetype::ArchetypeOfInput);
    archetype::archetype_of(input, &crate_root_ident())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// For use in doctests, where `crate` doesn't work as root identifier.
#[proc_macro]
pub fn archetype_of_doctest(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as archetype::ArchetypeOfInput);
    archetype::archetype_of(input, &crate_root_ident_doctest())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

#[proc_macro]
pub fn setup(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as setup::SetupInput);
    setup::setup(input, &crate_root_ident())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// For use in doctests, where `crate` doesn't work as root identifier.
#[proc_macro]
pub fn setup_doctest(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as setup::SetupInput);
    setup::setup(input, &crate_root_ident_doctest())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

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
