//! Procedural macros for generating equivalents of Rust types and methods in
//! Roc.

#[cfg(feature = "enabled")]
mod roc_attr;

/// Attribute macro for annotating Rust types and associated methods that
/// should be available in Roc.
///
/// When applied to a Rust type, the macro will infer and register a
/// corresponding [`RegisteredType`](roc_codegen::meta::RegisteredType), which is used to
/// [`generate`](roc_codegen::generate) a Roc module with a type declaration
/// and some associated utility functions.
///
/// The macro can additionally be applied to the type's `impl` block and
/// selected methods therein in order to register
/// [`AssociatedFunction`](roc_codegen::meta::AssociatedFunction)s whose generated Roc code will
/// be included in the type's Roc module.
///
/// Note that the registration of types and methods is only performed when the
/// crate hosting the target type has an active feature named `roc_codegen` and
/// the `enabled` feature is active for the [`roc_codegen`] crate.
///
/// Three categories of types can be annotated with `roc`, and the requested
/// category can be specified as an argument to the macro:
/// `#[roc(<category>)]`. The available categories are:
///
/// - `pod`: The type is Plain Old Data (POD) and, to prove it, implements the
///   [`bytemuck::Pod`] trait. This allows it to be passed more efficiently
///   between Rust and Roc. This is the inferred category when it is not
///   specified and the type derives `Pod`. Types of this category can only
///   contain other `roc`-annotated types with the `primitive` or `pod`
///   category, as well as arrays of such types.
///
/// - `inline`: This category is more flexible than `pod`, as it also supports
///   enums and types with padding. However, the type is not allowed to contain
///   pointers or references to heap-allocated memory; all the data must be
///   "inline". This is the inferred category when it is not specified and the
///   type does not derive `Pod`. Types of this category can only contain other
///   `roc`-annotated types (but of any category), as well as arrays of such
///   types.
///
/// - `primitive`: These are the building blocks of `pod` and `inline` types.
///   No Roc code will be generated for any `primitive` type. Instead, it is
///   assumed that a Roc implementation already exists. This category is never
///   inferred when it is not specified explicitly. Types of this category can
///   contain types that are not `roc`-annotated, but it is a requirement that
///   `primitive` types are POD.
///
/// When applied to a type, the `roc` macro accepts the following optional
/// arguments:
///
/// - `name = "<name>"`: The name used for the type in Roc. Defaults to the
///   Rust name.
/// - `module = "<module>"`: The name used for the module holding the type's
///   Roc code. Defaults to the (Roc) name of the type.
/// - `package = "<package>"`: The name of the Roc package the module should be
///   imported from when used. This is currently only relevant when using this
///   macro to declare primitive types, as all generated (i.e. non-primitive)
///   types are put in the same package.
///
/// When applied to an `impl` block, this macro accepts the following optional
/// argument:
///
/// - `dependencies=[<type1>, <type2>, ..]`: A list of Rust types whose Roc
///   modules should be imported into the module for the present type. The
///   modules for the types comprising the present type will always be
///   imported, so this is only needed when some of the generated methods
///   make use of additional modules.
///
/// When applied to a method in a `roc`-annotated `impl` block, the macro
/// requires the Roc source code for the body of the method to be specified
/// in an argument like this: `body = "<Roc code>"`. The argument names will
/// be the same in Roc as in Rust. The macro also accepts the following
/// optional argument:
///
/// - `name = "<method name>"`: The name used for the method in Roc. Defaults
///   to the Rust name.
///
/// Not all methods can be translated to Roc. The following requirements have
/// to hold for the method signature:
///
/// - Each type in the method signature must be either a primitive or generated
///   Roc type (by reference or value), a string (as `&str` or `String`) or an
///   array or slice of such types.
/// - No generic parameters or `impl <Trait>`.
/// - No mutable references.
/// - There must be a return type.
#[proc_macro_attribute]
pub fn roc(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    inner::roc(attr, item)
}

