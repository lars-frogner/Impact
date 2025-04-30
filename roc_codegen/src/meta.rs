//! Metadata for Rust types whose Roc equivalents should be generated.

use bitflags::bitflags;
use std::{
    borrow::Cow,
    fmt::{self, Display},
};

/// Represents types that have a Roc equivalent. This should never be
/// implemented directly. Instead, annotate types using the
/// [`roc`](crate::roc) attribute macro.
pub trait Roc {
    const ROC_TYPE_ID: RocTypeID;
    const SERIALIZED_SIZE: usize;
}

/// Helper trait to enforce that certain Roc types are POD.
pub trait RocPod: Roc + bytemuck::Pod {}

/// A unique ID identifying a type implementing [`Roc`].
#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RocTypeID(u64);

/// Metadata for types that can be converted to Roc.
#[derive(Clone, Debug)]
pub struct RocTypeDescriptor {
    /// A unique ID for the type.
    pub id: RocTypeID,
    /// The name of the Roc package where the type will be defined.
    pub package_name: &'static str,
    /// The name of the Roc module where the type will be defined.
    pub module_name: &'static str,
    /// The name of the type in Roc.
    pub type_name: &'static str,
    /// Postfix for the functions operating on this type.
    pub function_postfix: &'static str,
    /// The size in bytes of an object of this type when serialized to match
    /// the ABI used for FFI between the engine and Roc.
    pub serialized_size: usize,
    /// Flags describing various properties of the type.
    pub flags: RocTypeFlags,
    /// Information about the layout and contents of this type.
    pub composition: RocTypeComposition,
    /// The docstring (in Roc format) for the type.
    pub docstring: &'static str,
}

#[derive(Clone, Debug)]
pub struct RocConstructorDescriptor {
    /// The position of this constructor in the sequence of constructors for
    /// the type.
    pub sequence_number: usize,
    /// The type that this is a constructor for.
    pub for_type_id: RocTypeID,
    /// The name of the constructor function.
    pub function_name: &'static str,
    /// The arguments of the constructor function.
    pub arguments: RocFunctionArguments<MAX_ROC_CONSTRUCTOR_ARGS>,
    /// The Roc source code for the body of the constructor function.
    pub roc_body: &'static str,
    /// The docstring (in Roc format) for the method.
    pub docstring: &'static str,
}

#[derive(Clone, Debug)]
pub struct RocDependencies {
    /// The type having these dependencies.
    pub for_type_id: RocTypeID,
    /// The types being depended on.
    pub dependencies: StaticList<RocTypeID, MAX_ROC_DEPENDENCIES>,
}

// Whenever a type is annotated with the `roc` attribute macro, the macro
// infers a [`RocTypeDescriptor`] and registers it using [`inventory::submit!`]
// (provided that the `enabled` feature is active). This
// [`inventory::collect!`] allows all type descriptors registered in any crate
// linked with this one to be gathered using [`inventory::iter`] when we are to
// [`generate`](crate::generate) the Roc code.
#[cfg(feature = "enabled")]
inventory::collect!(RocTypeDescriptor);

#[cfg(feature = "enabled")]
inventory::collect!(RocConstructorDescriptor);

#[cfg(feature = "enabled")]
inventory::collect!(RocDependencies);

bitflags! {
    #[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
    pub struct RocTypeFlags: u8 {
        /// Whether the type is Plain Old Data (POD).
        const IS_POD       = 1 << 0;
        /// Whether the type is an ECS component. Note that this flag is
        /// determined at generation time (by comparing against registered
        /// components), not at compile/derive time.
        const IS_COMPONENT = 1 << 1;
    }
}

// These need to match the corresponding constants in `roc_codegen_macros`.
// We can't use dynamically sized collections for this information, since the
// [`RocTypeDescriptor`]s must allow us to define them statically.
pub const MAX_ROC_TYPE_ENUM_VARIANTS: usize = 8;
pub const MAX_ROC_TYPE_ENUM_VARIANT_FIELDS: usize = 2;
pub const MAX_ROC_TYPE_STRUCT_FIELDS: usize =
    MAX_ROC_TYPE_ENUM_VARIANTS * MAX_ROC_TYPE_ENUM_VARIANT_FIELDS;

pub const MAX_ROC_CONSTRUCTOR_ARGS: usize = 16;

pub const MAX_ROC_DEPENDENCIES: usize = 16;

