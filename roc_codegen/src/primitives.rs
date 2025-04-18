//! Registration of foreign types as Roc primitive types (as considered by the
//! engine).

#[macro_export]
macro_rules! impl_roc_for_existing_primitive {
    ($t:ty, $roc_name:expr, $kind:expr) => {
        impl $crate::meta::Roc for $t {
            const ROC_TYPE_ID: $crate::meta::RocTypeID =
                $crate::meta::RocTypeID::hashed_from_str(stringify!($t));
            const SERIALIZED_SIZE: usize = ::std::mem::size_of::<$t>();
        }
        impl $crate::meta::RocPod for $t {}

        inventory::submit! {
            $crate::meta::RocTypeDescriptor {
                id: <$t as $crate::meta::Roc>::ROC_TYPE_ID,
                roc_name: $roc_name,
                serialized_size: <$t as $crate::meta::Roc>::SERIALIZED_SIZE,
                flags: $crate::meta::RocTypeFlags::IS_POD,
                composition: $crate::meta::RocTypeComposition::Primitive($kind),
                docstring: "",
            }
        }
    };
}

#[macro_export]
macro_rules! impl_roc_for_builtin_primitives {
    ($($t:ty => $roc_name:expr),+ $(,)?) => {
        $(
            $crate::impl_roc_for_existing_primitive!($t, $roc_name, $crate::meta::RocPrimitiveKind::Builtin);
        )*
    };
}

#[macro_export]
macro_rules! impl_roc_for_library_provided_primitives {
    ($($t:ty => $roc_name:expr, $precision:ident),+ $(,)?) => {
        $(
            $crate::impl_roc_for_existing_primitive!(
                $t,
                $roc_name,
                $crate::meta::RocPrimitiveKind::LibraryProvided {
                    precision: $crate::meta::RocLibraryPrimitivePrecision::$precision,
                }
            );
        )*
    };
    ($($t:ty => $roc_name:expr),+ $(,)?) => {
        $(
            $crate::impl_roc_for_existing_primitive!(
                $t,
                $roc_name,
                $crate::meta::RocPrimitiveKind::LibraryProvided {
                    precision: $crate::meta::RocLibraryPrimitivePrecision::PrecisionIrrelevant,
                }
            );
        )*
    };
}

// Roc's builtin primitive types
#[cfg(feature = "enabled")]
impl_roc_for_builtin_primitives! {
    u8 => "U8",
    u16 => "U16",
    u32 => "U32",
    u64 => "U64",
    u128 => "U128",
    i8 => "I8",
    i16 => "I16",
    i32 => "I32",
    i64 => "I64",
    i128 => "I128",
    f32 => "F32",
    f64 => "F64",
}

// The Roc definitions and impementations of these types are hand-coded in a
// Roc library rather than generated.
#[cfg(feature = "enabled")]
impl_roc_for_library_provided_primitives! {
    usize => "Usize", PrecisionIrrelevant,
    nalgebra::Vector2<f32> => "Vector2", SinglePrecision,
    nalgebra::Vector2<f64> => "Vector2", DoublePrecision,
    nalgebra::Vector3<f32> => "Vector3", SinglePrecision,
    nalgebra::Vector3<f64> => "Vector3", DoublePrecision,
    nalgebra::Vector4<f32> => "Vector4", SinglePrecision,
    nalgebra::Vector4<f64> => "Vector4", DoublePrecision,
    nalgebra::Matrix3<f32> => "Matrix3", SinglePrecision,
    nalgebra::Matrix3<f64> => "Matrix3", DoublePrecision,
    nalgebra::Matrix4<f32> => "Matrix4", SinglePrecision,
    nalgebra::Matrix4<f64> => "Matrix4", DoublePrecision,
    nalgebra::UnitVector3<f32> => "UnitVector3", SinglePrecision,
    nalgebra::UnitVector3<f64> => "UnitVector3", DoublePrecision,
    nalgebra::UnitQuaternion<f32> => "UnitQuaternion", SinglePrecision,
    nalgebra::UnitQuaternion<f64> => "UnitQuaternion", DoublePrecision,
    nalgebra::Point3<f32> => "Point3", SinglePrecision,
    nalgebra::Point3<f64> => "Point3", DoublePrecision,
}
