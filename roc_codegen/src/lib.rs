//! Automatic generation of [Roc](roc-lang.org) code for interfacing with the
//! engine.

pub mod meta;

#[macro_use]
mod primitives;

#[cfg(feature = "enabled")]
pub mod generate;

/// Attribute macro for annotating Rust types that should be available in Roc.
/// The macro will infer and register a
/// [`RocTypeDescriptor`](crate::meta::RocTypeDescriptor) for the target type,
/// which is used to [`generate`](crate::generate) the associated Roc code.
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
pub use roc_codegen_macros::roc;

pub use meta::RocTypeID;
