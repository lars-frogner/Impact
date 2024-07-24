//! Macro for performing setup on components before
//! creating entities.

use crate::querying_util::{self, TypeList};
use proc_macro2::{Ident, Span, TokenStream};
use quote::{format_ident, quote, ToTokens};
use syn::{
    braced, parenthesized,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    token::Paren,
    Expr, GenericArgument, PathArguments, Result, Token, Type, TypeReference,
};

pub(crate) struct SetupInput {
    scope: Option<SetupScope>,
    components_name: Ident,
    closure: SetupClosure,
    also_required_list: Option<TypeList>,
    disallowed_list: Option<TypeList>,
}

struct SetupScope {
    scope: TokenStream,
}

struct SetupCompClosureArg {
    var: Ident,
    ty: Type,
    interpreted_ty: InterpretedArgType,
}

enum InterpretedArgType {
    CompRef(Type),
    OptionalCompRef(Type),
}

struct SetupClosure {
    comp_args: Punctuated<SetupCompClosureArg, Token![,]>,
    return_comp_types: Option<Punctuated<Type, Token![,]>>,
    body: Expr,
}

struct ProcessedSetupInput {
    scope: Option<TokenStream>,
    components_name: Ident,
    closure_body: Expr,
    arg_names: Vec<Ident>,
    /// Types of all arguments, classified as being with or without a wrapping
    /// [`Option`].
    interpreted_arg_types: Vec<InterpretedArgType>,
    /// Types of all non-[`Option`] arguments and types wrapped by `Option`s.
    all_arg_comp_types: Vec<Type>,
    /// Types of all non-[`Option`] arguments and required types listed after
    /// the closure.
    required_comp_types: Vec<Type>,
    /// Types of all non-[`Option`] arguments, types wrapped by `Option`s and
    /// required types listed after the closure.
    requested_comp_types: Vec<Type>,
    /// Types in the return tuple.
    return_comp_types: Option<Vec<Type>>,
    /// Disallowed types listed after the closure.
    disallowed_comp_types: Option<Vec<Type>>,
    full_closure_args: Vec<TokenStream>,
}

pub(crate) fn setup(input: SetupInput, crate_root: &Ident) -> Result<TokenStream> {
    let input = input.process();

    querying_util::verify_comp_types_unique(&input.requested_comp_types)?;
    querying_util::verify_disallowed_comps_unique(
        &input.requested_comp_types,
        &input.disallowed_comp_types,
    )?;
    if let Some(return_comp_types) = &input.return_comp_types {
        querying_util::verify_comp_types_unique(return_comp_types)?;
    }

    let input_verification_code = querying_util::generate_input_verification_code(
        &input.all_arg_comp_types,
        &input.requested_comp_types,
        [&input.return_comp_types, &input.disallowed_comp_types],
        crate_root,
    )?;

    let (closure_name, closure_def_code) = generate_closure_def_code(
        &input.full_closure_args,
        &input.return_comp_types,
        &input.closure_body,
    );

    let (archetype_name, archetype_creation_code) =
        querying_util::generate_archetype_creation_code(&input.required_comp_types, crate_root);

    let if_expr_code = generate_if_expr_code(
        &input.components_name,
        &archetype_name,
        &input.disallowed_comp_types,
        crate_root,
    );

    let scope_code = input.scope.unwrap_or_else(|| quote! {});

    let (component_storage_array_name, component_storage_array_creation_code) =
        generate_component_storage_array_creation_code(
            &input.components_name,
            &input.return_comp_types,
        );

    let (component_iter_names, component_iter_code) = generate_component_iter_names_and_code(
        &input.components_name,
        &input.arg_names,
        &input.interpreted_arg_types,
    );

    let closure_call_code = generate_closure_call_code(
        &input.components_name,
        &closure_name,
        &input.arg_names,
        &component_iter_names,
        &component_storage_array_name,
        &input.return_comp_types,
    );

    let extension_code =
        generate_extension_code(&input.components_name, &component_storage_array_name);

    Ok(quote! {
        // Use local scope to avoid polluting surrounding code
        {
            // Code for verifying argument types
            #input_verification_code

            // Create archetype for all required components
            #archetype_creation_code

            // Run code if existing components match query
            if #if_expr_code
            {
                #scope_code

                // Define closure to call for each set of components
                #closure_def_code

                // Create array with empty component storages
                #component_storage_array_creation_code

                // Create iterators over requested components
                #(#component_iter_code)*

                // Call closure for each set of component instances
                // and store any returned components
                #closure_call_code

                // Add any new components to existing component set,
                // overwriting if already present
                #extension_code
            }
        }
    })
}