#[cfg(feature = "enabled")]
mod inner {
    use super::roc_attr;
    use lazy_static::lazy_static;
    use proc_macro::TokenStream;
    use proc_macro_crate::{self, FoundCrate};
    use proc_macro2::TokenStream as TokenStream2;
    use quote::quote;
    use std::str::FromStr;

    pub fn roc(attr: TokenStream, item: TokenStream) -> TokenStream {
        if let Ok(input) = syn::parse::<syn::DeriveInput>(item.clone()) {
            if attr.is_empty() {
                roc_attr::apply_type_attribute(None, input, &crate_root_tokens())
            } else {
                syn::parse::<TypeAttributeArgs>(attr).and_then(|args| {
                    roc_attr::apply_type_attribute(Some(args), input, &crate_root_tokens())
                })
            }
        } else if let Ok(block) = syn::parse::<syn::ItemImpl>(item.clone()) {
            if attr.is_empty() {
                roc_attr::apply_impl_attribute(None, block, &crate_root_tokens())
            } else {
                syn::parse::<ImplAttributeArgs>(attr).and_then(|args| {
                    roc_attr::apply_impl_attribute(Some(args), block, &crate_root_tokens())
                })
            }
        } else if let Ok(constant) = syn::parse::<syn::ImplItemConst>(item.clone()) {
            syn::parse::<AssociatedConstantAttributeArgs>(attr).and_then(|args| roc_attr::apply_associated_constant_attribute(args, constant))
        } else if let Ok(func) = syn::parse::<syn::ImplItemFn>(item.clone()) {
            syn::parse::<AssociatedFunctionAttributeArgs>(attr).and_then(|args| roc_attr::apply_associated_function_attribute(args, func))
        } else {
            Err(syn::Error::new_spanned(
                TokenStream2::from(item.clone()),
                "the `roc` attribute can only be applied to type definitions, impl blocks and associated constants and functions",
            ))
        }
        .unwrap_or_else(|err| {
            let item = TokenStream2::from(item);
            let error = err.to_compile_error();
            quote! {
                #item
                #error
            }
        })
        .into()
    }

    // These need to match the corresponding constants in `roc_codegen`.
    pub const MAX_ENUM_VARIANTS: usize = 8;
    pub const MAX_ENUM_VARIANT_FIELDS: usize = 2;
    pub const MAX_STRUCT_FIELDS: usize = MAX_ENUM_VARIANTS * MAX_ENUM_VARIANT_FIELDS;

    pub const MAX_FUNCTION_ARGS: usize = 16;

    pub const MAX_DEPENDENCIES: usize = 16;

    #[derive(Clone, Debug)]
    pub struct TypeAttributeArgs {
        pub category: TypeCategory,
        pub package_name: Option<String>,
        pub module_name: Option<String>,
        pub type_name: Option<String>,
        pub function_postfix: Option<String>,
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum TypeCategory {
        Primitive,
        Pod,
        Inline,
    }

    #[derive(Clone)]
    pub struct ImplAttributeArgs {
        pub dependency_types: Vec<syn::Type>,
    }

    #[derive(Clone)]
    pub struct AssociatedConstantAttributeArgs {
        pub expr: String,
        pub name: Option<String>,
    }

    #[derive(Clone)]
    pub struct AssociatedFunctionAttributeArgs {
        pub body: String,
        pub name: Option<String>,
    }

    struct KeyStringValueArg {
        key: syn::Ident,
        _eq_token: syn::Token![=],
        value: syn::LitStr,
    }

    struct KeyTypeListValueArg {
        key: syn::Ident,
        _eq_token: syn::Token![=],
        _bracket_token: syn::token::Bracket,
        types: syn::punctuated::Punctuated<syn::Type, syn::Token![,]>,
    }

    const CRATE_NAME: &str = "roc_codegen";

