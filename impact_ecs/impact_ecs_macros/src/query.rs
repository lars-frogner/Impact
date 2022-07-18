//! Macro for querying for specific sets of component types.

use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote, quote_spanned, ToTokens};
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    spanned::Spanned,
    Error, Expr, Ident, Result, Token, Type, TypeReference,
};

pub(crate) struct QueryInput {
    world: Expr,
    closure: QueryClosure,
}

struct QueryClosure {
    args: Punctuated<QueryClosureArg, Token![,]>,
    body: Expr,
}

struct QueryClosureArg {
    var: Ident,
    ty: TypeReference,
}

/// See [`super::query`].
pub(crate) fn query(input: QueryInput, crate_root: &Ident) -> Result<TokenStream> {
    let (world, arg_names, arg_type_refs, body) = input.into_components();
    let arg_names_and_type_refs: Vec<_> = arg_names.iter().zip(arg_type_refs.iter()).collect();

    let arg_types: Vec<_> = arg_type_refs
        .iter()
        .map(|type_ref| type_ref.elem.clone())
        .collect();

    verify_arg_names_unique(&arg_names)?;
    verify_arg_types_unique(&arg_types)?;

    let verification_code = generate_input_verification(&arg_names, &arg_types, crate_root)?;

    let closure_args: Vec<_> = arg_names_and_type_refs
        .iter()
        .map(|(name, type_ref)| quote! { #name: #type_ref })
        .collect();

    let table_name = Ident::new("table", Span::call_site());
    let (storage_iter_names, storage_iter_code): (Vec<_>, Vec<_>) = arg_names_and_type_refs
        .iter()
        .map(|(name, type_ref)| generate_storage_iter(&table_name, name, type_ref))
        .unzip();

    let (zipped_iter, nested_arg_names) = if arg_names.len() > 1 {
        (
            generate_nested_tuple(&quote! {::core::iter::zip }, &storage_iter_names),
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
            let closure = |#(#closure_args),*| #body;

            let archetype = #crate_root::archetype::Archetype::new_from_component_id_arr([
                #(<#arg_types as Component>::component_id()),*
            ])
            .unwrap(); // This `unwrap` will never panic since we have verified the components

            let tables = #world.find_tables_containing_archetype(archetype);
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
        Ok(Self { world, closure })
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

impl QueryInput {
    fn into_components(self) -> (Expr, Vec<Ident>, Vec<TypeReference>, Expr) {
        let QueryInput { world, closure } = self;
        let QueryClosure { args, body } = closure;
        let (arg_names, arg_type_refs) = args
            .into_iter()
            .map(|QueryClosureArg { var, ty }| (var, ty))
            .unzip();
        (world, arg_names, arg_type_refs, body)
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
    let type_tokens: Vec<_> = arg_types
        .iter()
        .map(|ty| ty.to_token_stream().to_string())
        .collect();
    for (idx, ty) in type_tokens.iter().enumerate() {
        if type_tokens[..idx].contains(ty) {
            return Err(Error::new_spanned(
                &arg_types[idx],
                format!("component type `{}` occurs more than once", ty),
            ));
        }
    }
    Ok(())
}

fn generate_input_verification(
    arg_names: &[Ident],
    arg_types: &[Box<Type>],
    crate_root: &Ident,
) -> Result<TokenStream> {
    let impl_assertions: Vec<_> = arg_types
        .iter()
        .zip(arg_names.iter())
        .map(|(ty, name)| create_assertion_that_type_impls_trait(ty, name, crate_root))
        .collect();
    Ok(quote! {
        #(#impl_assertions)*
    })
}

fn create_assertion_that_type_impls_trait(
    ty: &Type,
    name: &Ident,
    crate_root: &Ident,
) -> TokenStream {
    let dummy_struct_name = format_ident!("_assert_{}_impls_component", name);
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
