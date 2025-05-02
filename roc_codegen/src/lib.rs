//! Automatic generation of [Roc](roc-lang.org) code for interfacing with the
//! engine.

pub mod meta;

#[macro_use]
mod primitives;

#[cfg(feature = "enabled")]
pub mod generate;

/// Attribute macro for annotating Rust types and associated methods that
/// should be available in Roc.
///
/// When applied to a Rust type, the macro will infer and register a
/// corresponding [`RocType`](crate::meta::RocType), which is used to
/// [`generate`](crate::generate) a Roc module with a type declaration and some
/// associated utility functions.
///
/// The macro can additionally be applied to the type's `impl` block and
/// selected methods therein in order to register
/// [`RocMethod`](crate::meta::RocMethod)s whose generated Roc code will be
/// included in the type's Roc module.
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
pub use roc_codegen_macros::roc;

pub use meta::RocTypeID;