#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug)]
pub enum RocTypeComposition {
    /// Types that are not generated from Rust code.
    Primitive(RocPrimitiveKind),
    Struct {
        /// [`std::mem::align_of`] this struct.
        alignment: usize,
        fields: RocTypeFields<MAX_ROC_TYPE_STRUCT_FIELDS>,
    },
    Enum(RocTypeVariants<MAX_ROC_TYPE_ENUM_VARIANTS, MAX_ROC_TYPE_ENUM_VARIANT_FIELDS>),
}

#[derive(Clone, Debug)]
pub enum RocPrimitiveKind {
    /// Roc's builtin primitive types.
    Builtin,
    /// Non-builtin types whose Roc equivalents will be defined and implemented
    /// in a Roc package/library rather than generated from Rust code.
    LibraryProvided {
        /// If the library-provided primitive has single- and double-precision
        /// versions, this specifies which one this instance of the type uses.
        precision: RocPrimitivePrecision,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RocPrimitivePrecision {
    PrecisionIrrelevant,
    SinglePrecision,
    DoublePrecision,
}

/// Struct fields, either in an explicit struct or in enum variants.
#[derive(Clone, Debug)]
pub enum RocTypeFields<const N_FIELDS: usize> {
    None,
    Named(StaticList<NamedRocTypeField, N_FIELDS>),
    Unnamed(StaticList<UnnamedRocTypeField, N_FIELDS>),
}

/// The variants of an enum.
#[derive(Clone, Debug)]
pub struct RocTypeVariants<const N_VARIANTS: usize, const N_FIELDS: usize>(
    pub StaticList<RocTypeVariant<N_FIELDS>, N_VARIANTS>,
);

/// An enum variant.
#[derive(Clone, Debug)]
pub struct RocTypeVariant<const N_FIELDS: usize> {
    /// The docstring (in Roc format) for the enum variant.
    pub docstring: &'static str,
    /// The identifier (name) of the variant.
    pub ident: &'static str,
    /// The memory size of the struct representing this variant's payload.
    pub size: usize,
    /// [`std::mem::align_of`] the struct representing this variant's payload.
    pub alignment: usize,
    /// The fields of the struct representing this variant's payload.
    pub fields: RocTypeFields<N_FIELDS>,
}

/// Explicitly named struct fields.
#[derive(Clone, Debug)]
pub struct NamedRocTypeField {
    /// The docstring (in Roc format) for the struct field.
    pub docstring: &'static str,
    /// The identifier (name) of the struct field.
    pub ident: &'static str,
    /// This struct field's Roc type.
    pub ty: RocFieldType,
}

/// Unnamed (tuple) struct fields.
#[derive(Clone, Debug)]
pub struct UnnamedRocTypeField {
    /// This tuple field's Roc type.
    pub ty: RocFieldType,
}

/// A field that is either a single concrete type or an array of such types.
#[derive(Clone, Debug)]
pub enum RocFieldType {
    Single { type_id: RocTypeID },
    Array { elem_type_id: RocTypeID, len: usize },
}

/// The list of arguments for a function.
#[derive(Clone, Debug)]
pub struct RocFunctionArguments<const N_ARGS: usize>(pub StaticList<RocFunctionArgument, N_ARGS>);

/// A function argument.
#[derive(Clone, Debug)]
pub struct RocFunctionArgument {
    /// The argument name.
    pub ident: &'static str,
    /// The argument type.
    pub type_id: RocTypeID,
}

#[derive(Clone, Debug)]
pub struct StaticList<T, const N: usize>(pub [Option<T>; N]);

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

impl RocTypeDescriptor {
    /// Returns the fully qualified Roc import statement required for using
    /// this type in Roc.
    pub fn import_module(
        &self,
        import_prefix: &str,
        core_package_name: &str,
        platform_package_name: &str,
    ) -> String {
        match &self.composition {
            RocTypeComposition::Primitive(_) => {
                let package_name = match self.package_name {
                    "pf" => platform_package_name,
                    "core" => core_package_name,
                    name => name,
                };
                format!(
                    "{package_name}.{module_name} as {module_name}",
                    module_name = self.module_name
                )
            }
            RocTypeComposition::Struct { .. } | RocTypeComposition::Enum(_) => {
                format!(
                    "{import_prefix}{module_name} as {module_name}",
                    module_name = self.module_name
                )
            }
        }
    }

    /// Returns a string describing the type.
    pub fn description(&self) -> String {
        format!(
            "{} ({})",
            self.resolved_type_name(false),
            self.composition_description()
        )
    }

    /// The name of this Roc type without any unspecified type variables, e.g.
    /// `Vector3 Binary32` as opposed to just `Vector3` (which could have
    /// either 32- or 64-bit precision). The name may also be prefixed by its
    /// module path if required to fully specify the type based on how it was
    /// imported.
    pub fn resolved_type_name(&self, use_parenthesis: bool) -> Cow<'static, str> {
        if let Some(type_variable) = self.roc_type_variable() {
            Cow::Owned(format!(
                "{open_paren}{module_name}.{type_name} {type_variable}{close_paren}",
                module_name = self.module_name,
                type_name = self.type_name,
                open_paren = if use_parenthesis { "(" } else { "" },
                close_paren = if use_parenthesis { ")" } else { "" }
            ))
        } else if matches!(
            &self.composition,
            RocTypeComposition::Primitive(RocPrimitiveKind::Builtin)
        ) {
            Cow::Borrowed(self.type_name)
        } else {
            Cow::Owned(format!(
                "{module_name}.{type_name}",
                module_name = self.module_name,
                type_name = self.type_name
            ))
        }
    }

    /// Returns a label describing the composition of the type.
    pub fn composition_description(&self) -> Cow<'static, str> {
        match &self.composition {
            RocTypeComposition::Primitive(RocPrimitiveKind::Builtin) => Cow::Borrowed("builtin"),
            RocTypeComposition::Primitive(RocPrimitiveKind::LibraryProvided { .. }) => {
                Cow::Owned(format!("from package {}", self.package_name))
            }
            RocTypeComposition::Struct {
                fields: RocTypeFields::None,
                ..
            } => Cow::Borrowed("unit struct"),
            RocTypeComposition::Struct {
                fields: RocTypeFields::Named(..),
                ..
            } => Cow::Borrowed("struct"),
            RocTypeComposition::Struct {
                fields: RocTypeFields::Unnamed(..),
                ..
            } => Cow::Borrowed("tuple struct"),
            RocTypeComposition::Enum(_) => Cow::Borrowed("enum"),
        }
    }

