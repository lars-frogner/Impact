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
    also_required_list: Option<TypeList>,
    disallowed_list: Option<TypeList>,
}

struct QueryClosure {
    entity_arg: Option<EntityClosureArg>,
    args: Punctuated<QueryClosureArg, Token![,]>,
    body: Expr,
}

struct EntityClosureArg {
    var: Ident,
    ty: Type,
}

struct QueryClosureArg {
    var: Ident,
    ty: TypeReference,
}

struct TypeList {
    tys: Punctuated<Type, Token![,]>,
}

pub(crate) fn query(input: QueryInput, crate_root: &Ident) -> Result<TokenStream> {
    let (
        world,
        entity_arg,
        comp_arg_names,
        comp_arg_type_refs,
        body,
        also_required_comp_types,
        disallowed_comp_types,
    ) = input.into_components();
    let comp_arg_names_and_type_refs: Vec<_> = comp_arg_names
        .iter()
        .zip(comp_arg_type_refs.iter())
        .collect();

    let comp_arg_types: Vec<_> = comp_arg_type_refs
        .iter()
        .map(|type_ref| type_ref.elem.as_ref().to_owned())
        .collect();

    let required_comp_types: Vec<_> = match also_required_comp_types {
        Some(mut also_required_comp_types) => {
            also_required_comp_types.extend_from_slice(&comp_arg_types);
            also_required_comp_types
        }
        None => comp_arg_types.clone(),
    };

    verify_required_comp_types_unique(&required_comp_types)?;
    verify_disallowed_comps_unique(&required_comp_types, disallowed_comp_types.as_deref())?;

    let verification_code = generate_input_verification(
        &comp_arg_types,
        &required_comp_types,
        disallowed_comp_types.as_deref(),
        crate_root,
    )?;

    let (mut arg_names, mut closure_args) = match &entity_arg {
        Some(EntityClosureArg { var, ty }) => (vec![var.clone()], vec![quote! { #var: #ty }]),
        None => (Vec::new(), Vec::new()),
    };
    arg_names.extend_from_slice(&comp_arg_names);
    closure_args.extend(
        comp_arg_names_and_type_refs
            .iter()
            .map(|(name, type_ref)| quote! { #name: #type_ref }),
    );

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

    let mut iter_names = Vec::with_capacity(closure_args.len());
    let mut iter_code = Vec::with_capacity(closure_args.len());

    if let Some(EntityClosureArg { var, ty: _ }) = &entity_arg {
        let (entity_iter_name, entity_iter_code) = generate_entity_iter(&table_name, var);
        iter_names.push(entity_iter_name);
        iter_code.push(entity_iter_code);
    }

    let (mut storage_iter_names, storage_iter_code): (Vec<_>, Vec<_>) =
        comp_arg_names_and_type_refs
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
    let mut storage_iter_code =
        get_storage_iter_code_sorted_by_arg_type(&comp_arg_types, storage_iter_code);

    iter_names.append(&mut storage_iter_names);
    iter_code.append(&mut storage_iter_code);

    let (zipped_iter, nested_arg_names) = if arg_names.len() > 1 {
        (
            generate_nested_tuple(&quote! { ::core::iter::zip }, &iter_names),
            generate_nested_tuple(&quote! {}, &arg_names),
        )
    } else {
        // For a single component type no zipping is needed
        (
            iter_names[0].to_token_stream(),
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

            // Create archetype for all required components
            let archetype = #crate_root::archetype::Archetype::new_from_component_id_arr([
                #(<#required_comp_types as Component>::component_id()),*
            ])
            .unwrap(); // This `unwrap` should never panic since we have verified the components

            // Obtain archetype tables matching the query
            #find_tables

            for #table_name in tables {
                // Code for acquiring read/write locks and creating iterator
                // over each component type
                #(#iter_code)*

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

        Ok(Self {
            world,
            closure,
            also_required_list,
            disallowed_list,
        })
    }
}

impl Parse for QueryClosure {
    fn parse(input: ParseStream) -> Result<Self> {
        input.parse::<Token![|]>()?;
        let mut args = Punctuated::new();
        let first_arg_var = input.parse()?;
        input.parse::<Token![:]>()?;
        let entity_arg = match input.parse()? {
            Type::Reference(first_arg_ty) => {
                args.push(QueryClosureArg {
                    var: first_arg_var,
                    ty: first_arg_ty,
                });
                None
            }
            entity_ty => Some(EntityClosureArg {
                var: first_arg_var,
                ty: entity_ty,
            }),
        };
        if input.lookahead1().peek(Token![,]) {
            input.parse::<Token![,]>()?;
            args.extend(
                Punctuated::<QueryClosureArg, Token![,]>::parse_separated_nonempty(input)?
                    .into_iter(),
            );
        }
        input.parse::<Token![|]>()?;
        let body = input.parse()?;
        Ok(Self {
            entity_arg,
            args,
            body,
        })
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

impl Parse for TypeList {
    fn parse(input: ParseStream) -> Result<Self> {
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
        Option<EntityClosureArg>,
        Vec<Ident>,
        Vec<TypeReference>,
        Expr,
        Option<Vec<Type>>,
        Option<Vec<Type>>,
    ) {
        let QueryInput {
            world,
            closure,
            also_required_list,
            disallowed_list,
        } = self;

        let QueryClosure {
            entity_arg,
            args,
            body,
        } = closure;

        let (arg_names, arg_type_refs) = args
            .into_iter()
            .map(|QueryClosureArg { var, ty }| (var, ty))
            .unzip();

        let also_required_comp_types =
            also_required_list.map(|TypeList { tys }| tys.into_iter().collect());

        let disallowed_comp_types =
            disallowed_list.map(|TypeList { tys }| tys.into_iter().collect());

        (
            world,
            entity_arg,
            arg_names,
            arg_type_refs,
            body,
            also_required_comp_types,
            disallowed_comp_types,
        )
    }
}

/// Returns an error if any of the given required component types occurs
/// more than once.
fn verify_required_comp_types_unique(required_comp_types: &[Type]) -> Result<()> {
    for (idx, ty) in required_comp_types.iter().enumerate() {
        if required_comp_types[..idx].contains(ty) {
            return Err(Error::new_spanned(
                &required_comp_types[idx],
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
    required_comp_types: &[Type],
    disallowed_comp_types: Option<&[Type]>,
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

fn generate_input_verification(
    comp_arg_types: &[Type],
    required_comp_types: &[Type],
    disallowed_comp_types: Option<&[Type]>,
    crate_root: &Ident,
) -> Result<TokenStream> {
    let mut impl_assertions: Vec<_> = comp_arg_types
        .iter()
        .map(|ty| create_assertion_that_type_is_not_zero_sized(ty))
        .collect();

    impl_assertions.extend(
        required_comp_types
            .iter()
            .map(|ty| create_assertion_that_type_impls_trait(ty, crate_root)),
    );

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

fn create_assertion_that_type_is_not_zero_sized(ty: &Type) -> TokenStream {
    quote_spanned! {ty.span()=>
        const _: () = assert!(::std::mem::size_of::<#ty>() != 0, "Zero-sized component in closure signature");
    }
}

fn create_assertion_that_type_impls_trait(ty: &Type, crate_root: &Ident) -> TokenStream {
    let mut ty_name = ty.to_token_stream().to_string();
    ty_name.retain(|c| c.is_alphanumeric()); // Remove possible invalid characters for identifier
    let dummy_struct_name = format_ident!("__assert_{}_impls_component", ty_name);
    quote_spanned! {ty.span()=>
        // This definition will fail to compile if the type `ty`
        // doesn't implement `Component`
        #[allow(non_camel_case_types)]
        struct #dummy_struct_name where #ty: #crate_root::component::Component;
    }
}

fn generate_entity_iter(table_name: &Ident, entity_arg_name: &Ident) -> (Ident, TokenStream) {
    let iter_name = format_ident!("{}_iter_internal__", entity_arg_name);
    let code = quote! {
        let #iter_name = #table_name.all_entities();
    };
    (iter_name, code)
}

fn generate_storage_iter(
    table_name: &Ident,
    arg_name: &Ident,
    arg_type_ref: &TypeReference,
) -> (Ident, TokenStream) {
    let storage_name = format_ident!("{}_storage_internal__", arg_name);
    let iter_name = format_ident!("{}_iter_internal__", arg_name);
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
    arg_types: &[Type],
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
