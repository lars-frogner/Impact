//! Procedural macros for generating equivalents of Rust types in Roc.

mod roc;

use std::str::FromStr;

use lazy_static::lazy_static;
use proc_macro::TokenStream;
use proc_macro_crate::{self, FoundCrate};
use proc_macro2::TokenStream as TokenStream2;
use syn::{DeriveInput, parse_macro_input};

/// Derive macro generating an implementation of the
/// [`Roc`](roc_codegen::meta::Roc) trait. The macro will infer and register a
/// [`RocTypeDescriptor`](roc_codegen::meta::RocTypeDescriptor) for the target
/// type, which is used to [`generate`](roc_codegen::generate) the associated
/// Roc code.
///
/// Note that this registration is only performed when the crate hosting the
/// target type has an active feature named `roc_codegen` and the `enabled`
/// feature is active for the [`roc_codegen`] crate.
///
/// Deriving the `Roc` trait will only work for types whose constituent types
/// all implement `Roc` or [`RocPod`](roc_codegen::meta::RocPod).
#[proc_macro_derive(Roc)]
pub fn derive_roc(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    roc::impl_roc(input, &crate_root_tokens())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Derive macro generating an implementation of the
/// [`RocPod`](roc_codegen::meta::RocPod) trait. The macro will infer and
/// register a [`RocTypeDescriptor`](roc_codegen::meta::RocTypeDescriptor)
/// for the target type, which is used to [`generate`](roc_codegen::generate)
/// the associated Roc code.
///
/// Note that this registration is only performed when the crate hosting the
/// target type has an active feature named `roc_codegen` and the `enabled`
/// feature is active for the [`roc_codegen`] crate.
///
/// The `RocPod` trait represents [`Roc`](roc_codegen::meta::Roc) types
/// consisting of Plain Old Data (POD). Derive this trait rather that the `Roc`
/// trait for types implementing [`Pod`](bytemuck::Pod), as this will unlock
/// more use cases for the generated Roc code. In particular, ECS components,
/// which are always POD, should always derive this trait rather than plain
/// `Roc`.
///
/// Deriving the `RocPod` trait will only work for types whose constituent
/// types are all POD and implement `Roc` or `RocPod`.
#[proc_macro_derive(RocPod)]
pub fn derive_roc_pod(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    roc::impl_roc_pod(input, &crate_root_tokens())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
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