impl Parse for SetupInput {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let scope = querying_util::parse_scope(input)?;
        let components_name = querying_util::parse_state(input)?;
        let closure = querying_util::parse_closure(input)?;
        let (also_required_list, disallowed_list) = querying_util::parse_type_lists(input)?;
        Ok(Self {
            scope,
            components_name,
            closure,
            also_required_list,
            disallowed_list,
        })
    }
}

impl Parse for SetupScope {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let content;
        braced!(content in input);
        let scope = content.parse()?;
        Ok(Self { scope })
    }
}

impl Parse for SetupClosure {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
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

impl Parse for SetupCompClosureArg {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let var = input.parse()?;
        input.parse::<Token![:]>()?;
        let ty = input.parse()?;
        let interpreted_ty = InterpretedArgType::from(&var, &ty)?;
        Ok(Self {
            var,
            ty,
            interpreted_ty,
        })
    }
}

impl InterpretedArgType {
    fn from(name: &Ident, ty: &Type) -> Result<Self> {
        let err = || {
            Err(syn::Error::new(
                name.span(),
                format!(
                    "Invalid type for argument `{}`: expected `&C` or `Option<&C>` for a type `C`",
                    name
                ),
            ))
        };
        match ty {
            Type::Path(type_path) => {
                let last_segment = type_path.path.segments.last().unwrap();
                if last_segment.ident == Ident::new("Option", Span::call_site()) {
                    if let PathArguments::AngleBracketed(bracketed) = &last_segment.arguments {
                        if bracketed.args.len() == 1 {
                            if let GenericArgument::Type(wrapped_ty) =
                                bracketed.args.first().unwrap()
                            {
                                match wrapped_ty {
                                    Type::Reference(TypeReference {
                                        mutability, elem, ..
                                    }) if mutability.is_none() => {
                                        Ok(Self::OptionalCompRef(elem.as_ref().clone()))
                                    }
                                    _ => err(),
                                }
                            } else {
                                err()
                            }
                        } else {
                            err()
                        }
                    } else {
                        err()
                    }
                } else {
                    err()
                }
            }
            Type::Reference(TypeReference {
                mutability, elem, ..
            }) if mutability.is_none() => Ok(Self::CompRef(elem.as_ref().clone())),
            _ => err(),
        }
    }

    fn unwrap_type(&self) -> Type {
        match self {
            Self::CompRef(ty) | Self::OptionalCompRef(ty) => ty.clone(),
        }
    }

    fn get_non_optional(&self) -> Option<Type> {
        if let Self::CompRef(ty) = self {
            Some(ty.clone())
        } else {
            None
        }
    }
}

impl SetupInput {
    fn process(self) -> ProcessedSetupInput {
        let Self {
            scope,
            components_name,
            closure,
            also_required_list,
            disallowed_list,
        } = self;

        let scope = scope.map(|s| s.scope);

        let SetupClosure {
            comp_args,
            return_comp_types,
            body: closure_body,
        } = closure;

        let mut arg_names = Vec::with_capacity(comp_args.len());
        let mut arg_types = Vec::with_capacity(comp_args.len());
        let mut interpreted_arg_types = Vec::with_capacity(comp_args.len());
        comp_args.into_iter().for_each(
            |SetupCompClosureArg {
                 var,
                 ty,
                 interpreted_ty,
             }| {
                arg_names.push(var);
                arg_types.push(ty);
                interpreted_arg_types.push(interpreted_ty);
            },
        );

        // Types of all arguments that are not `Option`s
        let required_arg_comp_types: Vec<_> = interpreted_arg_types
            .iter()
            .filter_map(InterpretedArgType::get_non_optional)
            .collect();

        // Types of all arguments that are not `Option`s and the types inside
        // the `Option`s
        let all_arg_comp_types: Vec<_> = interpreted_arg_types
            .iter()
            .map(InterpretedArgType::unwrap_type)
            .collect();

        // Types in the return tuple
        let return_comp_types =
            return_comp_types.map(|return_comp_types| return_comp_types.into_iter().collect());

        // Required type list specified after the closure
        let also_required_comp_types =
            also_required_list.map(|TypeList { tys }| tys.into_iter().collect());

        // Disallowed type list specified after the closure
        let disallowed_comp_types =
            disallowed_list.map(|TypeList { tys }| tys.into_iter().collect());

        // Types of all arguments that are not `Option`s and required type list
        // specified after the closure
        let required_comp_types = querying_util::include_also_required_comp_types(
            &required_arg_comp_types,
            also_required_comp_types.clone(),
        );

        // Types of all arguments that are not `Option`s, types inside the
        // `Option`s and required type list specified after the closure
        let requested_comp_types = querying_util::include_also_required_comp_types(
            &all_arg_comp_types,
            also_required_comp_types,
        );

        let full_closure_args = create_full_closure_args(&arg_names, &arg_types);

        ProcessedSetupInput {
            scope,
            components_name,
            closure_body,
            arg_names,
            interpreted_arg_types,
            all_arg_comp_types,
            required_comp_types,
            requested_comp_types,
            return_comp_types,
            disallowed_comp_types,
            full_closure_args,
        }
    }
}

