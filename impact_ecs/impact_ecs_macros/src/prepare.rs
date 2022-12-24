//! Macro for performing preparation on components before
//! creating entities.

use crate::querying_util::{self, TypeList};
use proc_macro2::{Ident, Span, TokenStream};
use quote::{format_ident, quote, ToTokens};
use syn::{
    parenthesized,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    token::Paren,
    Expr, Result, Token, Type,
};

pub(crate) struct PrepareInput {
    extender_name: Ident,
    closure: PrepareClosure,
    also_required_list: Option<TypeList>,
    disallowed_list: Option<TypeList>,
}

struct PrepareCompClosureArg {
    var: Ident,
    ty: Type,
}

struct PrepareClosure {
    comp_args: Punctuated<PrepareCompClosureArg, Token![,]>,
    return_comp_types: Option<Punctuated<Type, Token![,]>>,
    body: Expr,
}

struct ProcessedPrepareInput {
    extender_name: Ident,
    closure_body: Expr,
    comp_arg_names: Vec<Ident>,
    comp_arg_types: Vec<Type>,
    return_comp_types: Option<Vec<Type>>,
    disallowed_comp_types: Option<Vec<Type>>,
    required_comp_types: Vec<Type>,
    full_closure_args: Vec<TokenStream>,
}

pub(crate) fn prepare(input: PrepareInput, crate_root: &Ident) -> Result<TokenStream> {
    let input = input.process();

    querying_util::verify_comp_types_unique(&input.required_comp_types)?;
    querying_util::verify_disallowed_comps_unique(
        &input.required_comp_types,
        &input.disallowed_comp_types,
    )?;
    if let Some(return_comp_types) = &input.return_comp_types {
        querying_util::verify_comp_types_unique(return_comp_types)?;
    }

    let input_verification_code = querying_util::generate_input_verification_code(
        &input.comp_arg_types,
        &input.required_comp_types,
        [&input.return_comp_types, &input.disallowed_comp_types],
        crate_root,
    )?;

    let (closure_name, closure_def_code) = generate_closure_def_code(
        &input.full_closure_args,
        &input.return_comp_types,
        &input.closure_body,
    );

    let (archetype_name, archetype_creation_code) =
        querying_util::generate_archetype_creation_code(&input.required_comp_types, &crate_root);

    let if_expr_code = generate_if_expr_code(
        &input.extender_name,
        &archetype_name,
        &input.disallowed_comp_types,
        &crate_root,
    );

    let (component_storage_array_name, component_storage_array_creation_code) =
        generate_component_storage_array_creation_code(
            &input.extender_name,
            &input.return_comp_types,
        );

    let (initial_component_iter_names, initial_component_iter_code) =
        generate_initial_component_iter_names_and_code(
            &input.extender_name,
            &input.comp_arg_names,
            &input.comp_arg_types,
        );

    let closure_call_code = generate_closure_call_code(
        &closure_name,
        &input.comp_arg_names,
        &initial_component_iter_names,
        &component_storage_array_name,
        &input.return_comp_types,
    );

    let extension_code =
        generate_extension_code(&input.extender_name, &component_storage_array_name);

    Ok(quote! {
        // Use local scope to avoid polluting surrounding code
        {
            // Code for verifying argument types
            #input_verification_code

            // Create archetype for all required components
            #archetype_creation_code

            // Run code if components match query
            if #if_expr_code
            {
                // Define closure to call for each set of components
                #closure_def_code

                // Create array with empty component storages
                #component_storage_array_creation_code

                // Create iterators over requested initial components
                #(#initial_component_iter_code)*

                // Call closure for each set of component instances
                // and store any returned components
                #closure_call_code

                // Pass any new components to extender
                #extension_code
            }
        }
    })
}

impl Parse for PrepareInput {
    fn parse(input: ParseStream) -> Result<Self> {
        let (extender_name, closure, also_required_list, disallowed_list) =
            querying_util::parse_querying_input(input)?;
        Ok(Self {
            extender_name,
            closure,
            also_required_list,
            disallowed_list,
        })
    }
}

