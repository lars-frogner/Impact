//! Derive macro for the `Component` trait.

use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::{DeriveInput, Path};

pub(crate) fn impl_component(input: DeriveInput, crate_root: &Path) -> TokenStream {
    let type_name = &input.ident;

    let component_impl = generate_component_impl(type_name, crate_root);

    let descriptor_submit =
        generate_component_descriptor_submit(type_name, crate_root, &format_ident!("Standard"));

    quote! {
        #component_impl
        #descriptor_submit
    }
}

pub(crate) fn impl_setup_component(input: DeriveInput, crate_root: &Path) -> TokenStream {
    let type_name = &input.ident;

    let component_impl = generate_component_impl(type_name, crate_root);
    let setup_component_impl = generate_setup_component_impl(type_name, crate_root);

    let descriptor_submit =
        generate_component_descriptor_submit(type_name, crate_root, &format_ident!("Setup"));

    quote! {
        #component_impl
        #setup_component_impl
        #descriptor_submit
    }
}

fn generate_component_impl(type_name: &Ident, crate_root: &Path) -> TokenStream {
    let type_path_tail = format!("::{type_name}");
    let component_id = quote!(
        #crate_root::component::ComponentID::hashed_from_str(concat!(
            module_path!(),
            #type_path_tail
        ))
    );
    quote! {
        impl #crate_root::component::Component for #type_name {
            const COMPONENT_ID: #crate_root::component::ComponentID = #component_id;
        }
    }
}

fn generate_setup_component_impl(type_name: &Ident, crate_root: &Path) -> TokenStream {
    quote! {
        impl #crate_root::component::SetupComponent for #type_name {}
    }
}

fn generate_component_descriptor_submit(
    type_name: &Ident,
    crate_root: &Path,
    category: &Ident,
) -> TokenStream {
    let name = type_name.to_string();
    quote! {
        ::inventory::submit! {
            #crate_root::component::ComponentDescriptor {
                id: <#type_name as #crate_root::component::Component>::COMPONENT_ID,
                name: #name,
                category: #crate_root::component::ComponentCategory::#category,
            }
        }
    }
}
