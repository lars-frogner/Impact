//! Automatic generation of [Roc](roc-lang.org) code for interfacing with the
//! engine.

/// Attribute macro for annotating Rust types and associated methods that
/// should be available in Roc.
///
/// When applied to a Rust type, the macro will infer and register a
/// corresponding `RegisteredType`, which is used to generate a Roc module with
/// a type declaration and some associated utility functions.
///
/// The macro can additionally be applied to the type's `impl` block and
/// selected associated constants and functions therein in order to register
/// associated constants and functions whose generated Roc code will be
/// included in the type's Roc module.
///
/// Note that the registration of types and associated items is only performed
/// when the crate hosting the target type has an active feature named
/// `roc_codegen` and the `roc_codegen` feature is active for the
/// `roc_integration` crate.
///
/// Three categories of types can be annotated with `roc`, and the requested
/// category can be specified as an argument to the macro:
/// `#[roc(category = "<category>")]`. The available categories are:
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
/// argument:
///
/// - `dependencies=[<type1>, <type2>, ..]`: A list of Rust types whose Roc
///   modules should be imported into the module for the present type. The
///   modules for the types comprising the present type will always be
///   imported, so this is only needed when some of the generated methods
///   make use of additional modules.
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
/// specified in an argument like this: `body = "<Roc code>"`. The argument
/// names will be the same in Roc as in Rust. The macro also accepts the
/// following optional argument:
///
/// - `name = "<function name>"`: The name used for the function in Roc.
///   Defaults to the Rust name.
///
/// Not all associated functions can be translated to Roc. The following
/// requirements have to hold for the function signature:
///
/// - Each type in the function signature must be either a primitive or
///   generated Roc type (by reference or value), a string (as `&str` or
///   `String`) or an array, slice, 2- or 3-element tuple or `Result` of such
///   types.
/// - No generic parameters or `impl <Trait>`.
/// - No mutable references.
/// - There must be a return type.
pub use roc_integration_macros::roc;

#[cfg(feature = "roc_codegen")]
pub mod generate;
#[cfg(feature = "roc_codegen")]
pub mod ir;
#[cfg(feature = "roc_codegen")]
pub mod utils;

#[macro_use]
mod primitives;

#[cfg(feature = "roc_codegen")]
pub use inner::*;

use anyhow::Result;
use std::fmt;

/// Represents types that have a Roc equivalent. This should never be
/// implemented directly. Instead, annotate types using the [`roc`]
/// attribute macro.
pub trait Roc: Sized {
    const ROC_TYPE_ID: RocTypeID;

    /// The number of bytes [`Self::write_roc_bytes`] will write to the buffer
    /// when serializing a value of this type.
    const SERIALIZED_SIZE: usize;

    /// Deserializes the first [`Self::SERIALIZED_SIZE`] bytes in the given
    /// slice into a value of this type. The encoding is expected to match that
    /// used by the serialization and deserialization functions associated with
    /// the Roc counterpart of this type, as well as with that used by
    /// [`Self::write_roc_bytes`].
    ///
    /// # Errors
    /// Returns an error if:
    /// - `bytes` does not have size `Self::SERIALIZED_SIZE`.
    /// - An unexpected enum discriminant is encountered.
    fn from_roc_bytes(bytes: &[u8]) -> Result<Self>;

    /// Serializes this value into [`Self::SERIALIZED_SIZE`] bytes and writes
    /// them to the beginning of the given buffer. The encoding matches that
    /// used by the serialization and deserialization functions associated with
    /// the Roc counterpart of this type, as well as with that used by
    /// [`Self::from_roc_bytes`].
    ///
    /// # Errors
    /// Returns an error if `buffer` does not have size
    /// `Self::SERIALIZED_SIZE`.
    fn write_roc_bytes(&self, buffer: &mut [u8]) -> Result<()>;
}

/// Helper trait to enforce that certain Roc types are POD.
pub trait RocPod: Roc + bytemuck::Pod {}

/// A unique ID identifying a type implementing [`Roc`].
#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RocTypeID(u64);

impl RocTypeID {
    pub const fn hashed_from_str(input: &str) -> Self {
        // WARNING: we guarantee that this matches
        // `impact_ecs::component::ComponentID::hashed_from_str`
        let hash = const_fnv1a_hash::fnv1a_hash_str_64(input);
        Self(if hash == 0 { 1 } else { hash }) // Reserve the zero ID
    }

