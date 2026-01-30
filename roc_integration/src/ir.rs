//! Intermediate representation of code for translation between Rust and Roc.

use crate::{RocTypeID, utils::StaticList};

// These need to match the corresponding constants in `roc_integration_macros`.
// We can't use dynamically sized collections in the IR, since the macros must
// be able to define values statically.
pub const MAX_ENUM_VARIANTS: usize = 32;
pub const MAX_ENUM_VARIANT_FIELDS: usize = 4;
pub const MAX_STRUCT_FIELDS: usize = MAX_ENUM_VARIANTS * MAX_ENUM_VARIANT_FIELDS;
pub const MAX_BITFLAGS: usize = 64;

pub const MAX_FUNCTION_ARGS: usize = 16;

pub const MAX_DEPENDENCIES: usize = 16;
pub const MAX_LITERAL_IMPORTS: usize = 4;

#[derive(Clone, Debug)]
pub struct Type {
    /// A unique ID for the type.
    pub id: RocTypeID,
    /// The docstring for the type.
    pub docstring: &'static str,
    /// The name of the type.
    pub name: &'static str,
    /// Information about the layout and contents of this type.
    pub composition: TypeComposition,
}

#[derive(Clone, Debug)]
pub struct AssociatedDependencies {
    /// The type having these dependencies.
    pub for_type_id: RocTypeID,
    /// The types being depended on.
    pub type_dependencies: StaticList<RocTypeID, MAX_DEPENDENCIES>,
    /// Literal items to import in addition to depended on types.
    pub literal_imports: StaticList<&'static str, MAX_LITERAL_IMPORTS>,
}

/// A constant associated with a specific type.
#[derive(Clone, Debug)]
pub struct AssociatedConstant {
    /// The type the constant is associated with.
    pub for_type_id: RocTypeID,
    /// The position of this constant in the sequence of associated constants
    /// for the type (to preserve ordering).
    pub sequence_number: usize,
    /// The docstring for the constant.
    pub docstring: &'static str,
    /// The name of the constant.
    pub name: &'static str,
    /// The type of the constant.
    pub ty: AssociatedConstantType,
    /// The source code for the constant's expression.
    pub expr: &'static str,
}

/// A function or method associated with a specific type.
#[derive(Clone, Debug)]
pub struct AssociatedFunction {
    /// The type this function is associated with.
    pub for_type_id: RocTypeID,
    /// The position of this function in the sequence of associated functions
    /// for the type (to preserve ordering).
    pub sequence_number: usize,
    /// The docstring for the function.
    pub docstring: &'static str,
    /// The name of the function.
    pub name: &'static str,
    /// The arguments of the function.
    pub arguments: FunctionArguments<MAX_FUNCTION_ARGS>,
    /// The source code for the body of the function.
    pub body: &'static str,
    /// The return type of the function.
    pub return_type: AssociatedFunctionReturnType,
    /// Whether the function body produces side effects.
    pub is_effectful: bool,
}

#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug)]
pub enum TypeComposition {
    /// Types that are not generated from Rust code.
    Primitive(PrimitiveKind),
    Struct {
        /// [`std::mem::align_of`] this struct.
        alignment: usize,
        fields: TypeFields<MAX_STRUCT_FIELDS>,
    },
    Enum(TypeVariants<MAX_ENUM_VARIANTS, MAX_ENUM_VARIANT_FIELDS>),
    /// Bitflags types with named bit constants.
    Bitflags(Bitflags<MAX_BITFLAGS>),
}

