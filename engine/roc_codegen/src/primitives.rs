//!

use super::meta::RocLibraryPrimitivePrecision::{Double, Single};
use nalgebra::{Matrix3, Matrix4, Point3, UnitQuaternion, UnitVector3, Vector3, Vector4};

macro_rules! impl_roc_for_primitive {
    ($t:ty, $roc_name:expr, $kind:expr) => {
        impl $crate::meta::Roc for $t {
            const ROC_TYPE_ID: $crate::meta::RocTypeID =
                $crate::meta::RocTypeID::hashed_from_str(stringify!($t));
            const SERIALIZED_SIZE: usize = ::std::mem::size_of::<$t>();
        }

        inventory::submit! {
            $crate::meta::RocTypeDescriptor {
                id: <$t as $crate::meta::Roc>::ROC_TYPE_ID,
                roc_name: $roc_name,
                serialized_size: <$t as $crate::meta::Roc>::SERIALIZED_SIZE,
                composition: $crate::meta::RocTypeComposition::Primitive($kind),
            }
        }
    };
}

macro_rules! impl_roc_for_builtin_primitives {
    ($($t:ty => $roc_name:expr),+ $(,)?) => {
        $(
            impl_roc_for_primitive!($t, $roc_name, $crate::meta::RocPrimitiveKind::Builtin);
        )*
    };
}

macro_rules! impl_roc_for_library_provided_primitives {
    ($($t:ty => $roc_name:expr, $precision:expr),+ $(,)?) => {
        $(
            impl_roc_for_primitive!(
                $t,
                $roc_name,
                $crate::meta::RocPrimitiveKind::LibraryProvided {
                    precision: $precision,
                }
            );
        )*
    };
}

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

impl_roc_for_library_provided_primitives! {
    Vector3<f32> => "Vector3", Some(Single),
    Vector3<f64> => "Vector3", Some(Double),
    Vector4<f32> => "Vector4", Some(Single),
    Vector4<f64> => "Vector4", Some(Double),
    Matrix3<f32> => "Matrix3", Some(Single),
    Matrix3<f64> => "Matrix3", Some(Double),
    Matrix4<f32> => "Matrix4", Some(Single),
    Matrix4<f64> => "Matrix4", Some(Double),
    UnitVector3<f32> => "UnitVector3", Some(Single),
    UnitVector3<f64> => "UnitVector3", Some(Double),
    UnitQuaternion<f32> => "UnitQuaternion", Some(Single),
    UnitQuaternion<f64> => "UnitQuaternion", Some(Double),
    Point3<f32> => "Point3", Some(Single),
    Point3<f64> => "Point3", Some(Double),
}
