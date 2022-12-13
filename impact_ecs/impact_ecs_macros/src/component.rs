//! Derive macro for the `Component` trait.

use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::DeriveInput;

pub(crate) fn impl_component(input: DeriveInput, crate_root: &Ident) -> TokenStream {
    let type_name = &input.ident;
    let generics = input.generics;
    if generics.params.is_empty() {
        // Non-generic type
        quote! {
            impl #crate_root::component::Component for #type_name {
                fn component_bytes(&self) -> #crate_root::component::ComponentByteView {
                    #crate_root::component::ComponentByteView::new_for_single_instance(
                        Self::component_id(),
                        ::std::mem::size_of::<#type_name>(),
                        bytemuck::bytes_of(self),
                    )
                }
            }
        }
    } else {
        // Generic type
        let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
        quote! {
            impl #impl_generics #crate_root::component::Component for #type_name #ty_generics #where_clause {
                fn component_bytes(&self) -> #crate_root::component::ComponentByteView {
                    #crate_root::component::ComponentByteView::new_for_single_instance(
                        Self::component_id(),
                        ::std::mem::size_of::<#type_name #ty_generics>(),
                        bytemuck::bytes_of(self),
                    )
                }
            }
        }
    }
}
