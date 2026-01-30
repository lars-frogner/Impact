//! Procedural macros for generating equivalents of Rust types and methods in
//! Roc.

#[cfg_attr(not(feature = "roc_codegen"), allow(dead_code))]
mod roc_attr;

use lazy_static::lazy_static;
use proc_macro::TokenStream;
use proc_macro_crate::{self, FoundCrate};
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use std::{collections::HashSet, str::FromStr};

/// Attribute macro for annotating Rust types and associated methods that
/// should be available in Roc.
///
/// When applied to a Rust type, the macro will infer and register a
/// corresponding `RegisteredType`, which is used to generate a Roc module
/// with a type declaration and some associated utility functions.
///
/// The macro can additionally be applied to the type's `impl` block and
/// selected associated constants and functions therein in order to register
/// `AssociatedConstant`s and `AssociatedFunction`s whose generated Roc code
/// will be included in the type's Roc module.
///
/// Note that the registration of types and associated items is only performed
/// when the crate hosting the target type has an active feature named
/// `roc_codegen` and the `roc_codegen` feature is active for the
/// `roc_integration` crate.
///
/// Four categories of types can be annotated with `roc`, and the requested
/// category can be specified as an argument to the macro:
/// `#[roc(category = "<category>")]`. The available categories are:
///
/// - `pod`: The type is Plain Old Data (POD) and, to prove it, implements the
///   `Pod` trait. This allows it to be passed more efficiently between Rust
///   and Roc. This is the inferred category when it is not specified and the
///   type derives `Pod`. Types of this category can only
///   contain other `roc`-annotated types with the `primitive` or `pod`
///   category, as well as arrays of such types.
///
/// - `bitflags`: The type represents a set of bitflags. It must be POD and 1,
///   2, 4 or 8 bytes in size. In Roc, it will be represented as an opaque
///   unsigned integer. The name and representing bit of each flag must be
///   declared in an argument named `flags`, like so:
///   `#[roc(category = "bitflags", flags=[FLAG_A=0, FLAG_B=1, FLAG_C=2])]`.
///   The integer specifies which bit (from the left) should be one when the
///   flag is set.
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
/// - `package = "<package>"`: The name of the Roc package this type resides
///   in. If not specified, the package is assumed to be the package code is
///   being generated in. This is typically specified for primitive types and
///   not for generated types. When specified for a generated type, that type
///   will only be generated when the specified package name matches the name
///   of the target package for generation.
/// - `parents = "<Parent>.<Modules>"`: The parent Roc module(s) for this
///   type's module, if any. Multiple module names should be separated by `.`.
/// - `module = "<Module>"`: The name of the Roc module where the type will be
///   defined. Defaults to the (Roc) name of the type.
/// - `name = "<Name>"`: The name used for the type in Roc. Defaults to the
///   Rust name.
/// - `postfix = "<function postfix>"`: Postfix for the functions operating on
///   this type (for primitive types, typically when the type's module has both
///   32- and 64-bit versions of the type).
///
/// When applied to an `impl` block, this macro accepts the following optional
/// arguments:
///
/// - `dependencies=[<type1>, <type2>, ..]`: A list of Rust types whose Roc
///   modules should be imported into the module for the present type. The
///   modules for the types comprising the present type will always be
///   imported, so this is only needed when some of the generated methods
///   make use of additional modules.
/// - `imports=[<Module1>, "<Module2.Submodule>", ..]`: A list of identifiers
///   and/or strings representing literal Roc items to import in addition to
///   the other imports.
///
/// When applied to an associated constant in a `roc`-annotated `impl` block,
/// the macro requires the Roc expression for the constant to be specified in
/// an argument like this: `expr = "<Roc code>"`. The macro also accepts the
/// following optional argument:
///
/// - `name = "<constant name>"`: The name used for the constant in Roc.
///   Defaults to the Rust name.
///
/// When applied to an associated function in a `roc`-annotated `impl` block,
/// the macro requires the Roc source code for the body of the function to be
/// specified in an argument like this: `body = "<Roc code>"`, or like this:
/// `body = ["strings", "to", "concatenate"]`. The argument names will be the
/// same in Roc as in Rust. The macro also accepts the following optional
/// arguments:
///
/// - `name = "<function name>"`: The name used for the function in Roc.
///   Defaults to the Rust name.
/// - `effectful = true/false`: Whether the function should be defined as
///   effectful, meaning a "!" will be appended to the name and a thick
///   return arrow (`=>`) will be used in the type signature.
/// - `ignore_types = true/false`: Whether to ignore the types in the Rust
///   function signature and use a "_" placeholder for the corresponding type
///   in Roc.
///
/// Not all associated functions can be translated to Roc. The following
/// requirements have to hold for the function signature:
///
/// - Each type in the function signature must be either a primitive or
///   generated Roc type (by reference or value), a string (as `&str` or
///   `String`) or an array, slice, 2- or 3-element tuple or `Result` of such
///   types (unless types are ignored).
/// - No generic parameters or `impl <Trait>`.
/// - No mutable references.
/// - There must be a return type (unless types are ignored).
#[proc_macro_attribute]
pub fn roc(attr: TokenStream, item: TokenStream) -> TokenStream {
    if let Ok(input) = syn::parse::<syn::DeriveInput>(item.clone()) {
            if attr.is_empty() {
                roc_attr::apply_type_attribute(TypeAttributeArgs::default(), input, &crate_root_tokens())
            } else {
                syn::parse::<TypeAttributeArgs>(attr).and_then(|args| {
                    roc_attr::apply_type_attribute(args, input, &crate_root_tokens())
                })
            }
        } else if let Ok(block) = syn::parse::<syn::ItemImpl>(item.clone()) {
            if attr.is_empty() {
                roc_attr::apply_impl_attribute(ImplAttributeArgs::default(), block, &crate_root_tokens())
            } else {
                syn::parse::<ImplAttributeArgs>(attr).and_then(|args| {
                    roc_attr::apply_impl_attribute(args, block, &crate_root_tokens())
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

// These need to match the corresponding constants in `roc_integration`.
const MAX_ENUM_VARIANTS: usize = 32;
const MAX_ENUM_VARIANT_FIELDS: usize = 4;
const MAX_STRUCT_FIELDS: usize = MAX_ENUM_VARIANTS * MAX_ENUM_VARIANT_FIELDS;
const MAX_BITFLAGS: usize = 64;

const MAX_FUNCTION_ARGS: usize = 16;

const MAX_DEPENDENCIES: usize = 16;
const MAX_LITERAL_IMPORTS: usize = 4;

#[derive(Clone, Debug, Default)]
struct TypeAttributeArgs {
    category: Option<TypeCategory>,
    package_name: Option<String>,
    parent_modules: Option<String>,
    module_name: Option<String>,
    type_name: Option<String>,
    function_postfix: Option<String>,
    flags: Option<Bitflags>,
}

enum TypeAttributeArg {
    String(KeyStringValueArg),
    KeyedIntList(KeyKeyedIntListValueArg),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TypeCategory {
    Primitive,
    Pod,
    Inline,
    Bitflags,
}

#[cfg_attr(not(feature = "roc_codegen"), allow(dead_code))]
#[derive(Clone, Debug, Default)]
struct Bitflags {
    flags: Vec<Bitflag>,
}

#[cfg_attr(not(feature = "roc_codegen"), allow(dead_code))]
#[derive(Clone, Debug, Default)]
struct Bitflag {
    name: String,
    bit: u32,
}

#[cfg_attr(not(feature = "roc_codegen"), allow(dead_code))]
#[derive(Clone, Default)]
struct ImplAttributeArgs {
    dependency_types: Vec<syn::Type>,
    literal_imports: Vec<String>,
}

#[cfg_attr(not(feature = "roc_codegen"), allow(dead_code))]
enum ImplAttributeArg {
    TypeList(KeyTypeListValueArg),
    StringOrIdentList(KeyStringOrIdentListValueArg),
}

#[cfg_attr(not(feature = "roc_codegen"), allow(dead_code))]
#[derive(Clone)]
struct AssociatedConstantAttributeArgs {
    expr: String,
    name: Option<String>,
}

#[cfg_attr(not(feature = "roc_codegen"), allow(dead_code))]
#[derive(Clone)]
struct AssociatedFunctionAttributeArgs {
    body: String,
    name: Option<String>,
    effectful: Option<bool>,
    ignore_types: Option<bool>,
}

#[cfg_attr(not(feature = "roc_codegen"), allow(dead_code))]
enum AssociatedFunctionAttributeArg {
    String(KeyStringValueArg),
    StringOrIdentList(KeyStringOrIdentListValueArg),
    Bool(KeyBoolValueArg),
}

struct KeyStringValueArg {
    key: syn::Ident,
    _eq_token: syn::Token![=],
    value: syn::LitStr,
}

struct KeyIntValueArg {
    key: syn::Ident,
    _eq_token: syn::Token![=],
    value: syn::LitInt,
}

struct KeyBoolValueArg {
    key: syn::Ident,
    _eq_token: syn::Token![=],
    value: syn::LitBool,
}

struct KeyTypeListValueArg {
    key: syn::Ident,
    _eq_token: syn::Token![=],
    _bracket_token: syn::token::Bracket,
    types: syn::punctuated::Punctuated<syn::Type, syn::Token![,]>,
}

struct KeyKeyedIntListValueArg {
    key: syn::Ident,
    _eq_token: syn::Token![=],
    _bracket_token: syn::token::Bracket,
    values: syn::punctuated::Punctuated<KeyIntValueArg, syn::Token![,]>,
}

struct KeyStringOrIdentListValueArg {
    key: syn::Ident,
    _eq_token: syn::Token![=],
    _bracket_token: syn::token::Bracket,
    values: syn::punctuated::Punctuated<StringOrIdent, syn::Token![,]>,
}

enum StringOrIdent {
    String(syn::LitStr),
    Ident(syn::Ident),
}

const CRATE_NAME: &str = "roc_integration";

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
        format!("crate::{CRATE_NAME}")
    }
}

fn crate_root_tokens() -> TokenStream2 {
    TokenStream2::from_str(&CRATE_IMPORT_ROOT).unwrap()
}

impl syn::parse::Parse for TypeAttributeArgs {
    fn parse(input: syn::parse::ParseStream<'_>) -> syn::Result<Self> {
        let mut category = None;
        let mut package_name = None;
        let mut parent_modules = None;
        let mut module_name = None;
        let mut type_name = None;
        let mut function_postfix = None;
        let mut flags = None;

        if input.is_empty() {
            return Ok(Self {
                category,
                package_name,
                parent_modules,
                module_name,
                type_name,
                function_postfix,
                flags,
            });
        }

        let args =
            syn::punctuated::Punctuated::<TypeAttributeArg, syn::token::Comma>::parse_terminated(
                input,
            )?;

        for arg in args {
            match arg {
                TypeAttributeArg::String(arg) => match arg.key.to_string().as_str() {
                    "category" => {
                        let value = arg.value.value();

                        let cat = if value == "primitive" {
                            TypeCategory::Primitive
                        } else if value == "pod" {
                            TypeCategory::Pod
                        } else if value == "inline" {
                            TypeCategory::Inline
                        } else if value == "bitflags" {
                            TypeCategory::Bitflags
                        } else {
                            return Err(syn::Error::new_spanned(
                                arg.value,
                                format!(
                                    "invalid category `{value}`, must be one of `pod`, `inline`, `primitive`, `bitflags`"
                                ),
                            ));
                        };

                        if category.replace(cat).is_some() {
                            return Err(syn::Error::new_spanned(
                                arg.key,
                                "repeated argument `category`",
                            ));
                        }
                    }
                    "package" => {
                        if package_name.replace(arg.value.value()).is_some() {
                            return Err(syn::Error::new_spanned(
                                arg.key,
                                "repeated argument `package`",
                            ));
                        }
                    }
                    "parents" => {
                        if parent_modules.replace(arg.value.value()).is_some() {
                            return Err(syn::Error::new_spanned(
                                arg.key,
                                "repeated argument `parents`",
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
                                "invalid argument `{other}`, must be one of \
                                 `category`, `package`, `parents`, `module`, `name`, `postfix`, `flags`"
                            ),
                        ));
                    }
                },
                TypeAttributeArg::KeyedIntList(args) => match args.key.to_string().as_str() {
                    "flags" => {
                        if flags.replace(Bitflags::from_args(&args)?).is_some() {
                            return Err(syn::Error::new_spanned(
                                args.key,
                                "repeated argument `flags`",
                            ));
                        }
                    }
                    other => {
                        return Err(syn::Error::new_spanned(
                            args.key,
                            format!(
                                "invalid argument `{other}`, must be one of \
                                 `category`, `package`, `parents`, `module`, `name`, `postfix`, `flags`"
                            ),
                        ));
                    }
                },
            }
        }

        Ok(TypeAttributeArgs {
            category,
            package_name,
            parent_modules,
            module_name,
            type_name,
            function_postfix,
            flags,
        })
    }
}

impl syn::parse::Parse for TypeAttributeArg {
    fn parse(input: syn::parse::ParseStream<'_>) -> syn::Result<Self> {
        let ahead = input.fork();
        let _key: syn::Ident = ahead.parse()?;
        let _eq: syn::Token![=] = ahead.parse()?;
        if ahead.peek(syn::token::Bracket) {
            Ok(Self::KeyedIntList(input.parse()?))
        } else {
            Ok(Self::String(input.parse()?))
        }
    }
}

impl Bitflags {
    fn from_args(args: &KeyKeyedIntListValueArg) -> syn::Result<Self> {
        let mut names = HashSet::with_capacity(args.values.len());
        let mut bits = 0_u64;

        let flags = args
            .values
            .iter()
            .map(|KeyIntValueArg { key, value, .. }| {
                let name = key.to_string();
                if !names.insert(name.clone()) {
                    return Err(syn::Error::new(
                        key.span(),
                        format!("duplicate bitflag name `{name}`"),
                    ));
                }

                let bit: u32 = value.base10_parse()?;
                if bit >= 64 {
                    return Err(syn::Error::new(value.span(), "bit must be smaller than 64"));
                }

                let next_bits = bits | 1 << bit;
                if next_bits == bits {
                    return Err(syn::Error::new(
                        value.span(),
                        format!("duplicate bit {bit}"),
                    ));
                }
                bits = next_bits;

                Ok(Bitflag { name, bit })
            })
            .collect::<syn::Result<Vec<_>>>()?;

        if flags.len() > MAX_BITFLAGS {
            return Err(syn::Error::new(
                args.key.span(),
                format!("too many bitflags ({}/{})", flags.len(), MAX_BITFLAGS),
            ));
        }

        Ok(Self { flags })
    }
}

impl syn::parse::Parse for ImplAttributeArgs {
    fn parse(input: syn::parse::ParseStream<'_>) -> syn::Result<Self> {
        let args =
            syn::punctuated::Punctuated::<ImplAttributeArg, syn::token::Comma>::parse_terminated(
                input,
            )?;

        let mut dependency_types = None;
        let mut imports = None;

        for arg in args {
            match arg {
                ImplAttributeArg::TypeList(arg) => match arg.key.to_string().as_str() {
                    "dependencies" => {
                        if dependency_types
                            .replace(arg.types.iter().cloned().collect::<Vec<_>>())
                            .is_some()
                        {
                            return Err(syn::Error::new_spanned(
                                arg.key,
                                "repeated argument `dependencies`",
                            ));
                        }
                    }
                    other => {
                        return Err(syn::Error::new_spanned(
                            arg.key,
                            format!(
                                "invalid argument `{other}`, must be one of `dependencies`, `imports`"
                            ),
                        ));
                    }
                },
                ImplAttributeArg::StringOrIdentList(arg) => match arg.key.to_string().as_str() {
                    "imports" => {
                        let values: Vec<String> = arg
                            .values
                            .iter()
                            .map(|v| match v {
                                StringOrIdent::String(s) => s.value(),
                                StringOrIdent::Ident(i) => i.to_string(),
                            })
                            .collect();
                        if imports.replace(values).is_some() {
                            return Err(syn::Error::new_spanned(
                                arg.key,
                                "repeated argument `imports`",
                            ));
                        }
                    }
                    other => {
                        return Err(syn::Error::new_spanned(
                            arg.key,
                            format!(
                                "invalid argument `{other}`, must be one of `dependencies`, `imports`"
                            ),
                        ));
                    }
                },
            }
        }

        let dependency_types = dependency_types.unwrap_or_default();

        if dependency_types.len() > MAX_DEPENDENCIES {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                format!(
                    "the `roc` attribute does not support this many dependencies ({}/{})",
                    dependency_types.len(),
                    MAX_DEPENDENCIES
                ),
            ));
        }

        let literal_imports = imports.unwrap_or_default();

        if literal_imports.len() > MAX_LITERAL_IMPORTS {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                format!(
                    "the `roc` attribute does not support this many imports ({}/{})",
                    literal_imports.len(),
                    MAX_LITERAL_IMPORTS
                ),
            ));
        }

        Ok(Self {
            dependency_types,
            literal_imports,
        })
    }
}