    /// The qualified function name to use when evoking this type's standard
    /// (always generated or pre-implemented) `write_bytes` function in Roc.
    pub fn write_bytes_func_name(&self) -> String {
        self.resolved_func_name("write_bytes")
    }

    /// The qualified function name to use when evoking this type's standard
    /// (always generated or pre-implemented) `from_bytes` function in Roc.
    pub fn from_bytes_func_name(&self) -> String {
        self.resolved_func_name("from_bytes")
    }

    fn resolved_func_name(&self, func_base: impl Display) -> String {
        format!(
            "{module_name}.{func_base}{postfix}",
            module_name = self.module_name,
            postfix = self.function_postfix
        )
    }

    /// The alignment of this type if it is a POD struct.
    pub fn alignment_as_pod_struct(&self) -> Option<usize> {
        if let RocTypeComposition::Struct { alignment, .. } = &self.composition {
            if self.flags.contains(RocTypeFlags::IS_POD) {
                return Some(*alignment);
            }
        }
        None
    }

    /// Whether this type is POD.
    pub fn is_pod(&self) -> bool {
        self.flags.contains(RocTypeFlags::IS_POD)
    }

    /// Whether this type is an ECS component.
    pub fn is_component(&self) -> bool {
        self.flags.contains(RocTypeFlags::IS_COMPONENT)
    }

    fn roc_type_variable(&self) -> Option<&'static str> {
        if let RocTypeComposition::Primitive(RocPrimitiveKind::LibraryProvided {
            precision, ..
        }) = &self.composition
        {
            precision.roc_type_variable()
        } else {
            None
        }
    }
}

impl RocPrimitivePrecision {
    const fn roc_type_variable(self) -> Option<&'static str> {
        match self {
            Self::PrecisionIrrelevant => None,
            Self::SinglePrecision => Some("Binary32"),
            Self::DoublePrecision => Some("Binary64"),
        }
    }
}

impl<T, const N: usize> StaticList<T, N> {
    pub fn len(&self) -> usize {
        self.iter().count()
    }

    pub fn is_empty(&self) -> bool {
        self.0.first().is_none_or(|first| first.is_none())
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.into_iter()
    }
}

impl<'a, T, const N: usize> IntoIterator for &'a StaticList<T, N> {
    type Item = &'a T;
    type IntoIter =
        std::iter::FilterMap<std::slice::Iter<'a, Option<T>>, fn(&'a Option<T>) -> Option<&'a T>>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.as_slice().iter().filter_map(Option::as_ref)
    }
}

impl<T, const N: usize> Default for StaticList<T, N> {
    fn default() -> Self {
        Self(std::array::from_fn(|_| None))
    }
}