    lazy_static! {
        static ref CRATE_IMPORT_ROOT: String = determine_crate_import_root();
    }

    /// Determines whether to use `crate`, the actual crate name or a re-export of
    /// the crate as root for `use` statements.
    fn determine_crate_import_root() -> String {
        if let Ok(found_crate) = proc_macro_crate::crate_name(CRATE_NAME) {
            let crate_root = match found_crate {
                FoundCrate::Itself => "crate".to_string(),
                FoundCrate::Name(name) => name,
            };
            crate_root
        } else {
            format!("crate::{}", CRATE_NAME)
        }
    }

    fn crate_root_tokens() -> TokenStream2 {
        TokenStream2::from_str(&CRATE_IMPORT_ROOT).unwrap()
    }

    impl syn::parse::Parse for TypeAttributeArgs {
        fn parse(input: syn::parse::ParseStream<'_>) -> syn::Result<Self> {
            let category: syn::Ident = input.parse()?;

            let category = if category == "primitive" {
                TypeCategory::Primitive
            } else if category == "pod" {
                TypeCategory::Pod
            } else if category == "inline" {
                TypeCategory::Inline
            } else {
                return Err(syn::Error::new_spanned(
                    category.clone(),
                    format!(
                        "invalid category `{}`, must be one of `pod`, `inline`, `primitive`",
                        category
                    ),
                ));
            };

            let mut package_name = None;
            let mut module_name = None;
            let mut type_name = None;
            let mut function_postfix = None;

            if input.is_empty() {
                return Ok(Self {
                    category,
                    package_name,
                    module_name,
                    type_name,
                    function_postfix,
                });
            }

            input.parse::<syn::token::Comma>()?;

            let args =
            syn::punctuated::Punctuated::<KeyStringValueArg, syn::token::Comma>::parse_terminated(
                input,
            )?;

            for arg in args {
                match arg.key.to_string().as_str() {
                    "package" => {
                        if package_name.replace(arg.value.value()).is_some() {
                            return Err(syn::Error::new_spanned(
                                arg.key,
                                "repeated argument `package`",
                            ));
                        }
                    }
                    "module" => {
                        if module_name.replace(arg.value.value()).is_some() {
                            return Err(syn::Error::new_spanned(
                                arg.key,
                                "repeated argument `module`",
                            ));
                        }
                    }
                    "name" => {
                        if type_name.replace(arg.value.value()).is_some() {
                            return Err(syn::Error::new_spanned(
                                arg.key,
                                "repeated argument `name`",
                            ));
                        }
                    }
                    "postfix" => {
                        if function_postfix.replace(arg.value.value()).is_some() {
                            return Err(syn::Error::new_spanned(
                                arg.key,
                                "repeated argument `postfix`",
                            ));
                        }
                    }
                    other => {
                        return Err(syn::Error::new_spanned(
                            arg.key,
                            format!(
                                "invalid argument `{}`, must be one of `package`, `module`, `name`, `postfix`",
                                other
                            ),
                        ));
                    }
                }
            }

            Ok(Self {
                category,
                package_name,
                module_name,
                type_name,
                function_postfix,
            })
        }
    }

    impl syn::parse::Parse for ImplAttributeArgs {
        fn parse(input: syn::parse::ParseStream<'_>) -> syn::Result<Self> {
            let arg: KeyTypeListValueArg = input.parse()?;

            let dependency_types: Vec<_> = match arg.key.to_string().as_str() {
                "dependencies" => arg.types.iter().cloned().collect(),
                other => {
                    return Err(syn::Error::new_spanned(
                        arg.key,
                        format!("invalid argument `{}`, must be `dependencies`", other),
                    ));
                }
            };

            if dependency_types.len() > MAX_DEPENDENCIES {
                return Err(syn::Error::new_spanned(
                    arg.types,
                    format!(
                        "the `roc` attribute does not support this many dependencies ({}/{})",
                        dependency_types.len(),
                        MAX_DEPENDENCIES
                    ),
                ));
            }

            Ok(Self { dependency_types })
        }
    }

