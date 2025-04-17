//! Procedural macros for generating equivalents of Rust types in Roc.

mod roc_attr;

use lazy_static::lazy_static;
use proc_macro::TokenStream;
use proc_macro_crate::{self, FoundCrate};
use proc_macro2::TokenStream as TokenStream2;
use std::str::FromStr;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RocAttributeArg {
    None,
    Auto,
    Primitive,
    Pod,
    Inline,
}

/// Attribute macro for annotating Rust types that should be available in Roc.
/// The macro will infer and register a
/// [`RocTypeDescriptor`](roc_codegen::meta::RocTypeDescriptor) for the target
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
///   category.
/// - `inline`: This category is more flexible than `pod`, as it also supports
///   enums and types with padding. However, the type is not allowed to contain
///   pointers or references to heap-allocated memory; all the data must be
///   "inline". This is the inferred category when it is not specified and the
///   type does not derive `Pod`. Types of this category can only contain other
///   `roc`-annotated types (but of any category).
/// - `primitive`: These are the building blocks of `pod` and `inline` types.
///   No Roc code will be generated for any `primitive` type. Instead, it is
///   assumed that a Roc implementation already exists. This category is never
///   inferred when it is not specified explicitly. Types of this category can
///   contain types that are not `roc`-annotated, but it is a requirement that
///   `primitive` types are POD.
#[cfg(feature = "enabled")]
#[proc_macro_attribute]
pub fn roc(attr: TokenStream, item: TokenStream) -> TokenStream {
    let arg = syn::parse_macro_input!(attr as RocAttributeArg);
    let input = syn::parse_macro_input!(item as syn::DeriveInput);
    roc_attr::apply_roc_attribute(arg, input, &crate_root_tokens())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

#[cfg(not(feature = "enabled"))]
#[proc_macro_attribute]
pub fn roc(attr: TokenStream, item: TokenStream) -> TokenStream {
    syn::parse_macro_input!(attr as RocAttributeArgs);
    item
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

impl syn::parse::Parse for RocAttributeArg {
    fn parse(input: syn::parse::ParseStream<'_>) -> syn::Result<Self> {
        if input.is_empty() {
            return Ok(RocAttributeArg::None);
        }

        let ident: proc_macro2::Ident = input.parse()?;
        match ident.to_string().as_str() {
            "auto" => Ok(RocAttributeArg::Auto),
            "primitive" => Ok(RocAttributeArg::Primitive),
            "pod" => Ok(RocAttributeArg::Pod),
            "inline" => Ok(RocAttributeArg::Inline),
            other => Err(syn::Error::new(
                ident.span(),
                format!(
                    "unknown argument '{other}', must be one of 'pod', 'inline', 'primitive' and 'auto'"
                ),
            )),
        }
    }
}
