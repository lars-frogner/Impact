//! Procedural macros for generating equivalents of Rust types in Roc.

mod roc;

use lazy_static::lazy_static;
use proc_macro::TokenStream;
use proc_macro_crate::{self, FoundCrate};
use proc_macro2::{Ident, Span};
use syn::{DeriveInput, parse_macro_input};

/// Derive macro generating an impl of the trait [`Roc`].
#[proc_macro_derive(Roc)]
pub fn derive_roc(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    roc::impl_roc(input, &crate_root_ident())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

const CRATE_NAME: &str = "roc_codegen";

lazy_static! {
    static ref CRATE_IMPORT_ROOT: String = determine_crate_import_root();
}

/// Determines whether to use `crate` or the actual crate name as root
/// for `use` statements.
fn determine_crate_import_root() -> String {
    let found_crate =
        proc_macro_crate::crate_name(CRATE_NAME).expect("roc_codegen not found in Cargo.toml");
    match found_crate {
        FoundCrate::Itself => "crate".to_string(),
        FoundCrate::Name(name) => name,
    }
}

fn crate_root_ident() -> Ident {
    Ident::new(CRATE_IMPORT_ROOT.as_str(), Span::call_site())
}