    impl syn::parse::Parse for AssociatedConstantAttributeArgs {
        fn parse(input: syn::parse::ParseStream<'_>) -> syn::Result<Self> {
            let args =
                        syn::punctuated::Punctuated::<KeyStringValueArg, syn::token::Comma>::parse_terminated(
                            input,
                        )?;

            let mut expr = None;
            let mut name = None;

            for arg in &args {
                match arg.key.to_string().as_str() {
                    "expr" => {
                        if expr.replace(arg.value.value()).is_some() {
                            return Err(syn::Error::new_spanned(
                                arg.key.clone(),
                                "repeated argument `expr`",
                            ));
                        }
                    }
                    "name" => {
                        if name.replace(arg.value.value()).is_some() {
                            return Err(syn::Error::new_spanned(
                                arg.key.clone(),
                                "repeated argument `name`",
                            ));
                        }
                    }
                    other => {
                        return Err(syn::Error::new_spanned(
                            arg.key.clone(),
                            format!(
                                "invalid argument `{}`, must be one of `expr`, `name`",
                                other
                            ),
                        ));
                    }
                }
            }

            let Some(expr) = expr else {
                let span = args
                    .first()
                    .map_or_else(proc_macro2::Span::call_site, |arg| arg.key.span());
                return Err(syn::Error::new(span, "missing required argument `expr`"));
            };

            Ok(Self { expr, name })
        }
    }

    impl syn::parse::Parse for AssociatedFunctionAttributeArgs {
        fn parse(input: syn::parse::ParseStream<'_>) -> syn::Result<Self> {
            let args =
                syn::punctuated::Punctuated::<KeyStringValueArg, syn::token::Comma>::parse_terminated(
                    input,
                )?;

            let mut body = None;
            let mut name = None;

            for arg in &args {
                match arg.key.to_string().as_str() {
                    "body" => {
                        if body.replace(arg.value.value()).is_some() {
                            return Err(syn::Error::new_spanned(
                                arg.key.clone(),
                                "repeated argument `body`",
                            ));
                        }
                    }
                    "name" => {
                        if name.replace(arg.value.value()).is_some() {
                            return Err(syn::Error::new_spanned(
                                arg.key.clone(),
                                "repeated argument `name`",
                            ));
                        }
                    }
                    other => {
                        return Err(syn::Error::new_spanned(
                            arg.key.clone(),
                            format!(
                                "invalid argument `{}`, must be one of `body`, `name`",
                                other
                            ),
                        ));
                    }
                }
            }

            let Some(body) = body else {
                let span = args
                    .first()
                    .map_or_else(proc_macro2::Span::call_site, |arg| arg.key.span());
                return Err(syn::Error::new(span, "missing required argument `body`"));
            };

            Ok(Self { body, name })
        }
    }

    impl syn::parse::Parse for KeyStringValueArg {
        fn parse(input: syn::parse::ParseStream<'_>) -> syn::Result<Self> {
            Ok(Self {
                key: input.parse()?,
                _eq_token: input.parse()?,
                value: input.parse()?,
            })
        }
    }

    impl syn::parse::Parse for KeyTypeListValueArg {
        fn parse(input: syn::parse::ParseStream<'_>) -> syn::Result<Self> {
            let content;
            Ok(Self {
                key: input.parse()?,
                _eq_token: input.parse()?,
                _bracket_token: syn::bracketed!(content in input),
                types: content.parse_terminated(syn::Type::parse, syn::Token![,])?,
            })
        }
    }
}

#[cfg(not(feature = "enabled"))]
mod inner {
    pub fn roc(
        _attr: proc_macro::TokenStream,
        item: proc_macro::TokenStream,
    ) -> proc_macro::TokenStream {
        item
    }
}
