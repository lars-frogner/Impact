//! Macro for performing setup on components before
//! creating entities.

use crate::querying_util::{self, TypeList};
use proc_macro2::{Ident, Span, TokenStream};
use quote::{ToTokens, format_ident, quote};
use syn::{
    Error, Expr, GenericArgument, PathArguments, Result, Token, Type, TypeReference, TypeTuple,
    braced, parenthesized,
    parse::{Parse, ParseStream, discouraged::Speculative},
    punctuated::Punctuated,
    spanned::Spanned,
    token::Paren,
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
    return_type: SetupClosureReturnType,
    body: Expr,
}

#[allow(clippy::large_enum_variant)]
enum SetupClosureReturnType {
    ResultWrapped(ResultWrappedReturnCompTypes),
    Plain(ReturnCompTypes),
    None,
}

struct ResultWrappedReturnCompTypes {
    start_token: Option<Token![::]>,
    result_path: Punctuated<Ident, Token![::]>,
    begin_bracket: Token![<],
    ok_ty: ReturnCompTypes,
    err_ty: Option<ResultWrappedReturnErrType>,
    end_bracket: Token![>],
}

struct ResultWrappedReturnErrType {
    comma: Token![,],
    ty: Type,
}

struct ReturnCompTypes(Punctuated<Type, Token![,]>);

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
    /// The return type of the closure.
    return_type: ProcessedSetupClosureReturnType,
    /// Disallowed types listed after the closure.
    disallowed_comp_types: Option<Vec<Type>>,
    full_closure_args: Vec<TokenStream>,
}

struct ProcessedSetupClosureReturnType {
    comp_types: Vec<Type>,
    is_result_wrapped: bool,
    return_tokens: TokenStream,
}

pub(crate) fn setup(input: SetupInput, crate_root: &Ident) -> Result<TokenStream> {
    let input = input.process();

    querying_util::verify_comp_types_unique(&input.requested_comp_types)?;
    querying_util::verify_disallowed_comps_unique(
        &input.requested_comp_types,
        &input.disallowed_comp_types,
    )?;
    querying_util::verify_comp_types_unique(&input.return_type.comp_types)?;

    let input_verification_code = querying_util::generate_input_verification_code(
        &input.all_arg_comp_types,
        &input.requested_comp_types,
        [
            Some(&input.return_type.comp_types),
            input.disallowed_comp_types.as_ref(),
        ],
        crate_root,
    )?;

    let (closure_name, closure_def_code) = generate_closure_def_code(
        &input.full_closure_args,
        &input.return_type,
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
            &input.return_type.comp_types,
        );

    let (component_iter_names, component_iter_code) = generate_component_iter_names_and_code(
        &input.components_name,
        &input.arg_names,
        &input.interpreted_arg_types,
    );

    let (closure_error_name, closure_call_code) = generate_closure_call_code(
        &input.components_name,
        &closure_name,
        &input.arg_names,
        &component_iter_names,
        &component_storage_array_name,
        &input.return_type,
    );

    let extension_code =
        generate_extension_code(&input.components_name, &component_storage_array_name);

    let (extension_code_with_error_handling, else_branch_expr) =
        generate_closure_error_handling_code(&closure_error_name, extension_code);

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

                // If there was no error from the closure calls or they were
                // infallible, add any new components to existing component set,
                // overwriting if already present. If the calls were fallible
                // and there was an error, let the branch evaluate to it.
                #extension_code_with_error_handling
            } else {
                // If the closure calls were fallible, this branch must evaluate
                // to `Ok(())`
                #else_branch_expr
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

        let return_type = if input.lookahead1().peek(Token![->]) {
            input.parse::<Token![->]>()?;

            let fork = input.fork();

            if let Ok(return_comp_types) = fork.parse::<ResultWrappedReturnCompTypes>() {
                if return_comp_types
                    .result_path
                    .last()
                    .is_some_and(|ident| ident == "Result")
                {
                    input.advance_to(&fork);
                    SetupClosureReturnType::ResultWrapped(return_comp_types)
                } else {
                    return Err(Error::new(
                        return_comp_types.result_path.span(),
                        "Returned components wrapped in non-`Result` type",
                    ));
                }
            } else {
                SetupClosureReturnType::Plain(input.parse::<ReturnCompTypes>()?)
            }
        } else {
            SetupClosureReturnType::None
        };

        let body = input.parse()?;

        Ok(Self {
            comp_args,
            return_type,
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

impl Parse for ResultWrappedReturnCompTypes {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let start_token = input.parse::<Option<Token![::]>>()?;
        let result_path = Punctuated::parse_separated_nonempty(input)?;
        let begin_bracket: Token![<] = input.parse()?;
        let ok_ty = input.parse::<ReturnCompTypes>()?;
        let err_ty = if input.peek(Token![,]) {
            Some(input.parse::<ResultWrappedReturnErrType>()?)
        } else {
            None
        };
        let end_bracket: Token![>] = input.parse()?;
        Ok(Self {
            start_token,
            result_path,
            begin_bracket,
            ok_ty,
            err_ty,
            end_bracket,
        })
    }
}

impl Parse for ResultWrappedReturnErrType {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        Ok(Self {
            comma: input.parse()?,
            ty: input.parse()?,
        })
    }
}

impl Parse for ReturnCompTypes {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        Ok(if input.lookahead1().peek(Paren) {
            let content;
            parenthesized!(content in input);
            Self(Punctuated::parse_terminated(&content)?)
        } else {
            let ty = input.parse::<Type>()?;
            let mut types = Punctuated::new();
            types.push_value(ty);
            Self(types)
        })
    }
}

