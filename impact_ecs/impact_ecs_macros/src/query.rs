//! Macro for querying for specific sets of component types.

use crate::querying_util::{self, TypeList};
use proc_macro2::{Ident, Span, TokenStream};
use quote::{ToTokens, format_ident, quote};
use syn::{
    Expr, Result, Token, Type, TypeReference,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
};

pub(crate) struct QueryInput {
    world: Expr,
    closure: QueryClosure,
    also_required_list: Option<TypeList>,
    disallowed_list: Option<TypeList>,
}

struct QueryClosure {
    entity_arg: Option<EntityClosureArg>,
    comp_args: Punctuated<QueryCompClosureArg, Token![,]>,
    body: Expr,
}

struct QueryClosureArg<T> {
    var: Ident,
    ty: T,
}

type EntityClosureArg = QueryClosureArg<Type>;
type QueryCompClosureArg = QueryClosureArg<TypeReference>;

struct ProcessedQueryInput {
    world: Expr,
    closure_body: Expr,
    entity_arg: Option<EntityClosureArg>,
    comp_arg_names: Vec<Ident>,
    comp_arg_type_refs: Vec<TypeReference>,
    comp_arg_types: Vec<Type>,
    disallowed_comp_types: Option<Vec<Type>>,
    required_comp_types: Vec<Type>,
    closure_arg_names: Vec<Ident>,
    full_closure_args: Vec<TokenStream>,
}

pub(crate) fn query(input: QueryInput, crate_root: &Ident) -> Result<TokenStream> {
    let input = input.process();

    querying_util::verify_comp_types_unique(&input.required_comp_types)?;
    querying_util::verify_disallowed_comps_unique(
        &input.required_comp_types,
        &input.disallowed_comp_types,
    )?;

    let input_verification_code = querying_util::generate_input_verification_code(
        &input.comp_arg_types,
        &input.required_comp_types,
        [&input.disallowed_comp_types],
        crate_root,
    )?;

    let (closure_name, closure_def_code) =
        generate_closure_def_code(&input.full_closure_args, &input.closure_body);

    let (archetype_name, archetype_creation_code) =
        querying_util::generate_archetype_creation_code(&input.required_comp_types, crate_root);

    let (tables_iter_name, table_search_code) = generate_table_search_code(
        &input.world,
        &input.disallowed_comp_types,
        &archetype_name,
        crate_root,
    );

    let (table_var_name, table_iter_names, table_iter_code) =
        generate_table_iter_names_and_code(&input.entity_arg, &input.full_closure_args);

    let (storage_iter_names, storage_iter_code) = generate_storage_iter_names_and_code(
        &table_var_name,
        &input.comp_arg_names,
        &input.comp_arg_type_refs,
        crate_root,
    );

    let closure_call_code = generate_closure_call_code(
        &closure_name,
        &input.closure_arg_names,
        &table_iter_names,
        &storage_iter_names,
    );

    Ok(quote! {
        // Use local scope to avoid polluting surrounding code
        {
            // Code for verifying argument types
            #input_verification_code

            // Define closure to call for each set of components
            #closure_def_code

            // Create archetype for all required components
            #archetype_creation_code

            // Obtain archetype tables matching the query
            #table_search_code

            for #table_var_name in #tables_iter_name {
                // Code for acquiring read/write locks and creating iterator
                // over each component type
                #(#table_iter_code)*
                #(#storage_iter_code)*

                // Loop through zipped iterators and call closure
                #closure_call_code
            }
        }
    })
}

impl Parse for QueryInput {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let world = querying_util::parse_state(input)?;
        let closure = querying_util::parse_closure(input)?;
        let (also_required_list, disallowed_list) = querying_util::parse_type_lists(input)?;
        Ok(Self {
            world,
            closure,
            also_required_list,
            disallowed_list,
        })
    }
}