    pub const fn from_u64(value: u64) -> Self {
        Self(value)
    }

    pub const fn as_u64(&self) -> u64 {
        self.0
    }
}

impl fmt::Display for RocTypeID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(feature = "roc_codegen")]
mod inner {
    use super::ir;
    use bitflags::bitflags;
    use std::borrow::Cow;

    /// A type registered for use in Roc.
    #[derive(Clone, Debug)]
    pub struct RegisteredType {
        /// For generated types, the type name qualified by its parent modules can
        /// be specified here for inclusion in the header of the generated Roc
        /// module.
        pub rust_type_path: Option<&'static str>,
        /// The name of the Roc package this type resides in. If not specified, the
        /// package is assumed to be the package code is being generated in. This
        /// is typically specified for primitive types and not for generated types.
        /// When specified for a generated type, that type will only be generated
        /// when the specified package name matches the name of the target package
        /// for generation.
        pub package_name: Option<&'static str>,
        /// The parent Roc module(s) for this type's module, if any. Multiple module
        /// names should be separated by `.`.
        pub parent_modules: Option<&'static str>,
        /// The name of the Roc module where the type will be defined.
        pub module_name: &'static str,
        /// Postfix for the functions operating on this type (for primitive types,
        /// typically when the type's module has both 32- and 64-bit versions of
        /// the type).
        pub function_postfix: Option<&'static str>,
        /// The size in bytes of an object of this type when serialized to match
        /// the ABI used for FFI between the engine and Roc.
        pub serialized_size: usize,
        /// Flags describing various properties of the type.
        pub flags: RegisteredTypeFlags,
        /// The intermediate representation of the type.
        pub ty: ir::Type,
    }

    // Whenever a type is annotated with the `roc` attribute macro, the macro
    // infers a [`RegisteredType`] and registers it using [`inventory::submit!`]
    // (provided that the `enabled` feature is active). This
    // [`inventory::collect!`] allows all type types registered in any crate
    // linked with this one to be gathered using [`inventory::iter`] when we are to
    // [`generate`](crate::generate) the Roc code.
    inventory::collect!(RegisteredType);
    inventory::collect!(ir::AssociatedDependencies);
    inventory::collect!(ir::AssociatedFunction);
    inventory::collect!(ir::AssociatedConstant);

    bitflags! {
        #[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
        pub struct RegisteredTypeFlags: u8 {
            /// Whether the type is Plain Old Data (POD).
            const IS_POD       = 1 << 0;
            /// Whether the type is an ECS component. Note that this flag is
            /// determined at generation time (by comparing against registered
            /// components), not at compile/derive time.
            const IS_COMPONENT = 1 << 1;
        }
    }

    impl RegisteredType {
        /// Whether this type is POD.
        pub fn is_pod(&self) -> bool {
            self.flags.contains(RegisteredTypeFlags::IS_POD)
        }

        /// Whether this type is an ECS component.
        pub fn is_component(&self) -> bool {
            self.flags.contains(RegisteredTypeFlags::IS_COMPONENT)
        }

        /// Whether the type is a primitive type.
        pub fn is_primitive(&self) -> bool {
            matches!(self.ty.composition, ir::TypeComposition::Primitive(_))
        }

        /// Whether the type is a Roc builtin type.
        pub fn is_builtin(&self) -> bool {
            matches!(
                &self.ty.composition,
                ir::TypeComposition::Primitive(ir::PrimitiveKind::Builtin)
            )
        }

        /// The alignment of this type if it is a POD struct.
        pub fn alignment_as_pod_struct(&self) -> Option<usize> {
            if let ir::TypeComposition::Struct { alignment, .. } = &self.ty.composition {
                if self.flags.contains(RegisteredTypeFlags::IS_POD) {
                    return Some(*alignment);
                }
            }
            None
        }

        /// Returns the fully qualified path for the Roc import statement required
        /// for using this type in Roc.
        pub fn import_path(&self, current_package: &str) -> String {
            let mut path = String::new();
            self.write_package_to_path(&mut path, current_package);
            self.write_parent_modules_to_path(&mut path);
            path.push_str(self.module_name);
            path
        }

