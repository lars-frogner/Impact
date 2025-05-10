//! Registration of foreign types as Roc primitive types (as considered by the
//! engine).

#[macro_export]
macro_rules! impl_roc_for_existing_primitive {
    ($t:ty, $package:ident, $parents:expr, $module:ident, $name:ident, $postfix:expr, $kind:expr) => {
        impl $crate::Roc for $t {
            const ROC_TYPE_ID: $crate::RocTypeID =
                $crate::RocTypeID::hashed_from_str(stringify!($t));
            const SERIALIZED_SIZE: usize = ::std::mem::size_of::<Self>();

            fn from_roc_bytes(bytes: &[u8]) -> ::anyhow::Result<Self> {
                if bytes.len() != <Self as $crate::Roc>::SERIALIZED_SIZE {
                    ::anyhow::bail!(
                        "Expected {} bytes for `{}`, got {}",
                        <Self as $crate::Roc>::SERIALIZED_SIZE,
                        stringify!($t),
                        bytes.len()
                    );
                }
                Ok(::bytemuck::pod_read_unaligned(bytes))
            }

            fn write_roc_bytes(&self, buffer: &mut [u8]) -> ::anyhow::Result<()> {
                if buffer.len() != <Self as $crate::Roc>::SERIALIZED_SIZE {
                    ::anyhow::bail!(
                        "Expected buffer of length {} for writing `{}`, got {}",
                        <Self as $crate::Roc>::SERIALIZED_SIZE,
                        stringify!($t),
                        buffer.len()
                    );
                }
                buffer.copy_from_slice(::bytemuck::bytes_of(self));
                Ok(())
            }
        }
        impl $crate::RocPod for $t {}

        inventory::submit! {
            $crate::RegisteredType {
                rust_type_path: None,
                package_name: Some(stringify!($package)),
                parent_modules: $parents,
                module_name: stringify!($module),
                function_postfix: $postfix,
                serialized_size: <$t as $crate::Roc>::SERIALIZED_SIZE,
                flags: $crate::RegisteredTypeFlags::IS_POD,
                ty: $crate::ir::Type {
                    id: <$t as $crate::Roc>::ROC_TYPE_ID,
                    docstring: "",
                    name: stringify!($name),
                    composition: $crate::ir::TypeComposition::Primitive($kind),
                }
            }
        }
    };
}

#[macro_export]
macro_rules! impl_roc_for_builtin_primitives {
    ($($t:ty => $package:ident, $parents:expr, $module:ident, $name:ident, $postfix:expr),+ $(,)?) => {
        $(
            $crate::impl_roc_for_existing_primitive!(
                $t,
                $package, $parents, $module, $name, $postfix,
                $crate::ir::PrimitiveKind::Builtin
            );
        )*
    };
}

#[macro_export]
macro_rules! impl_roc_for_library_provided_primitives {
    ($($t:ty => $package:ident, $parents:expr, $module:ident, $name:ident, $postfix:expr, $precision:ident),+ $(,)?) => {
        $(
            $crate::impl_roc_for_existing_primitive!(
                $t,
                $package, $parents, $module, $name, $postfix,
                $crate::ir::PrimitiveKind::LibraryProvided {
                    precision: $crate::ir::PrimitivePrecision::$precision,
                }
            );
        )*
    };
    ($($t:ty => $package:ident, $parents:expr, $module:ident, $name:ident, $postfix:expr),+ $(,)?) => {
        $(
            $crate::impl_roc_for_existing_primitive!(
                $t,
                $package, $parents, $module, $name, $postfix,
                $crate::PrimitiveKind::LibraryProvided {
                    precision: $crate::PrimitivePrecision::PrecisionIrrelevant,
                }
            );
        )*
    };
}

// Roc's builtin primitive types
impl_roc_for_builtin_primitives! {
//  Type    Pkg   Parents  Module   Roc name  Postfix
    u8   => core, None,    Builtin, U8,       Some("_u8"),
    u16  => core, None,    Builtin, U16,      Some("_u16"),
    u32  => core, None,    Builtin, U32,      Some("_u32"),
    u64  => core, None,    Builtin, U64,      Some("_u64"),
    u128 => core, None,    Builtin, U128,     Some("_u128"),
    i8   => core, None,    Builtin, I8,       Some("_i8"),
    i16  => core, None,    Builtin, I16,      Some("_i16"),
    i32  => core, None,    Builtin, I32,      Some("_i32"),
    i64  => core, None,    Builtin, I64,      Some("_i64"),
    i128 => core, None,    Builtin, I128,     Some("_i128"),
    f32  => core, None,    Builtin, F32,      Some("_f32"),
    f64  => core, None,    Builtin, F64,      Some("_f64"),
}

// The Roc definitions and impementations of these types are hand-coded in a
// Roc library rather than generated.
impl_roc_for_library_provided_primitives! {
//  Type                             Pkg   Parents  Module          Roc name        Postfix         Precision
    usize                         => core, None,    NativeNum,      Usize,          Some("_usize"), PrecisionIrrelevant,
    nalgebra::Vector2<f32>        => core, None,    Vector2,        Vector2,        Some("_32"),    SinglePrecision,
    nalgebra::Vector2<f64>        => core, None,    Vector2,        Vector2,        Some("_64"),    DoublePrecision,
    nalgebra::Vector3<f32>        => core, None,    Vector3,        Vector3,        Some("_32"),    SinglePrecision,
    nalgebra::Vector3<f64>        => core, None,    Vector3,        Vector3,        Some("_64"),    DoublePrecision,
    nalgebra::Vector4<f32>        => core, None,    Vector4,        Vector4,        Some("_32"),    SinglePrecision,
    nalgebra::Vector4<f64>        => core, None,    Vector4,        Vector4,        Some("_64"),    DoublePrecision,
    nalgebra::Matrix3<f32>        => core, None,    Matrix3,        Matrix3,        Some("_32"),    SinglePrecision,
    nalgebra::Matrix3<f64>        => core, None,    Matrix3,        Matrix3,        Some("_64"),    DoublePrecision,
    nalgebra::Matrix4<f32>        => core, None,    Matrix4,        Matrix4,        Some("_32"),    SinglePrecision,
    nalgebra::Matrix4<f64>        => core, None,    Matrix4,        Matrix4,        Some("_64"),    DoublePrecision,
    nalgebra::UnitVector3<f32>    => core, None,    UnitVector3,    UnitVector3,    Some("_32"),    SinglePrecision,
    nalgebra::UnitVector3<f64>    => core, None,    UnitVector3,    UnitVector3,    Some("_64"),    DoublePrecision,
    nalgebra::UnitQuaternion<f32> => core, None,    UnitQuaternion, UnitQuaternion, Some("_32"),    SinglePrecision,
    nalgebra::UnitQuaternion<f64> => core, None,    UnitQuaternion, UnitQuaternion, Some("_64"),    DoublePrecision,
    nalgebra::Point3<f32>         => core, None,    Point3,         Point3,         Some("_32"),    SinglePrecision,
    nalgebra::Point3<f64>         => core, None,    Point3,         Point3,         Some("_64"),    DoublePrecision,
}