impl Parse for QueryClosure {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        input.parse::<Token![|]>()?;
        let mut comp_args = Punctuated::new();
        let first_arg_var = input.parse()?;
        input.parse::<Token![:]>()?;
        let entity_arg = match input.parse()? {
            Type::Reference(first_arg_ty) => {
                comp_args.push(QueryCompClosureArg {
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
            comp_args.extend(
                Punctuated::<QueryCompClosureArg, Token![,]>::parse_separated_nonempty(input)?,
            );
        }
        input.parse::<Token![|]>()?;
        let body = input.parse()?;
        Ok(Self {
            entity_arg,
            comp_args,
            body,
        })
    }
}

impl<T: Parse> Parse for QueryClosureArg<T> {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let var = input.parse()?;
        input.parse::<Token![:]>()?;
        let ty = input.parse()?;

        Ok(Self { var, ty })
    }
}

impl QueryInput {
    fn process(self) -> ProcessedQueryInput {
        let Self {
            world,
            closure,
            also_required_list,
            disallowed_list,
        } = self;

        let QueryClosure {
            entity_arg,
            comp_args,
            body: closure_body,
        } = closure;

        let (comp_arg_names, comp_arg_type_refs): (Vec<_>, Vec<_>) = comp_args
            .into_iter()
            .map(|QueryCompClosureArg { var, ty }| (var, ty))
            .unzip();

        let comp_arg_types: Vec<_> = comp_arg_type_refs
            .iter()
            .map(|type_ref| type_ref.elem.as_ref().clone())
            .collect();

        let also_required_comp_types =
            also_required_list.map(|TypeList { tys }| tys.into_iter().collect());

        let disallowed_comp_types =
            disallowed_list.map(|TypeList { tys }| tys.into_iter().collect());

        let required_comp_types = querying_util::include_also_required_comp_types(
            &comp_arg_types,
            also_required_comp_types,
        );

        let (closure_arg_names, full_closure_args) =
            determine_all_closure_args(&comp_arg_names, &comp_arg_type_refs, &entity_arg);

        ProcessedQueryInput {
            world,
            closure_body,
            entity_arg,
            comp_arg_names,
            comp_arg_type_refs,
            comp_arg_types,
            disallowed_comp_types,
            required_comp_types,
            closure_arg_names,
            full_closure_args,
        }
    }
}

fn determine_all_closure_args(
    comp_arg_names: &[Ident],
    comp_arg_type_refs: &[TypeReference],
    entity_arg: &Option<EntityClosureArg>,
) -> (Vec<Ident>, Vec<TokenStream>) {
    let (mut arg_names, mut full_args) = match entity_arg {
        Some(EntityClosureArg { var, ty }) => (vec![var.clone()], vec![quote! { #var: #ty }]),
        None => (Vec::new(), Vec::new()),
    };
    arg_names.extend_from_slice(comp_arg_names);
    full_args.extend(
        comp_arg_names
            .iter()
            .zip(comp_arg_type_refs.iter())
            .map(|(name, type_ref)| quote! { #name: #type_ref }),
    );
    (arg_names, full_args)
}

fn generate_closure_def_code(
    full_closure_args: &[TokenStream],
    closure_body: &Expr,
) -> (Ident, TokenStream) {
    let closure_name = Ident::new("_closure_internal__", Span::call_site());
    let closure_def_code = quote! {
        let mut #closure_name = |#(#full_closure_args),*| #closure_body;
    };
    (closure_name, closure_def_code)
}

fn generate_table_search_code(
    world: &Expr,
    disallowed_comp_types: &Option<Vec<Type>>,
    archetype_name: &Ident,
    crate_root: &Ident,
) -> (Ident, TokenStream) {
    let tables_iter_name = Ident::new("_tables_internal__", Span::call_site());
    let table_search_code = match disallowed_comp_types {
        Some(disallowed_comp_types) if !disallowed_comp_types.is_empty() => {
            quote! {
                let #tables_iter_name = (#world).find_tables_containing_archetype_except_disallowed(
                    #archetype_name, [#(<#disallowed_comp_types as #crate_root::component::Component>::component_id()),*]
                );
            }
        }
        _ => {
            quote! { let #tables_iter_name = (#world).find_tables_containing_archetype(#archetype_name); }
        }
    };
    (tables_iter_name, table_search_code)
}

fn generate_table_iter_names_and_code(
    entity_arg: &Option<EntityClosureArg>,
    full_closure_args: &[TokenStream],
) -> (Ident, Vec<Ident>, Vec<TokenStream>) {
    let table_var_name = Ident::new("_table_internal__", Span::call_site());
    let mut iter_names = Vec::with_capacity(full_closure_args.len());
    let mut iter_code = Vec::with_capacity(full_closure_args.len());

    if let Some(EntityClosureArg { var, ty: _ }) = entity_arg {
        let (entity_iter_name, entity_iter_code) = generate_entity_iter_code(&table_var_name, var);
        iter_names.push(entity_iter_name);
        iter_code.push(entity_iter_code);
    }
    (table_var_name, iter_names, iter_code)
}

fn generate_storage_iter_names_and_code(
    table_var_name: &Ident,
    comp_arg_names: &[Ident],
    comp_arg_type_refs: &[TypeReference],
    crate_root: &Ident,
) -> (Vec<Ident>, Vec<TokenStream>) {
    let (iter_names, iter_code): (Vec<_>, Vec<_>) = comp_arg_names
        .iter()
        .zip(comp_arg_type_refs.iter())
        .map(|(name, type_ref)| {
            generate_storage_iter_code(table_var_name, name, type_ref, crate_root)
        })
        .unzip();

    // `iter_code` contains statements acquiring locks on
    // each of the involved ComponentStorages. When multiple locks
    // need to be acquired before continuing, there is a chance of
    // deadlock if another thread begins acquiring some of the same
    // locks in the opposite order. We therefore sort the statements
    // so that locks are always acquired in the same order regardless
    // of which order the component types were specified in.
    let iter_code = get_storage_iter_code_sorted_by_arg_type(comp_arg_type_refs, iter_code);

    (iter_names, iter_code)
}

fn generate_closure_call_code(
    closure_name: &Ident,
    closure_arg_names: &[Ident],
    table_iter_names: &[Ident],
    storage_iter_names: &[Ident],
) -> TokenStream {
    let mut iter_names = table_iter_names.iter().chain(storage_iter_names.iter());

    let (zipped_iter, nested_arg_names) = if closure_arg_names.len() > 1 {
        (
            querying_util::generate_nested_tuple(&quote! { ::core::iter::zip }, iter_names),
            querying_util::generate_nested_tuple(&quote! {}, closure_arg_names.iter()),
        )
    } else {
        // For a single component type, no zipping is needed
        (
            iter_names.next().unwrap().to_token_stream(),
            closure_arg_names[0].to_token_stream(),
        )
    };
    quote! {
        for #nested_arg_names in #zipped_iter {
            #closure_name(#(#closure_arg_names),*);
        }
    }
}

fn generate_entity_iter_code(table_name: &Ident, entity_arg_name: &Ident) -> (Ident, TokenStream) {
    let iter_name = format_ident!("{}_iter_internal__", entity_arg_name);
    let code = quote! {
        let #iter_name = #table_name.all_entities();
    };
    (iter_name, code)
}

fn generate_storage_iter_code(
    table_name: &Ident,
    arg_name: &Ident,
    arg_type_ref: &TypeReference,
    crate_root: &Ident,
) -> (Ident, TokenStream) {
    let storage_name = format_ident!("{}_storage_internal__", arg_name);
    let iter_name = format_ident!("{}_iter_internal__", arg_name);
    let code = if arg_type_ref.mutability.is_some() {
        generate_mutable_storage_iter(
            table_name,
            &storage_name,
            &iter_name,
            arg_type_ref.elem.as_ref(),
            crate_root,
        )
    } else {
        generate_immutable_storage_iter(
            table_name,
            &storage_name,
            &iter_name,
            arg_type_ref.elem.as_ref(),
            crate_root,
        )
    };
    (iter_name, code)
}

fn generate_mutable_storage_iter(
    table_name: &Ident,
    storage_name: &Ident,
    iter_name: &Ident,
    arg_type: &Type,
    crate_root: &Ident,
) -> TokenStream {
    quote! {
        let mut #storage_name = #table_name.component_storage(
            <#arg_type as #crate_root::component::Component>::component_id()
        ).write().unwrap();
        let #iter_name = #storage_name.slice_mut::<#arg_type>().iter_mut();
    }
}

fn generate_immutable_storage_iter(
    table_name: &Ident,
    storage_name: &Ident,
    iter_name: &Ident,
    arg_type: &Type,
    crate_root: &Ident,
) -> TokenStream {
    quote! {
        let #storage_name = #table_name.component_storage(
            <#arg_type as #crate_root::component::Component>::component_id()
        ).read().unwrap();
        let #iter_name = #storage_name.slice::<#arg_type>().iter();
    }
}

fn get_storage_iter_code_sorted_by_arg_type(
    arg_type_refs: &[TypeReference],
    storage_iter_code: Vec<TokenStream>,
) -> Vec<TokenStream> {
    let mut type_names_and_code: Vec<_> = arg_type_refs
        .iter()
        .map(|ty| ty.elem.to_token_stream().to_string())
        .zip(storage_iter_code)
        .collect();
    type_names_and_code.sort_by_key(|(ty, _)| ty.to_string());
    type_names_and_code
        .into_iter()
        .map(|(_, code)| code)
        .collect()
}
