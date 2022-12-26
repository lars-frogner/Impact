//!

use proc_macro2::{Ident, Span, TokenStream};
use quote::{format_ident, quote, quote_spanned, ToTokens};
use syn::{
    bracketed,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    spanned::Spanned,
    Error, Result, Token, Type,
};

pub(crate) struct TypeList {
    pub(crate) tys: Punctuated<Type, Token![,]>,
}

impl Parse for TypeList {
    fn parse(input: ParseStream) -> Result<Self> {
        let content;
        bracketed!(content in input);
        let tys = content.parse_terminated(Type::parse)?;
        Ok(Self { tys })
    }
}

pub(crate) fn parse_querying_input<S, C>(
    input: ParseStream,
) -> Result<(S, C, Option<TypeList>, Option<TypeList>)>
where
    S: Parse,
    C: Parse,
{
    let state = input.parse()?;

    input.parse::<Token![,]>()?;
    let closure = input.parse()?;

    let (also_required_list, disallowed_list) = if input.lookahead1().peek(Token![,]) {
        input.parse::<Token![,]>()?;
        if input.lookahead1().peek(Token![!]) {
            input.parse::<Token![!]>()?;
            let disallowed_list = Some(input.parse()?);
            let also_required_list = if input.lookahead1().peek(Token![,]) {
                input.parse::<Token![,]>()?;
                Some(input.parse()?)
            } else {
                None
            };
            (also_required_list, disallowed_list)
        } else {
            let also_required_list = Some(input.parse()?);
            let disallowed_list = if input.lookahead1().peek(Token![,]) {
                input.parse::<Token![,]>()?;
                input.parse::<Token![!]>()?;
                Some(input.parse()?)
            } else {
                None
            };
            (also_required_list, disallowed_list)
        }
    } else {
        (None, None)
    };

    Ok((state, closure, also_required_list, disallowed_list))
}

pub(crate) fn determine_all_required_comp_types(
    comp_arg_types: &[Type],
    also_required_comp_types: Option<Vec<Type>>,
) -> Vec<Type> {
    match also_required_comp_types {
        Some(mut also_required_comp_types) => {
            also_required_comp_types.extend_from_slice(comp_arg_types);
            also_required_comp_types
        }
        None => comp_arg_types.to_vec(),
    }
}

/// Returns an error if any of the given component types occurs
/// more than once.
pub(crate) fn verify_comp_types_unique(comp_types: &[Type]) -> Result<()> {
    for (idx, ty) in comp_types.iter().enumerate() {
        if comp_types[..idx].contains(ty) {
            return Err(Error::new_spanned(
                &comp_types[idx],
                format!(
                    "component type `{}` occurs more than once",
                    ty.to_token_stream().to_string()
                ),
            ));
        }
    }
    Ok(())
}

pub(crate) fn verify_disallowed_comps_unique(
    required_comp_types: &[Type],
    disallowed_comp_types: &Option<Vec<Type>>,
) -> Result<()> {
    if let Some(disallowed_comp_types) = disallowed_comp_types {
        for (idx, ty) in disallowed_comp_types.iter().enumerate() {
            if disallowed_comp_types[..idx].contains(ty) {
                return Err(Error::new_spanned(
                    &disallowed_comp_types[idx],
                    format!(
                        "disallowed component type `{}` occurs more than once",
                        ty.to_token_stream().to_string()
                    ),
                ));
            }
            if required_comp_types.contains(&ty) {
                return Err(Error::new_spanned(
                    &disallowed_comp_types[idx],
                    format!(
                        "disallowed component type `{}` is also required",
                        ty.to_token_stream().to_string()
                    ),
                ));
            }
        }
    }
    Ok(())
}

pub(crate) fn generate_input_verification_code<'a>(
    comp_arg_types: &[Type],
    required_comp_types: &[Type],
    additional_comp_types: impl IntoIterator<Item = &'a Option<Vec<Type>>>,
    crate_root: &Ident,
) -> Result<TokenStream> {
    let mut impl_assertions: Vec<_> = comp_arg_types
        .iter()
        .map(|ty| create_assertion_that_type_is_not_zero_sized(ty))
        .collect();

    impl_assertions.extend(
        required_comp_types
            .iter()
            .map(|ty| create_assertion_that_type_impls_component_trait(ty, crate_root)),
    );

    for comp_types in additional_comp_types {
        if let Some(comp_types) = comp_types {
            impl_assertions.extend(
                comp_types
                    .iter()
                    .map(|ty| create_assertion_that_type_impls_component_trait(ty, crate_root)),
            )
        }
    }

    Ok(quote! {
        #(#impl_assertions)*
    })
}

pub(crate) fn create_assertion_that_type_is_not_zero_sized(ty: &Type) -> TokenStream {
    quote_spanned! {ty.span()=>
        const _: () = assert!(::std::mem::size_of::<#ty>() != 0, "Zero-sized component in closure signature");
    }
}

pub(crate) fn create_assertion_that_type_impls_component_trait(
    ty: &Type,
    crate_root: &Ident,
) -> TokenStream {
    let dummy_struct_name = format_ident!(
        "__assert_{}_impls_component",
        type_to_valid_ident_string(ty)
    );
    quote_spanned! {ty.span()=>
        // This definition will fail to compile if the type `ty`
        // doesn't implement `Component`
        #[allow(non_camel_case_types)]
        struct #dummy_struct_name where #ty: #crate_root::component::Component;
    }
}

pub(crate) fn generate_archetype_creation_code(
    required_comp_types: &[Type],
    crate_root: &Ident,
) -> (Ident, TokenStream) {
    let archetype_name = Ident::new("_archetype_internal__", Span::call_site());
    let archetype_creation_code = quote! {
        let #archetype_name = #crate_root::archetype::Archetype::new_from_component_id_arr(
            [#(<#required_comp_types as #crate_root::component::Component>::component_id()),*]
        )
        .unwrap(); // This `unwrap` should never panic since we have verified the components
    };
    (archetype_name, archetype_creation_code)
}

pub(crate) fn generate_nested_tuple<'a, T: ToTokens + 'a>(
    pre_tokens: &TokenStream,
    values: impl Iterator<Item = &'a T>,
) -> TokenStream {
    generate_nested_token_tuple(pre_tokens, values.map(ToTokens::to_token_stream).collect())
}

pub(crate) fn type_to_valid_ident_string(ty: &Type) -> String {
    let mut ty_name = ty.to_token_stream().to_string();
    ty_name.retain(|c| c.is_alphanumeric()); // Remove possible invalid characters for identifier
    ty_name
}

fn generate_nested_token_tuple(
    pre_tokens: &TokenStream,
    mut items: Vec<TokenStream>,
) -> TokenStream {
    assert!(items.len() > 1);
    let tail = items.pop().unwrap();
    let head = if items.len() > 1 {
        generate_nested_token_tuple(pre_tokens, items)
    } else {
        items.pop().unwrap()
    };
    quote! { #pre_tokens (#head, #tail) }
}