impl syn::parse::Parse for ImplAttributeArg {
    fn parse(input: syn::parse::ParseStream<'_>) -> syn::Result<Self> {
        let ahead = input.fork();
        let key: syn::Ident = ahead.parse()?;
        let _eq: syn::Token![=] = ahead.parse()?;
        match key.to_string().as_str() {
            "dependencies" => Ok(Self::TypeList(input.parse()?)),
            "imports" => Ok(Self::StringOrIdentList(input.parse()?)),
            other => Err(syn::Error::new_spanned(
                key,
                format!("invalid argument `{other}`, must be one of `dependencies`, `imports`"),
            )),
        }
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
                        format!("invalid argument `{other}`, must be one of `expr`, `name`"),
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
            syn::punctuated::Punctuated::<AssociatedFunctionAttributeArg, syn::token::Comma>::parse_terminated(
                input,
            )?;

        let mut body = None;
        let mut name = None;
        let mut effectful = None;
        let mut ignore_types = None;

        for arg in &args {
            match arg {
                AssociatedFunctionAttributeArg::String(arg) => match arg.key.to_string().as_str() {
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
                                "invalid argument `{other}`, must be one of `body`, `name`, `effectful`, `ignore_types`"
                            ),
                        ));
                    }
                },
                AssociatedFunctionAttributeArg::StringOrIdentList(arg) => {
                    match arg.key.to_string().as_str() {
                        "body" => {
                            // Concatenate string and identifier list to single string
                            let mut body_string = String::new();
                            for v in &arg.values {
                                match v {
                                    StringOrIdent::String(v) => {
                                        body_string.push_str(&v.value());
                                    }
                                    StringOrIdent::Ident(v) => {
                                        body_string.push_str(&v.to_string());
                                    }
                                }
                            }
                            if body.replace(body_string).is_some() {
                                return Err(syn::Error::new_spanned(
                                    arg.key.clone(),
                                    "repeated argument `body`",
                                ));
                            }
                        }
                        other => {
                            return Err(syn::Error::new_spanned(
                                arg.key.clone(),
                                format!(
                                    "invalid argument `{other}`, must be one of `body`, `name`, `effectful`, `ignore_types`"
                                ),
                            ));
                        }
                    }
                }
                AssociatedFunctionAttributeArg::Bool(arg) => match arg.key.to_string().as_str() {
                    "effectful" => {
                        if effectful.replace(arg.value.value()).is_some() {
                            return Err(syn::Error::new_spanned(
                                arg.key.clone(),
                                "repeated argument `effectful`",
                            ));
                        }
                    }
                    "ignore_types" => {
                        if ignore_types.replace(arg.value.value()).is_some() {
                            return Err(syn::Error::new_spanned(
                                arg.key.clone(),
                                "repeated argument `ignore_types`",
                            ));
                        }
                    }
                    other => {
                        return Err(syn::Error::new_spanned(
                            arg.key.clone(),
                            format!(
                                "invalid argument `{other}`, must be one of `body`, `name`, `effectful`, `ignore_types`"
                            ),
                        ));
                    }
                },
            }
        }

        let Some(body) = body else {
            let span = args
                .first()
                .map_or_else(proc_macro2::Span::call_site, |arg| match arg {
                    AssociatedFunctionAttributeArg::String(arg) => arg.key.span(),
                    AssociatedFunctionAttributeArg::StringOrIdentList(arg) => arg.key.span(),
                    AssociatedFunctionAttributeArg::Bool(arg) => arg.key.span(),
                });
            return Err(syn::Error::new(span, "missing required argument `body`"));
        };

        Ok(Self {
            body,
            name,
            effectful,
            ignore_types,
        })
    }
}

