//! Registration of foreign types as Roc primitive types (as considered by the
//! engine).

#[macro_export]
macro_rules! impl_roc_for_existing_primitive {
    ($t:ty, $package:ident, $module:ident, $name:ident, $postfix:expr, $kind:expr) => {
        impl $crate::meta::Roc for $t {
            const ROC_TYPE_ID: $crate::meta::RocTypeID =
                $crate::meta::RocTypeID::hashed_from_str(stringify!($t));
            const SERIALIZED_SIZE: usize = ::std::mem::size_of::<$t>();
        }
        impl $crate::meta::RocPod for $t {}

        inventory::submit! {
            $crate::meta::RocTypeDescriptor {
                id: <$t as $crate::meta::Roc>::ROC_TYPE_ID,
                package_name: stringify!($package),
                module_name: stringify!($module),
                type_name: stringify!($name),
                function_postfix: $postfix,
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
    ($($t:ty => $package:ident, $module:ident, $name:ident, $postfix:expr),+ $(,)?) => {
        $(
            $crate::impl_roc_for_existing_primitive!(
                $t,
                $package, $module, $name, $postfix,
                $crate::meta::RocPrimitiveKind::Builtin
            );
        )*
    };
}

#[macro_export]
macro_rules! impl_roc_for_library_provided_primitives {
    ($($t:ty => $package:ident, $module:ident, $name:ident, $postfix:expr, $precision:ident),+ $(,)?) => {
        $(
            $crate::impl_roc_for_existing_primitive!(
                $t,
                $package, $module, $name, $postfix,
                $crate::meta::RocPrimitiveKind::LibraryProvided {
                    precision: $crate::meta::RocPrimitivePrecision::$precision,
                }
            );
        )*
    };
    ($($t:ty => $package:ident, $module:ident, $name:ident, $postfix:expr),+ $(,)?) => {
        $(
            $crate::impl_roc_for_existing_primitive!(
                $t,
                $package, $module, $name, $postfix,
                $crate::meta::RocPrimitiveKind::LibraryProvided {
                    precision: $crate::meta::RocPrimitivePrecision::PrecisionIrrelevant,
                }
            );
        )*
    };
}

// Roc's builtin primitive types
#[cfg(feature = "enabled")]
impl_roc_for_builtin_primitives! {
    u8   => core, Builtin, U8,   "_u8",
    u16  => core, Builtin, U16,  "_u16",
    u32  => core, Builtin, U32,  "_u32",
    u64  => core, Builtin, U64,  "_u64",
    u128 => core, Builtin, U128, "_u128",
    i8   => core, Builtin, I8,   "_i8",
    i16  => core, Builtin, I16,  "_i16",
    i32  => core, Builtin, I32,  "_i32",
    i64  => core, Builtin, I64,  "_i64",
    i128 => core, Builtin, I128, "_i128",
    f32  => core, Builtin, F32,  "_f32",
    f64  => core, Builtin, F64,  "_f64",
}

// The Roc definitions and impementations of these types are hand-coded in a
// Roc library rather than generated.
#[cfg(feature = "enabled")]
impl_roc_for_library_provided_primitives! {
    usize                         => core, NativeNum,      Usize,          "_usize", PrecisionIrrelevant,
    nalgebra::Vector2<f32>        => core, Vector2,        Vector2,        "_32",    SinglePrecision,
    nalgebra::Vector2<f64>        => core, Vector2,        Vector2,        "_64",    DoublePrecision,
    nalgebra::Vector3<f32>        => core, Vector3,        Vector3,        "_32",    SinglePrecision,
    nalgebra::Vector3<f64>        => core, Vector3,        Vector3,        "_64",    DoublePrecision,
    nalgebra::Vector4<f32>        => core, Vector4,        Vector4,        "_32",    SinglePrecision,
    nalgebra::Vector4<f64>        => core, Vector4,        Vector4,        "_64",    DoublePrecision,
    nalgebra::Matrix3<f32>        => core, Matrix3,        Matrix3,        "_32",    SinglePrecision,
    nalgebra::Matrix3<f64>        => core, Matrix3,        Matrix3,        "_64",    DoublePrecision,
    nalgebra::Matrix4<f32>        => core, Matrix4,        Matrix4,        "_32",    SinglePrecision,
    nalgebra::Matrix4<f64>        => core, Matrix4,        Matrix4,        "_64",    DoublePrecision,
    nalgebra::UnitVector3<f32>    => core, UnitVector3,    UnitVector3,    "_32",    SinglePrecision,
    nalgebra::UnitVector3<f64>    => core, UnitVector3,    UnitVector3,    "_64",    DoublePrecision,
    nalgebra::UnitQuaternion<f32> => core, UnitQuaternion, UnitQuaternion, "_32",    SinglePrecision,
    nalgebra::UnitQuaternion<f64> => core, UnitQuaternion, UnitQuaternion, "_64",    DoublePrecision,
    nalgebra::Point3<f32>         => core, Point3,         Point3,         "_32",    SinglePrecision,
    nalgebra::Point3<f64>         => core, Point3,         Point3,         "_64",    DoublePrecision,
}
