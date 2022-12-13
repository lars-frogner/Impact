//! Macro for querying for specific sets of component types.

use proc_macro2::{Ident, Span, TokenStream};
use quote::{format_ident, quote, quote_spanned, ToTokens};
use syn::{
    bracketed,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    spanned::Spanned,
    Error, Expr, Result, Token, Type, TypeReference,
};

pub(crate) struct QueryInput {
    world: Expr,
    closure: QueryClosure,
    disallowed_list: Option<DisallowedList>,
}

struct QueryClosure {
    args: Punctuated<QueryClosureArg, Token![,]>,
    body: Expr,
}

struct QueryClosureArg {
    var: Ident,
    ty: TypeReference,
}

struct DisallowedList {
    tys: Punctuated<Type, Token![,]>,
}

pub(crate) fn query(input: QueryInput, crate_root: &Ident) -> Result<TokenStream> {
    let (world, arg_names, arg_type_refs, body, disallowed_comp_types) = input.into_components();
    let arg_names_and_type_refs: Vec<_> = arg_names.iter().zip(arg_type_refs.iter()).collect();

    let arg_types: Vec<_> = arg_type_refs
        .iter()
        .map(|type_ref| type_ref.elem.clone())
        .collect();

    verify_arg_names_unique(&arg_names)?;
    verify_arg_types_unique(&arg_types)?;
    verify_disallowed_comps_unique(&arg_types, disallowed_comp_types.as_deref())?;

    let verification_code =
        generate_input_verification(&arg_types, disallowed_comp_types.as_deref(), crate_root)?;

    let closure_args: Vec<_> = arg_names_and_type_refs
        .iter()
        .map(|(name, type_ref)| quote! { #name: #type_ref })
        .collect();

    let find_tables = match disallowed_comp_types {
        Some(disallowed_comp_types) if !disallowed_comp_types.is_empty() => {
            quote! {
                let tables = (#world).find_tables_containing_archetype_except_disallowed(
                    archetype, [#(<#disallowed_comp_types as Component>::component_id()),*]
                );
            }
        }
        _ => {
            quote! { let tables = (#world).find_tables_containing_archetype(archetype); }
        }
    };

    let table_name = Ident::new("table", Span::call_site());
    let (storage_iter_names, storage_iter_code): (Vec<_>, Vec<_>) = arg_names_and_type_refs
        .iter()
        .map(|(name, type_ref)| generate_storage_iter(&table_name, name, type_ref))
        .unzip();

    // `storage_iter_code` contains statements acquiring locks on
    // each of the involved ComponentStorages. When multiple locks
    // need to be acquired before continuing, there is a chance of
    // deadlock if another thread begins acquiring some of the same
    // locks in the opposite order. We therefore sort the statements
    // so that locks are always acquired in the same order regardless
    // of which order the component types were specified in.
    let storage_iter_code = get_storage_iter_code_sorted_by_arg_type(&arg_types, storage_iter_code);

    let (zipped_iter, nested_arg_names) = if arg_names.len() > 1 {
        (
            generate_nested_tuple(&quote! { ::core::iter::zip }, &storage_iter_names),
            generate_nested_tuple(&quote! {}, &arg_names),
        )
    } else {
        // For a single component type no zipping is needed
        (
            storage_iter_names[0].to_token_stream(),
            arg_names[0].to_token_stream(),
        )
    };

    Ok(quote! {
        // Use local scope to avoid polluting surrounding code
        {
            use #crate_root::component::Component;

            // Code for verifying argument types
            #verification_code

            // Define closure to call for each set of components
            let mut closure = |#(#closure_args),*| #body;

            let archetype = #crate_root::archetype::Archetype::new_from_component_id_arr([
                #(<#arg_types as Component>::component_id()),*
            ])
            .unwrap(); // This `unwrap` will never panic since we have verified the components

            // Obtain archetype tables matching the query
            #find_tables

            for #table_name in tables {
                // Code for acquiring read/write locks and creating iterator
                // over each component type
                #(#storage_iter_code)*

                // Loop through zipped iterators and call closure
                for #nested_arg_names in #zipped_iter {
                    closure(#(#arg_names),*);
                }
            }
        }
    })
}

impl Parse for QueryInput {
    fn parse(input: ParseStream) -> Result<Self> {
        let world = input.parse()?;
        input.parse::<Token![,]>()?;
        let closure = input.parse()?;
        let lookahead = input.lookahead1();
        let disallowed_list = if lookahead.peek(Token![,]) {
            input.parse::<Token![,]>()?;
            Some(input.parse()?)
        } else {
            None
        };
        Ok(Self {
            world,
            closure,
            disallowed_list,
        })
    }
}

impl Parse for QueryClosure {
    fn parse(input: ParseStream) -> Result<Self> {
        input.parse::<Token![|]>()?;
        let args = Punctuated::parse_separated_nonempty(input)?;
        input.parse::<Token![|]>()?;
        let body = input.parse()?;
        Ok(Self { args, body })
    }
}

impl Parse for QueryClosureArg {
    fn parse(input: ParseStream) -> Result<Self> {
        let var = input.parse()?;
        input.parse::<Token![:]>()?;
        let ty = input.parse()?;

        Ok(Self { var, ty })
    }
}

impl Parse for DisallowedList {
    fn parse(input: ParseStream) -> Result<Self> {
        input.parse::<Token![!]>()?;
        let content;
        bracketed!(content in input);
        let tys = content.parse_terminated(Type::parse)?;
        Ok(Self { tys })
    }
}

impl QueryInput {
    fn into_components(
        self,
    ) -> (
        Expr,
        Vec<Ident>,
        Vec<TypeReference>,
        Expr,
        Option<Vec<Type>>,
    ) {
        let QueryInput {
            world,
            closure,
            disallowed_list,
        } = self;
        let QueryClosure { args, body } = closure;
        let (arg_names, arg_type_refs) = args
            .into_iter()
            .map(|QueryClosureArg { var, ty }| (var, ty))
            .unzip();
        let disallowed_comp_types =
            disallowed_list.map(|DisallowedList { tys }| tys.into_iter().collect());
        (world, arg_names, arg_type_refs, body, disallowed_comp_types)
    }
}

/// Returns an error if any of the given argument names occurs
/// more than once.
fn verify_arg_names_unique(arg_names: &[Ident]) -> Result<()> {
    for (idx, name) in arg_names.iter().enumerate() {
        if arg_names[..idx].contains(name) {
            return Err(Error::new_spanned(
                name,
                format!(
                    "identifier `{}` is bound more than once in this parameter list\n\
                     used as parameter more than once",
                    name
                ),
            ));
        }
    }
    Ok(())
}

/// Returns an error if any of the given argument types occurs
/// more than once.
fn verify_arg_types_unique(arg_types: &[Box<Type>]) -> Result<()> {
    for (idx, ty) in arg_types.iter().enumerate() {
        if arg_types[..idx].contains(ty) {
            return Err(Error::new_spanned(
                &arg_types[idx],
                format!(
                    "component type `{}` occurs more than once",
                    ty.to_token_stream().to_string()
                ),
            ));
        }
    }
    Ok(())
}

fn verify_disallowed_comps_unique(
    requested_comp_types: &[Box<Type>],
    disallowed_comp_types: Option<&[Type]>,
) -> Result<()> {
    if let Some(disallowed_comp_types) = disallowed_comp_types {
        let requested_comp_types: Vec<_> = requested_comp_types
            .into_iter()
            .map(|ty| ty.as_ref())
            .collect();
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
            if requested_comp_types.contains(&ty) {
                return Err(Error::new_spanned(
                    &disallowed_comp_types[idx],
                    format!(
                        "disallowed component type `{}` is requested in closure",
                        ty.to_token_stream().to_string()
                    ),
                ));
            }
        }
    }
    Ok(())
}

fn generate_input_verification(
    arg_types: &[Box<Type>],
    disallowed_comp_types: Option<&[Type]>,
    crate_root: &Ident,
) -> Result<TokenStream> {
    let mut impl_assertions: Vec<_> = arg_types
        .iter()
        .map(|ty| create_assertion_that_type_impls_trait(ty, crate_root))
        .collect();
    if let Some(disallowed_comp_types) = disallowed_comp_types {
        impl_assertions.extend(
            disallowed_comp_types
                .iter()
                .map(|ty| create_assertion_that_type_impls_trait(ty, crate_root)),
        )
    }
    Ok(quote! {
        #(#impl_assertions)*
    })
}

fn create_assertion_that_type_impls_trait(ty: &Type, crate_root: &Ident) -> TokenStream {
    let mut ty_name = ty.to_token_stream().to_string();
    ty_name.retain(|c| c.is_alphanumeric()); // Remove possible invalid characters for identifier
    let dummy_struct_name = format_ident!("_assert_{}_impls_component", ty_name);
    quote_spanned! {ty.span()=>
        // This definition will fail to compile if the type `ty`
        // doesn't implement `Component`
        #[allow(non_camel_case_types)]
        struct #dummy_struct_name where #ty: #crate_root::component::Component;
    }
}

fn generate_storage_iter(
    table_name: &Ident,
    arg_name: &Ident,
    arg_type_ref: &TypeReference,
) -> (Ident, TokenStream) {
    let storage_name = format_ident!("{}_storage", arg_name);
    let iter_name = format_ident!("{}_iter", arg_name);
    let code = if arg_type_ref.mutability.is_some() {
        generate_mutable_storage_iter(
            table_name,
            &storage_name,
            &iter_name,
            arg_type_ref.elem.as_ref(),
        )
    } else {
        generate_immutable_storage_iter(
            table_name,
            &storage_name,
            &iter_name,
            arg_type_ref.elem.as_ref(),
        )
    };
    (iter_name, code)
}

fn generate_mutable_storage_iter(
    table_name: &Ident,
    storage_name: &Ident,
    iter_name: &Ident,
    arg_type: &Type,
) -> TokenStream {
    quote! {
        let mut #storage_name = #table_name.component_storage(
            <#arg_type as Component>::component_id()
        ).write().unwrap();
        let #iter_name = #storage_name.slice_mut::<#arg_type>().iter_mut();
    }
}

fn generate_immutable_storage_iter(
    table_name: &Ident,
    storage_name: &Ident,
    iter_name: &Ident,
    arg_type: &Type,
) -> TokenStream {
    quote! {
        let #storage_name = #table_name.component_storage(
            <#arg_type as Component>::component_id()
        ).read().unwrap();
        let #iter_name = #storage_name.slice::<#arg_type>().iter();
    }
}

fn get_storage_iter_code_sorted_by_arg_type(
    arg_types: &[Box<Type>],
    storage_iter_code: Vec<TokenStream>,
) -> Vec<TokenStream> {
    let mut type_names_and_code: Vec<_> = arg_types
        .iter()
        .map(|ty| ty.to_token_stream().to_string())
        .zip(storage_iter_code.into_iter())
        .collect();
    type_names_and_code.sort_by_key(|(ty, _)| ty.to_string());
    type_names_and_code
        .into_iter()
        .map(|(_, code)| code)
        .collect()
}

fn generate_nested_tuple<T: ToTokens>(pre_tokens: &TokenStream, values: &[T]) -> TokenStream {
    generate_nested_token_tuple(
        pre_tokens,
        values.iter().map(ToTokens::to_token_stream).collect(),
    )
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