impl syn::parse::Parse for AssociatedFunctionAttributeArg {
    fn parse(input: syn::parse::ParseStream<'_>) -> syn::Result<Self> {
        let ahead = input.fork();
        let _key: syn::Ident = ahead.parse()?;
        let _eq: syn::Token![=] = ahead.parse()?;
        if ahead.peek(syn::token::Bracket) {
            Ok(Self::StringOrIdentList(input.parse()?))
        } else if ahead.parse::<syn::LitBool>().is_ok() {
            Ok(Self::Bool(input.parse()?))
        } else {
            Ok(Self::String(input.parse()?))
        }
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

impl syn::parse::Parse for KeyIntValueArg {
    fn parse(input: syn::parse::ParseStream<'_>) -> syn::Result<Self> {
        Ok(Self {
            key: input.parse()?,
            _eq_token: input.parse()?,
            value: input.parse()?,
        })
    }
}

impl syn::parse::Parse for KeyBoolValueArg {
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

impl syn::parse::Parse for KeyKeyedIntListValueArg {
    fn parse(input: syn::parse::ParseStream<'_>) -> syn::Result<Self> {
        let content;
        Ok(Self {
            key: input.parse()?,
            _eq_token: input.parse()?,
            _bracket_token: syn::bracketed!(content in input),
            values: content.parse_terminated(KeyIntValueArg::parse, syn::Token![,])?,
        })
    }
}

impl syn::parse::Parse for KeyStringOrIdentListValueArg {
    fn parse(input: syn::parse::ParseStream<'_>) -> syn::Result<Self> {
        let content;
        Ok(Self {
            key: input.parse()?,
            _eq_token: input.parse()?,
            _bracket_token: syn::bracketed!(content in input),
            values: content.parse_terminated(StringOrIdent::parse, syn::Token![,])?,
        })
    }
}

impl syn::parse::Parse for StringOrIdent {
    fn parse(input: syn::parse::ParseStream<'_>) -> syn::Result<Self> {
        if input.peek(syn::LitStr) {
            Ok(Self::String(input.parse()?))
        } else {
            Ok(Self::Ident(input.parse()?))
        }
    }
}