#[derive(Clone, Debug)]
pub enum PrimitiveKind {
    /// Roc's builtin primitive types.
    Builtin,
    /// Non-builtin types whose Roc equivalents will be defined and implemented
    /// in a Roc package/library rather than generated from Rust code.
    LibraryProvided {
        /// If the library-provided primitive has single- and double-precision
        /// versions, this specifies which one this instance of the type uses.
        precision: PrimitivePrecision,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrimitivePrecision {
    PrecisionIrrelevant,
    SinglePrecision,
    DoublePrecision,
}

/// Struct fields, either in an explicit struct or in enum variants.
#[derive(Clone, Debug)]
pub enum TypeFields<const N_FIELDS: usize> {
    None,
    Named(StaticList<NamedTypeField, N_FIELDS>),
    Unnamed(StaticList<UnnamedTypeField, N_FIELDS>),
}

/// The variants of an enum.
#[derive(Clone, Debug)]
pub struct TypeVariants<const N_VARIANTS: usize, const N_FIELDS: usize>(
    pub StaticList<TypeVariant<N_FIELDS>, N_VARIANTS>,
);

/// A list of bitflags constants.
#[derive(Clone, Debug)]
pub struct Bitflags<const N_FLAGS: usize>(pub StaticList<Bitflag, N_FLAGS>);

/// An enum variant.
#[derive(Clone, Debug)]
pub struct TypeVariant<const N_FIELDS: usize> {
    /// The docstring (in Roc format) for the enum variant.
    pub docstring: &'static str,
    /// The identifier (name) of the variant.
    pub ident: &'static str,
    /// The serialized size of this variant's payload.
    pub serialized_size: usize,
    /// The fields of the struct representing this variant's payload.
    pub fields: TypeFields<N_FIELDS>,
}

/// Explicitly named struct fields.
#[derive(Clone, Debug)]
pub struct NamedTypeField {
    /// The docstring (in Roc format) for the struct field.
    pub docstring: &'static str,
    /// The identifier (name) of the struct field.
    pub ident: &'static str,
    /// This struct field's Roc type.
    pub ty: FieldType,
}

/// Unnamed (tuple) struct fields.
#[derive(Clone, Debug)]
pub struct UnnamedTypeField {
    /// This tuple field's Roc type.
    pub ty: FieldType,
}

/// A field that is either a single concrete type or an array of such types.
#[derive(Clone, Debug)]
pub enum FieldType {
    Single { type_id: RocTypeID },
    Array { elem_type_id: RocTypeID, len: usize },
}

/// A single named bitflags constant.
#[derive(Clone, Debug)]
pub struct Bitflag {
    /// The name of the flag.
    pub name: &'static str,
    /// The bit representing this flag.
    pub bit: u32,
}

/// The type of an associated constant.
pub type AssociatedConstantType = Containable<Inferrable<TranslatableType>>;

/// The list of arguments for a function.
#[derive(Clone, Debug)]
pub struct FunctionArguments<const N_ARGS: usize>(pub StaticList<FunctionArgument, N_ARGS>);

/// A function or method argument.
#[derive(Clone, Debug)]
pub enum FunctionArgument {
    Receiver(MethodReceiver),
    Typed(TypedFunctionArgument),
}

/// The receiver of a method, which is some form of `self`, like `&self`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MethodReceiver {
    RefSelf,
    OwnedSelf,
}

/// A typed function argument.
#[derive(Clone, Debug)]
pub struct TypedFunctionArgument {
    /// The argument name.
    pub ident: &'static str,
    /// The argument type.
    pub ty: FunctionArgumentType,
}

/// The type of a function argument.
#[derive(Clone, Debug)]
pub enum FunctionArgumentType {
    Explicit(ExplicitFunctionArgumentType),
    Ignored,
}

/// The explicit type of a function argument.
pub type ExplicitFunctionArgumentType = Containable<Inferrable<TranslatableType>>;

/// The return type of an associated function.
#[derive(Clone, Debug)]
pub enum AssociatedFunctionReturnType {
    Explicit(ExplicitAssociatedFunctionReturnType),
    Ignored,
}

/// The explicit return type of an associated function.
pub type ExplicitAssociatedFunctionReturnType = Containable<Inferrable<TranslatableType>>;

/// Wrapper for types that may appear in a container.
#[derive(Clone, Debug)]
pub enum Containable<T> {
    Single(T),
    List(T),
    Tuple2(T, T),
    Tuple3(T, T, T),
    Result(T),
}

/// Either a specific type or a type that must be inferred from the context.
#[derive(Clone, Debug)]
pub enum Inferrable<T> {
    SelfType,
    Specific(T),
}

/// A type that can be translated between Rust and Roc. This means that it
/// has either been registered (in which case it has a `RocTypeID`), or is a
/// special type whose translation is handled explicitly.
#[derive(Clone, Debug)]
pub enum TranslatableType {
    Registered(RocTypeID),
    Special(SpecialType),
}

/// A special type whose translation between Rust and Roc is handled
/// explicitly.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpecialType {
    String,
}

/// In what context a type name is being invoked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TypeUsage {
    /// The type is used directly as a concrete type (e.g., `MyType`)
    Concrete,
    /// The type is used as a parameter in a type constructor
    /// (e.g., `List MyType`)
    TypeParameter,
}
