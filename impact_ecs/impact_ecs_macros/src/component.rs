//! Derive macro for the `Component` trait.

use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::DeriveInput;

pub(crate) fn impl_component(input: DeriveInput, crate_root: &Ident) -> TokenStream {
    let type_name = &input.ident;
    quote! {
        impl #crate_root::component::Component for #type_name {
            fn component_bytes(&self) -> #crate_root::component::ComponentByteView {
                #crate_root::component::ComponentByteView::new(
                    Self::component_id(),
                    ::std::mem::size_of::<#type_name>(),
                    bytemuck::bytes_of(self),
                )
            }
        }
    }
}
