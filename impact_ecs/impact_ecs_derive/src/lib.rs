//! Derive macros for the `impact_ecs` crate.

mod component;

use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(Component)]
pub fn derive_component(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    component::impl_component(input).into()
}

#[proc_macro_derive(ComponentDoctest)]
pub fn derive_component_doctest(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    component::impl_component_doctest(input).into()
}