        /// The fully qualified name of this Roc type, including parent modules,
        /// without any unspecified type variables, e.g. `Vector3 Binary32` as
        /// opposed to just `Vector3` (which could have either 32- or 64-bit
        /// precision). The specified usage context may affect whether the name
        /// with be put in parentheses to avoid ambiguities.
        pub fn qualified_type_name(&self, usage: ir::TypeUsage) -> Cow<'static, str> {
            if let Some(type_variable) = self.roc_type_variable() {
                let mut name = String::new();
                if usage == ir::TypeUsage::TypeParameter {
                    name.push('(');
                }
                self.write_parent_modules_to_path(&mut name);
                self.write_module_to_path(&mut name);
                name.push_str(self.ty.name);
                name.push(' ');
                name.push_str(type_variable);
                if usage == ir::TypeUsage::TypeParameter {
                    name.push(')');
                }
                Cow::Owned(name)
            } else if self.is_builtin() {
                Cow::Borrowed(self.ty.name)
            } else {
                let mut name = String::new();
                self.write_parent_modules_to_path(&mut name);
                self.write_module_to_path(&mut name);
                name.push_str(self.ty.name);
                Cow::Owned(name)
            }
        }

        /// Returns a string describing the type.
        pub fn description(&self) -> String {
            format!(
                "{} ({})",
                self.qualified_type_name(ir::TypeUsage::Concrete),
                self.composition_description()
            )
        }

        /// Returns a label describing the composition of the type.
        pub fn composition_description(&self) -> Cow<'static, str> {
            match &self.ty.composition {
                ir::TypeComposition::Primitive(ir::PrimitiveKind::Builtin) => {
                    Cow::Borrowed("builtin")
                }
                ir::TypeComposition::Primitive(ir::PrimitiveKind::LibraryProvided { .. }) => {
                    match self.package_name {
                        Some(package) => Cow::Owned(format!("primitive from {package}")),
                        None => Cow::Borrowed("primitive"),
                    }
                }
                ir::TypeComposition::Struct {
                    fields: ir::TypeFields::None,
                    ..
                } => Cow::Borrowed("unit struct"),
                ir::TypeComposition::Struct {
                    fields: ir::TypeFields::Named(..),
                    ..
                } => Cow::Borrowed("struct"),
                ir::TypeComposition::Struct {
                    fields: ir::TypeFields::Unnamed(..),
                    ..
                } => Cow::Borrowed("tuple struct"),
                ir::TypeComposition::Enum(_) => Cow::Borrowed("enum"),
            }
        }

        /// The qualified function name to use when invoking this type's standard
        /// (always generated or pre-implemented) `write_bytes` function in Roc.
        pub fn write_bytes_func_name(&self) -> String {
            self.qualified_func_name("write_bytes")
        }

        /// The qualified function name to use when invoking this type's standard
        /// (always generated or pre-implemented) `from_bytes` function in Roc.
        pub fn from_bytes_func_name(&self) -> String {
            self.qualified_func_name("from_bytes")
        }

        fn qualified_func_name(&self, func_base: &str) -> String {
            let mut path = String::new();
            self.write_parent_modules_to_path(&mut path);
            self.write_module_to_path(&mut path);
            path.push_str(func_base);
            path.push_str(self.function_postfix.unwrap_or_default());
            path
        }

        fn write_package_to_path(&self, path: &mut String, current_package: &str) {
            match self.package_name {
                Some(package) if package != current_package => {
                    path.push_str(package);
                    path.push('.');
                }
                _ => {}
            }
        }

        fn write_parent_modules_to_path(&self, path: &mut String) {
            if let Some(parent_modules) = self.parent_modules {
                path.push_str(parent_modules);
                path.push('.');
            }
        }

        fn write_module_to_path(&self, path: &mut String) {
            path.push_str(self.module_name);
            path.push('.');
        }

        fn roc_type_variable(&self) -> Option<&'static str> {
            if let ir::TypeComposition::Primitive(ir::PrimitiveKind::LibraryProvided {
                precision,
                ..
            }) = &self.ty.composition
            {
                precision.roc_type_variable()
            } else {
                None
            }
        }
    }

    impl ir::PrimitivePrecision {
        const fn roc_type_variable(self) -> Option<&'static str> {
            match self {
                Self::PrecisionIrrelevant => None,
                Self::SinglePrecision => Some("Binary32"),
                Self::DoublePrecision => Some("Binary64"),
            }
        }
    }
}
