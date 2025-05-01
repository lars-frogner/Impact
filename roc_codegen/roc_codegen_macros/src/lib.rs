//! Procedural macros for generating equivalents of Rust types and methods in
//! Roc.

#[cfg(feature = "enabled")]
mod roc_attr;

/// Attribute macro for annotating Rust types that should be available in Roc.
/// The macro will infer and register a
/// [`RocType`](roc_codegen::meta::RocType) for the target
/// type, which is used to [`generate`](roc_codegen::generate) the associated
/// Roc code.
///
/// Note that this registration is only performed when the crate hosting the
/// target type has an active feature named `roc_codegen` and the `enabled`
/// feature is active for the [`roc_codegen`] crate.
///
/// Three categories of types can be annotated with `roc`, and the requested
/// category can be specified as an argument to the macro:
/// `#[roc(<category>)]`. The available categories are:
/// - `pod`: The type is Plain Old Data (POD) and, to prove it, implements the
///   [`bytemuck::Pod`] trait. This allows it to be passed more efficiently
///   between Rust and Roc. This is the inferred category when it is not
///   specified and the type derives `Pod`. Types of this category can only
///   contain other `roc`-annotated types with the `primitive` or `pod`
///   category, as well as arrays of such types.
/// - `inline`: This category is more flexible than `pod`, as it also supports
///   enums and types with padding. However, the type is not allowed to contain
///   pointers or references to heap-allocated memory; all the data must be
///   "inline". This is the inferred category when it is not specified and the
///   type does not derive `Pod`. Types of this category can only contain other
///   `roc`-annotated types (but of any category), as well as arrays of such
///   types.
/// - `primitive`: These are the building blocks of `pod` and `inline` types.
///   No Roc code will be generated for any `primitive` type. Instead, it is
///   assumed that a Roc implementation already exists. This category is never
///   inferred when it is not specified explicitly. Types of this category can
///   contain types that are not `roc`-annotated, but it is a requirement that
///   `primitive` types are POD.
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
                roc_attr::apply_roc_type_attribute(None, input, &crate_root_tokens())
            } else {
                syn::parse::<RocTypeAttributeArgs>(attr).and_then(|args| {
                    roc_attr::apply_roc_type_attribute(Some(args), input, &crate_root_tokens())
                })
            }
        } else if let Ok(block) = syn::parse::<syn::ItemImpl>(item.clone()) {
            if attr.is_empty() {
                roc_attr::apply_roc_impl_attribute(None, block, &crate_root_tokens())
            } else {
                syn::parse::<RocImplAttributeArgs>(attr).and_then(|args| {
                    roc_attr::apply_roc_impl_attribute(Some(args), block, &crate_root_tokens())
                })
            }
        } else if let Ok(func) = syn::parse::<syn::ImplItemFn>(item.clone()) {
            syn::parse::<RocMethodAttributeArgs>(attr).and_then(|args| roc_attr::apply_roc_method_attribute(args, func))
        } else {
            Err(syn::Error::new_spanned(
                TokenStream2::from(item.clone()),
                "the `roc` attribute can only be applied to type definitions, impl blocks and methods",
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

    // These need to match the corresponding constants in `roc_codegen::meta`.
    pub const MAX_ROC_TYPE_ENUM_VARIANTS: usize = 8;
    pub const MAX_ROC_TYPE_ENUM_VARIANT_FIELDS: usize = 2;
    pub const MAX_ROC_TYPE_STRUCT_FIELDS: usize =
        MAX_ROC_TYPE_ENUM_VARIANTS * MAX_ROC_TYPE_ENUM_VARIANT_FIELDS;

    pub const MAX_ROC_FUNCTION_ARGS: usize = 16;

    pub const MAX_ROC_DEPENDENCIES: usize = 16;

    #[derive(Clone, Debug)]
    pub struct RocTypeAttributeArgs {
        pub category: RocTypeCategory,
        pub package_name: Option<String>,
        pub module_name: Option<String>,
        pub type_name: Option<String>,
        pub function_postfix: Option<String>,
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum RocTypeCategory {
        Primitive,
        Pod,
        Inline,
    }

    #[derive(Clone)]
    pub struct RocImplAttributeArgs {
        pub dependency_types: Vec<syn::Type>,
    }

    #[derive(Clone)]
    pub struct RocMethodAttributeArgs {
        pub body: String,
        pub method_name: Option<String>,
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

    impl syn::parse::Parse for RocTypeAttributeArgs {
        fn parse(input: syn::parse::ParseStream<'_>) -> syn::Result<Self> {
            let category: syn::Ident = input.parse()?;

            let category = if category == "primitive" {
                RocTypeCategory::Primitive
            } else if category == "pod" {
                RocTypeCategory::Pod
            } else if category == "inline" {
                RocTypeCategory::Inline
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

    impl syn::parse::Parse for RocImplAttributeArgs {
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

            if dependency_types.len() > MAX_ROC_DEPENDENCIES {
                return Err(syn::Error::new_spanned(
                    arg.types,
                    format!(
                        "the `roc` attribute does not support this many dependencies ({}/{})",
                        dependency_types.len(),
                        MAX_ROC_DEPENDENCIES
                    ),
                ));
            }

            Ok(Self { dependency_types })
        }
    }

    impl syn::parse::Parse for RocMethodAttributeArgs {
        fn parse(input: syn::parse::ParseStream<'_>) -> syn::Result<Self> {
            let args =
                syn::punctuated::Punctuated::<KeyStringValueArg, syn::token::Comma>::parse_terminated(
                    input,
                )?;

            let mut body = None;
            let mut method_name = None;

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
                        if method_name.replace(arg.value.value()).is_some() {
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

            Ok(Self { body, method_name })
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