impl Parse for PrepareClosure {
    fn parse(input: ParseStream) -> Result<Self> {
        input.parse::<Token![|]>()?;

        let comp_args = if input.lookahead1().peek(Token![|]) {
            Punctuated::new()
        } else {
            Punctuated::parse_separated_nonempty(input)?
        };

        input.parse::<Token![|]>()?;

        let return_comp_types = if input.lookahead1().peek(Token![->]) {
            input.parse::<Token![->]>()?;
            if input.lookahead1().peek(Paren) {
                let content;
                parenthesized!(content in input);
                Some(Punctuated::parse_separated_nonempty(&content)?)
            } else {
                Some(Punctuated::parse_separated_nonempty(input)?)
            }
        } else {
            None
        };

        let body = input.parse()?;

        Ok(Self {
            comp_args,
            return_comp_types,
            body,
        })
    }
}

impl Parse for PrepareCompClosureArg {
    fn parse(input: ParseStream) -> Result<Self> {
        let var = input.parse()?;
        input.parse::<Token![:]>()?;
        input.parse::<Token![&]>()?;
        let ty = input.parse()?;

        Ok(Self { var, ty })
    }
}

impl PrepareInput {
    fn process(self) -> ProcessedPrepareInput {
        let Self {
            extender_name,
            closure,
            also_required_list,
            disallowed_list,
        } = self;

        let PrepareClosure {
            comp_args,
            return_comp_types,
            body: closure_body,
        } = closure;

        let (comp_arg_names, comp_arg_types): (Vec<_>, Vec<_>) = comp_args
            .into_iter()
            .map(|PrepareCompClosureArg { var, ty }| (var, ty))
            .unzip();

        let return_comp_types =
            return_comp_types.map(|return_comp_types| return_comp_types.into_iter().collect());

        let also_required_comp_types =
            also_required_list.map(|TypeList { tys }| tys.into_iter().collect());

        let disallowed_comp_types =
            disallowed_list.map(|TypeList { tys }| tys.into_iter().collect());

        let required_comp_types = querying_util::determine_all_required_comp_types(
            &comp_arg_types,
            also_required_comp_types,
        );

        let full_closure_args = create_full_closure_args(&comp_arg_names, &comp_arg_types);

        ProcessedPrepareInput {
            extender_name,
            closure_body,
            comp_arg_names,
            comp_arg_types,
            return_comp_types,
            disallowed_comp_types,
            required_comp_types,
            full_closure_args,
        }
    }
}

