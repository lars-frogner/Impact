//! Utilities useful for various macros using a querying pattern.

use proc_macro2::{Ident, Span, TokenStream};
use quote::{format_ident, quote, quote_spanned, IdentFragment, ToTokens};
use syn::{
    bracketed,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    spanned::Spanned,
    token::Brace,
    Error, Result, Token, Type,
};

pub(crate) struct TypeList {
    pub(crate) tys: Punctuated<Type, Token![,]>,
}

impl Parse for TypeList {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let content;
        bracketed!(content in input);
        let tys = content.parse_terminated(Type::parse)?;
        Ok(Self { tys })
    }
}

pub(crate) fn parse_scope<Sc: Parse>(input: ParseStream<'_>) -> Result<Option<Sc>> {
    if input.lookahead1().peek(Brace) {
        let scope = input.parse()?;
        input.parse::<Token![,]>()?;
        Ok(Some(scope))
    } else {
        Ok(None)
    }
}

pub(crate) fn parse_state<S: Parse>(input: ParseStream<'_>) -> Result<S> {
    let state = input.parse()?;
    input.parse::<Token![,]>()?;
    Ok(state)
}

pub(crate) fn parse_closure<C: Parse>(input: ParseStream<'_>) -> Result<C> {
    input.parse()
}

pub(crate) fn parse_type_lists(
    input: ParseStream<'_>,
) -> Result<(Option<TypeList>, Option<TypeList>)> {
    if input.lookahead1().peek(Token![,]) {
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
            Ok((also_required_list, disallowed_list))
        } else {
            let also_required_list = Some(input.parse()?);
            let disallowed_list = if input.lookahead1().peek(Token![,]) {
                input.parse::<Token![,]>()?;
                input.parse::<Token![!]>()?;
                Some(input.parse()?)
            } else {
                None
            };
            Ok((also_required_list, disallowed_list))
        }
    } else {
        Ok((None, None))
    }
}

pub(crate) fn include_also_required_comp_types(
    required_arg_comp_types: &[Type],
    also_required_comp_types: Option<Vec<Type>>,
) -> Vec<Type> {
    match also_required_comp_types {
        Some(mut also_required_comp_types) => {
            also_required_comp_types.extend_from_slice(required_arg_comp_types);
            also_required_comp_types
        }
        None => required_arg_comp_types.to_vec(),
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
                    ty.to_token_stream()
                ),
            ));
        }
    }
    Ok(())
}

pub(crate) fn verify_disallowed_comps_unique(
    requested_comp_types: &[Type],
    disallowed_comp_types: &Option<Vec<Type>>,
) -> Result<()> {
    if let Some(disallowed_comp_types) = disallowed_comp_types {
        for (idx, ty) in disallowed_comp_types.iter().enumerate() {
            if disallowed_comp_types[..idx].contains(ty) {
                return Err(Error::new_spanned(
                    &disallowed_comp_types[idx],
                    format!(
                        "disallowed component type `{}` occurs more than once",
                        ty.to_token_stream()
                    ),
                ));
            }
            if requested_comp_types.contains(ty) {
                return Err(Error::new_spanned(
                    &disallowed_comp_types[idx],
                    format!(
                        "disallowed component type `{}` is also requested",
                        ty.to_token_stream()
                    ),
                ));
            }
        }
    }
    Ok(())
}

pub(crate) fn generate_input_verification_code<'a>(
    arg_comp_types: &[Type],
    requested_comp_types: &[Type],
    additional_comp_types: impl IntoIterator<Item = &'a Option<Vec<Type>>>,
    crate_root: &Ident,
) -> Result<TokenStream> {
    let mut impl_assertions: Vec<_> = arg_comp_types
        .iter()
        .map(create_assertion_that_type_is_not_zero_sized)
        .collect();

    impl_assertions.extend(
        requested_comp_types
            .iter()
            .map(|ty| create_assertion_that_type_impls_component_trait(ty, "required", crate_root)),
    );

    for (i, comp_types) in additional_comp_types.into_iter().flatten().enumerate() {
        // Use `i` as tag to avoid name clash if the same component
        // is represented multiple times
        impl_assertions.extend(
            comp_types
                .iter()
                .map(|ty| create_assertion_that_type_impls_component_trait(ty, i, crate_root)),
        );
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

pub(crate) fn create_assertion_that_type_impls_component_trait<T: IdentFragment>(
    ty: &Type,
    tag: T,
    crate_root: &Ident,
) -> TokenStream {
    let dummy_struct_name = format_ident!(
        "__{}_assert_{}_impls_component",
        tag,
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
