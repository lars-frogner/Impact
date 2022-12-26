//! Macro for performing setup on components before
//! creating entities.

use crate::querying_util;
use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    Result, Token, Type,
};

pub(crate) struct ArchetypeOfInput {
    component_types: Punctuated<Type, Token![,]>,
}

pub(crate) fn archetype_of(input: ArchetypeOfInput, crate_root: &Ident) -> Result<TokenStream> {
    let component_types: Vec<_> = input.component_types.into_iter().collect();

    querying_util::verify_comp_types_unique(&component_types)?;

    let assertions = component_types
        .iter()
        .map(|ty| querying_util::create_assertion_that_type_impls_component_trait(ty, crate_root));

    Ok(quote! {
        // Use local scope to avoid polluting surrounding code
        {
            #(#assertions)*
            #crate_root::archetype::Archetype::new_from_component_id_arr(
                [#(<#component_types as #crate_root::component::Component>::component_id()),*]
            )
            .unwrap() // This `unwrap` should never panic since we have verified the components
        }
    })
}

impl Parse for ArchetypeOfInput {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Self {
            component_types: Punctuated::parse_terminated(input)?,
        })
    }
}
