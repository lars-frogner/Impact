//!

use std::{
    borrow::Cow,
    fmt::{self, Display},
};

pub trait Roc {
    const ROC_TYPE_ID: RocTypeID;
    const SERIALIZED_SIZE: usize;
}

/// A unique ID identifying a type implementing [`Roc`].
#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RocTypeID(u64);

#[derive(Clone, Debug)]
pub struct RocTypeDescriptor {
    pub id: RocTypeID,
    pub roc_name: &'static str,
    pub serialized_size: usize,
    pub composition: RocTypeComposition,
}

inventory::collect!(RocTypeDescriptor);

// These need to match the corresponding constants in `roc_codegen_macros::roc`.
pub const MAX_ROC_TYPE_ENUM_VARIANTS: usize = 8;
pub const MAX_ROC_TYPE_ENUM_VARIANT_FIELDS: usize = 2;
pub const MAX_ROC_TYPE_STRUCT_FIELDS: usize =
    MAX_ROC_TYPE_ENUM_VARIANTS * MAX_ROC_TYPE_ENUM_VARIANT_FIELDS;

#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug)]
pub enum RocTypeComposition {
    Primitive(RocPrimitiveKind),
    Struct(RocTypeFields<MAX_ROC_TYPE_STRUCT_FIELDS>),
    Enum(RocTypeVariants<MAX_ROC_TYPE_ENUM_VARIANTS, MAX_ROC_TYPE_ENUM_VARIANT_FIELDS>),
}

#[derive(Clone, Debug)]
pub enum RocPrimitiveKind {
    Builtin,
    LibraryProvided {
        precision: Option<RocLibraryPrimitivePrecision>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RocLibraryPrimitivePrecision {
    Single,
    Double,
}

#[derive(Clone, Debug)]
pub enum RocTypeFields<const N_FIELDS: usize> {
    None,
    Named(StaticList<NamedRocTypeField, N_FIELDS>),
    Unnamed(StaticList<UnnamedRocTypeField, N_FIELDS>),
}

#[derive(Clone, Debug)]
pub struct RocTypeVariants<const N_VARIANTS: usize, const N_FIELDS: usize>(
    pub StaticList<RocTypeVariant<N_FIELDS>, N_VARIANTS>,
);

#[derive(Clone, Debug)]
pub struct RocTypeVariant<const N_FIELDS: usize> {
    pub ident: &'static str,
    pub size: usize,
    pub fields: RocTypeFields<N_FIELDS>,
}

#[derive(Clone, Debug)]
pub struct StaticList<T, const N: usize>(pub [Option<T>; N]);

#[derive(Clone, Debug)]
pub struct NamedRocTypeField {
    pub ident: &'static str,
    pub type_id: RocTypeID,
}

#[derive(Clone, Debug)]
pub struct UnnamedRocTypeField {
    pub type_id: RocTypeID,
}

impl RocTypeID {
    pub const fn hashed_from_str(string: &str) -> Self {
        Self(const_fnv1a_hash::fnv1a_hash_str_64(string))
    }
}

impl fmt::Display for RocTypeID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl RocTypeDescriptor {
    pub fn import_module(&self, module_prefix: &str, core_prefix: &str) -> String {
        match &self.composition {
            RocTypeComposition::Primitive(RocPrimitiveKind::Builtin) => {
                format!("{core_prefix}Core.Builtin as Builtin")
            }
            RocTypeComposition::Primitive(RocPrimitiveKind::LibraryProvided { .. }) => {
                format!("{core_prefix}Core.{name} as {name}", name = self.roc_name)
            }
            RocTypeComposition::Struct(_) | RocTypeComposition::Enum(_) => {
                format!("{module_prefix}{name} as {name}", name = self.roc_name)
            }
        }
    }

    pub fn concrete_roc_name(&self) -> Cow<'static, str> {
        if let Some(precision) = self.precision() {
            Cow::Owned(format!(
                "{} {}",
                self.roc_name,
                precision.roc_type_variable()
            ))
        } else {
            Cow::Borrowed(self.roc_name)
        }
    }

    pub fn write_bytes_func_name(&self) -> String {
        self.serialization_func_name("write_bytes")
    }

    pub fn from_bytes_func_name(&self) -> String {
        self.serialization_func_name("from_bytes")
    }

    fn serialization_func_name(&self, func_base: impl Display) -> String {
        match &self.composition {
            RocTypeComposition::Primitive(RocPrimitiveKind::Builtin) => {
                format!("Builtin.{}_{}!", func_base, self.roc_name.to_lowercase())
            }
            RocTypeComposition::Primitive(RocPrimitiveKind::LibraryProvided {
                precision: Some(precision),
            }) => {
                format!(
                    "{}.{}_{}!",
                    self.roc_name,
                    func_base,
                    precision.bit_count_str()
                )
            }
            _ => {
                format!("{}.{}!", self.roc_name, func_base)
            }
        }
    }

    fn precision(&self) -> Option<RocLibraryPrimitivePrecision> {
        if let RocTypeComposition::Primitive(RocPrimitiveKind::LibraryProvided {
            precision: Some(precision),
            ..
        }) = &self.composition
        {
            Some(*precision)
        } else {
            None
        }
    }
}

impl RocLibraryPrimitivePrecision {
    const fn roc_type_variable(self) -> &'static str {
        match self {
            Self::Single => "Binary32",
            Self::Double => "Binary64",
        }
    }

    const fn bit_count_str(self) -> &'static str {
        match self {
            Self::Single => "32",
            Self::Double => "64",
        }
    }
}

impl<T, const N: usize> StaticList<T, N> {
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
