//! Derive macro for the `Component` trait.

use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::{Data, DeriveInput, Error, Fields, Result};

#[cfg(feature = "enabled")]
pub(super) fn impl_roc(input: DeriveInput, crate_root: &Ident) -> Result<TokenStream> {
    let type_name = &input.ident;

    let roc_impl = generate_roc_impl(type_name, crate_root);

    let descriptor_submit = generate_roc_descriptor_submit(type_name, &input, crate_root)?;

    Ok(quote! {
        #roc_impl
        #descriptor_submit
    })
}

#[cfg(not(feature = "enabled"))]
pub(super) fn impl_roc(_input: DeriveInput, _crate_root: &Ident) -> Result<TokenStream> {
    Ok(quote! {})
}

fn generate_roc_impl(type_name: &Ident, crate_root: &Ident) -> TokenStream {
    quote! {
        impl #crate_root::roc::Roc for #type_name {}
    }
}

fn generate_roc_descriptor_submit(
    type_name: &Ident,
    input: &DeriveInput,
    crate_root: &Ident,
) -> Result<TokenStream> {
    let roc_name = type_name.to_string();
    let roc_definition = generate_roc_definition(input)?;
    Ok(quote! {
        inventory::submit! {
            #crate_root::roc::RocTypeDescriptor {
                size: ::std::mem::size_of::<#type_name>(),
                name: #roc_name,
                definition: #roc_definition,
            }
        }
    })
}

fn generate_roc_definition(input: &DeriveInput) -> Result<TokenStream> {
    let Data::Struct(data) = &input.data else {
        return Err(Error::new_spanned(
            input,
            "the `Roc` trait can only be derived for structs",
        ));
    };

    match &data.fields {
        Fields::Named(fields) => {
            todo!()
        }
        Fields::Unnamed(fields) => {
            todo!()
        }
        Fields::Unit => Err(Error::new_spanned(
            input,
            "the `Roc` trait can not be derived for unit structs",
        )),
    }
}