fn create_full_closure_args(arg_names: &[Ident], arg_types: &[Type]) -> Vec<TokenStream> {
    arg_names
        .iter()
        .zip(arg_types.iter())
        .map(|(name, ty)| quote! { #name: #ty })
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
    components_name: &Ident,
    archetype_name: &Ident,
    disallowed_comp_types: &Option<Vec<Type>>,
    crate_root: &Ident,
) -> TokenStream {
    let contains_all_expr = quote! {
        #components_name.archetype().contains(&#archetype_name)
    };
    match disallowed_comp_types {
        Some(disallowed_comp_types) if !disallowed_comp_types.is_empty() => {
            quote! {
                #contains_all_expr &&
                #components_name.archetype().contains_none_of(&[
                    #(<#disallowed_comp_types as #crate_root::component::Component>::component_id()),*
                ])
            }
        }
        _ => contains_all_expr,
    }
}

fn generate_component_storage_array_creation_code(
    components_name: &Ident,
    return_comp_types: &Option<Vec<Type>>,
) -> (Option<Ident>, TokenStream) {
    match return_comp_types {
        Some(return_comp_types) => {
            let array_name = Ident::new("_component_storage_array_internal__", Span::call_site());
            let array_creation_code = quote! {
                let mut #array_name = [
                    #(#components_name.new_storage_with_capacity::<#return_comp_types>()),*
                ];
            };
            (Some(array_name), array_creation_code)
        }
        None => (None, quote! {}),
    }
}

fn generate_component_iter_names_and_code(
    components_name: &Ident,
    arg_names: &[Ident],
    interpreted_arg_types: &[InterpretedArgType],
) -> (Vec<Ident>, Vec<TokenStream>) {
    let (iter_names, iter_code): (Vec<_>, Vec<_>) = arg_names
        .iter()
        .zip(interpreted_arg_types.iter())
        .map(|(name, interpreted_arg_type)| match interpreted_arg_type {
            InterpretedArgType::CompRef(ty) => {
                generate_required_component_iter_code(components_name, name, ty)
            }
            InterpretedArgType::OptionalCompRef(ty) => {
                generate_optional_component_iter_code(components_name, name, ty)
            }
        })
        .unzip();

    (iter_names, iter_code)
}

fn generate_required_component_iter_code(
    components_name: &Ident,
    arg_name: &Ident,
    comp_type: &Type,
) -> (Ident, TokenStream) {
    let iter_name = format_ident!("{}_iter_internal__", arg_name);
    let code = quote! {
        let #iter_name = #components_name.components_of_type::<#comp_type>().iter();
    };
    (iter_name, code)
}

fn generate_optional_component_iter_code(
    components_name: &Ident,
    arg_name: &Ident,
    comp_type: &Type,
) -> (Ident, TokenStream) {
    let iter_name = format_ident!("{}_iter_internal__", arg_name);
    let code = quote! {
        let #iter_name = #components_name.get_option_iter_for_component_of_type::<#comp_type>();
    };
    (iter_name, code)
}

fn generate_closure_call_code(
    components_name: &Ident,
    closure_name: &Ident,
    arg_names: &[Ident],
    component_iter_names: &[Ident],
    component_storage_array_name: &Option<Ident>,
    return_comp_types: &Option<Vec<Type>>,
) -> TokenStream {
    let (zipped_iter, nested_arg_names) = if arg_names.len() > 1 {
        (
            querying_util::generate_nested_tuple(
                &quote! { ::core::iter::zip },
                component_iter_names.iter(),
            ),
            querying_util::generate_nested_tuple(&quote! {}, arg_names.iter()),
        )
    } else if !arg_names.is_empty() {
        // For a single component type, no zipping is needed
        (
            component_iter_names[0].to_token_stream(),
            arg_names[0].to_token_stream(),
        )
    } else {
        (quote! {0..#components_name.component_count()}, quote! {_})
    };

    let closure_return_value_name = Ident::new("_closure_result_internal__", Span::call_site());
    let closure_call_code = quote! {
        let #closure_return_value_name = #closure_name(#(#arg_names),*);
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
                #[allow(non_snake_case)]
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
    components_name: &Ident,
    component_storage_array_name: &Option<Ident>,
) -> TokenStream {
    if let Some(storage_array_name) = component_storage_array_name {
        quote! {
            // We can just unwrap here because we know that all the added
            // components types will have the same number of instances
            #components_name.add_or_overwrite_component_types(#storage_array_name).unwrap();
        }
    } else {
        quote! {}
    }
}
