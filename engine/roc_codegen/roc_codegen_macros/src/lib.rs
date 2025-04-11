//! Procedural macros for generating equivalents of Rust types in Roc.

mod roc;

use std::str::FromStr;

use lazy_static::lazy_static;
use proc_macro::TokenStream;
use proc_macro_crate::{self, FoundCrate};
use proc_macro2::TokenStream as TokenStream2;
use syn::{DeriveInput, parse_macro_input};

/// Derive macro generating an impl of the trait [`Roc`].
#[proc_macro_derive(Roc)]
pub fn derive_roc(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    roc::impl_roc(input, &crate_root_tokens())
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
