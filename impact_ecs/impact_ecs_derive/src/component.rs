//! Derive macro for the `Component` trait.

use proc_macro2::{Ident, Span, TokenStream};
use proc_macro_crate::{self, FoundCrate};
use quote::quote;
use syn::DeriveInput;

const CRATE_NAME: &str = "impact_ecs";

pub(crate) fn impl_component(input: DeriveInput) -> TokenStream {
    // Determine whether to use `crate` or `impact_ecs` as root
    let found_crate =
        proc_macro_crate::crate_name(CRATE_NAME).expect("impact_ecs not found in Cargo.toml");
    let crate_root = Ident::new(
        match found_crate {
            FoundCrate::Itself => "crate".to_string(),
            FoundCrate::Name(name) => name,
        }
        .as_str(),
        Span::call_site(),
    );
    impl_component_with_use_root(input, &crate_root)
}

/// For use in doctests, where `crate` doesn't work as root identifier
pub(crate) fn impl_component_doctest(input: DeriveInput) -> TokenStream {
    impl_component_with_use_root(input, &Ident::new(CRATE_NAME, Span::call_site()))
}

fn impl_component_with_use_root(input: DeriveInput, root: &Ident) -> TokenStream {
    let type_name = &input.ident;
    quote! {
        impl #root::component::Component for #type_name {
            fn component_bytes(&self) -> #root::component::ComponentByteView {
                #root::component::ComponentByteView::new(
                    Self::component_id(),
                    ::std::mem::size_of::<#type_name>(),
                    bytemuck::bytes_of(self),
                )
            }
        }
    }
}