fn create_full_closure_args(comp_arg_names: &[Ident], comp_arg_types: &[Type]) -> Vec<TokenStream> {
    comp_arg_names
        .iter()
        .zip(comp_arg_types.iter())
        .map(|(name, ty)| quote! { #name: &#ty })
        .collect()
}

fn generate_closure_def_code(
    full_closure_args: &[TokenStream],
    return_comp_types: &Option<Vec<Type>>,
    closure_body: &Expr,
) -> (Ident, TokenStream) {
    let closure_name = Ident::new("_closure_internal__", Span::call_site());
    let return_type_code = match return_comp_types {
        Some(return_comp_types) => quote! { -> (#(#return_comp_types),*) },
        None => quote! {},
    };
    let closure_def_code = quote! {
        let mut #closure_name = |#(#full_closure_args),*| #return_type_code #closure_body;
    };
    (closure_name, closure_def_code)
}

fn generate_if_expr_code(
    extender_name: &Ident,
    archetype_name: &Ident,
    disallowed_comp_types: &Option<Vec<Type>>,
    crate_root: &Ident,
) -> TokenStream {
    let contains_all_expr = quote! {
        #extender_name.initial_archetype().contains(&#archetype_name)
    };
    match disallowed_comp_types {
        Some(disallowed_comp_types) if !disallowed_comp_types.is_empty() => {
            quote! {
                #contains_all_expr &&
                #extender_name.initial_archetype().contains_none_of(&[
                    #(<#disallowed_comp_types as #crate_root::component::Component>::component_id()),*
                ])
            }
        }
        _ => contains_all_expr,
    }
}

fn generate_component_storage_array_creation_code(
    extender_name: &Ident,
    return_comp_types: &Option<Vec<Type>>,
) -> (Option<Ident>, TokenStream) {
    match return_comp_types {
        Some(return_comp_types) => {
            let array_name = Ident::new("_component_storage_array_internal__", Span::call_site());
            let array_creation_code = quote! {
                let mut #array_name = [
                    #(#extender_name.new_storage::<#return_comp_types>()),*
                ];
            };
            (Some(array_name), array_creation_code)
        }
        None => (None, quote! {}),
    }
}

fn generate_initial_component_iter_names_and_code(
    extender_name: &Ident,
    comp_arg_names: &[Ident],
    comp_arg_types: &[Type],
) -> (Vec<Ident>, Vec<TokenStream>) {
    let (iter_names, iter_code): (Vec<_>, Vec<_>) = comp_arg_names
        .iter()
        .zip(comp_arg_types.iter())
        .map(|(name, ty)| generate_initial_component_iter_code(extender_name, name, ty))
        .unzip();

    (iter_names, iter_code)
}

fn generate_initial_component_iter_code(
    extender_name: &Ident,
    arg_name: &Ident,
    arg_type: &Type,
) -> (Ident, TokenStream) {
    let iter_name = format_ident!("{}_iter_internal__", arg_name);
    let code = quote! {
        let #iter_name = #extender_name.initial_components::<#arg_type>().iter();
    };
    (iter_name, code)
}

fn generate_closure_call_code(
    closure_name: &Ident,
    comp_arg_names: &[Ident],
    initial_component_iter_names: &[Ident],
    component_storage_array_name: &Option<Ident>,
    return_comp_types: &Option<Vec<Type>>,
) -> TokenStream {
    let (zipped_iter, nested_arg_names) = if comp_arg_names.len() > 1 {
        (
            querying_util::generate_nested_tuple(
                &quote! { ::core::iter::zip },
                initial_component_iter_names.iter(),
            ),
            querying_util::generate_nested_tuple(&quote! {}, comp_arg_names.iter()),
        )
    } else {
        // For a single component type, no zipping is needed
        (
            initial_component_iter_names[0].to_token_stream(),
            comp_arg_names[0].to_token_stream(),
        )
    };

    let closure_return_value_name = Ident::new("_closure_result_internal__", Span::call_site());
    let closure_call_code = quote! {
        let #closure_return_value_name = #closure_name(#(#comp_arg_names),*);
    };

    let component_storing_code = generate_component_storing_code(
        &closure_return_value_name,
        component_storage_array_name,
        return_comp_types,
    );

    quote! {
        for #nested_arg_names in #zipped_iter {
            #closure_call_code
            #component_storing_code
        }
    }
}

fn generate_component_storing_code(
    closure_return_value_name: &Ident,
    component_storage_array_name: &Option<Ident>,
    return_comp_types: &Option<Vec<Type>>,
) -> TokenStream {
    match (component_storage_array_name, return_comp_types) {
        (Some(storage_array_name), Some(return_comp_types)) => {
            let names = create_return_comp_names(return_comp_types);
            let indices = 0..names.len();
            quote! {
                let (#(#names),*) = #closure_return_value_name;
                #(
                    #storage_array_name[#indices].push(&#names);
                )*
            }
        }
        _ => quote! {},
    }
}

fn create_return_comp_names(return_comp_types: &[Type]) -> Vec<Ident> {
    return_comp_types
        .iter()
        .map(|ty| {
            format_ident!(
                "_closure_result_{}__",
                querying_util::type_to_valid_ident_string(ty)
            )
        })
        .collect()
}

fn generate_extension_code(
    extender_name: &Ident,
    component_storage_array_name: &Option<Ident>,
) -> TokenStream {
    if let Some(storage_array_name) = component_storage_array_name {
        quote! {
            #extender_name.extend(#storage_array_name);
        }
    } else {
        quote! {}
    }
}