impl InterpretedArgType {
    fn from(name: &Ident, ty: &Type) -> Result<Self> {
        let err = || {
            Err(syn::Error::new(
                name.span(),
                format!(
                    "Invalid type for argument `{name}`: expected `&C` or `Option<&C>` for a type `C`"
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
            return_type,
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

        // Types in the return tuple, potentially wrapped in a `Result`
        let return_type = return_type.process();

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
            return_type,
            disallowed_comp_types,
            full_closure_args,
        }
    }
}

impl SetupClosureReturnType {
    fn process(self) -> ProcessedSetupClosureReturnType {
        match self {
            Self::Plain(ty) => {
                let types: Vec<_> = ty.0.into_iter().collect();

                let comp_types = if is_only_unit_type(&types) {
                    Vec::new()
                } else {
                    types
                };

                let return_tokens = quote! { -> (#(#comp_types),*) };

                ProcessedSetupClosureReturnType {
                    comp_types,
                    is_result_wrapped: false,
                    return_tokens,
                }
            }
            Self::ResultWrapped(ResultWrappedReturnCompTypes {
                start_token,
                result_path,
                begin_bracket,
                ok_ty,
                err_ty,
                end_bracket,
            }) => {
                let types: Vec<_> = ok_ty.0.into_iter().collect();

                let (comp_types, ok_tokens) = if is_only_unit_type(&types) {
                    (Vec::new(), quote! { () })
                } else {
                    let ok_tokens = quote! { (#(#types),*) };
                    (types, ok_tokens)
                };

                let err_tokens = match err_ty {
                    Some(ResultWrappedReturnErrType { comma, ty }) => quote! { #comma #ty },
                    None => quote! {},
                };
                let return_tokens = quote! {
                    -> #start_token #result_path #begin_bracket #ok_tokens #err_tokens #end_bracket
                };

                ProcessedSetupClosureReturnType {
                    comp_types,
                    is_result_wrapped: true,
                    return_tokens,
                }
            }
            Self::None => ProcessedSetupClosureReturnType {
                comp_types: Vec::new(),
                is_result_wrapped: false,
                return_tokens: quote! {},
            },
        }
    }
}

fn is_only_unit_type(types: &[Type]) -> bool {
    matches!(
        types.first(),
        Some(Type::Tuple(TypeTuple { elems, .. })) if types.len() == 1 && elems.is_empty(),
    )
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
    return_type: &ProcessedSetupClosureReturnType,
    closure_body: &Expr,
) -> (Ident, TokenStream) {
    let closure_name = Ident::new("_closure_internal__", Span::call_site());
    let return_type = &return_type.return_tokens;
    let closure_def_code = quote! {
        let mut #closure_name = |#(#full_closure_args),*| #return_type #closure_body;
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
    return_comp_types: &Vec<Type>,
) -> (Option<Ident>, TokenStream) {
    if return_comp_types.is_empty() {
        (None, quote! {})
    } else {
        let array_name = Ident::new("_component_storage_array_internal__", Span::call_site());
        let array_creation_code = quote! {
            let mut #array_name = [
                #(#components_name.new_storage_with_capacity::<#return_comp_types>()),*
            ];
        };
        (Some(array_name), array_creation_code)
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
    return_type: &ProcessedSetupClosureReturnType,
) -> (Option<Ident>, TokenStream) {
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

    let component_storing_code = generate_component_storing_code(
        &closure_return_value_name,
        component_storage_array_name,
        &return_type.comp_types,
    );

    if return_type.is_result_wrapped {
        let error_value_name = Ident::new("_closure_err_internal__", Span::call_site());
        let code = quote! {
            let mut #error_value_name = None;
            for #nested_arg_names in #zipped_iter {
                let #closure_return_value_name = match #closure_name(#(#arg_names),*) {
                    Ok(#closure_return_value_name) => #closure_return_value_name,
                    Err(err) => {
                        #error_value_name = Some(err);
                        break;
                    }
                };
                #component_storing_code
            }
        };
        (Some(error_value_name), code)
    } else {
        let code = quote! {
            for #nested_arg_names in #zipped_iter {
                let #closure_return_value_name = #closure_name(#(#arg_names),*);
                #component_storing_code
            }
        };
        (None, code)
    }
}

fn generate_component_storing_code(
    closure_return_value_name: &Ident,
    component_storage_array_name: &Option<Ident>,
    return_comp_types: &[Type],
) -> TokenStream {
    match component_storage_array_name {
        Some(storage_array_name) if !return_comp_types.is_empty() => {
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

fn generate_closure_error_handling_code(
    closure_error_name: &Option<Ident>,
    ok_code: TokenStream,
) -> (TokenStream, TokenStream) {
    if let Some(error_name) = closure_error_name {
        let for_then_branch = quote! {
            if let Some(err) = #error_name {
                Err(err)
            } else {
                #ok_code
                Ok(())
            }
        };
        let for_else_branch = quote! { Ok(()) };
        (for_then_branch, for_else_branch)
    } else {
        (ok_code, quote! {})
    }
}
