//! Procedural macros for the `impact_ecs` crate.

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
    rust_2018_idioms,
    unexpected_cfgs
)]

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
