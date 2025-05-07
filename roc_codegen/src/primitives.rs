//! Registration of foreign types as Roc primitive types (as considered by the
//! engine).

#[macro_export]
macro_rules! impl_roc_for_existing_primitive {
    ($t:ty, $prefix:expr, $module:ident, $name:ident, $postfix:expr, $kind:expr) => {
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
                module_prefix: $prefix,
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
    ($($t:ty => $prefix:expr, $module:ident, $name:ident, $postfix:expr),+ $(,)?) => {
        $(
            $crate::impl_roc_for_existing_primitive!(
                $t,
                $prefix, $module, $name, $postfix,
                $crate::ir::PrimitiveKind::Builtin
            );
        )*
    };
}

#[macro_export]
macro_rules! impl_roc_for_library_provided_primitives {
    ($($t:ty => $prefix:expr, $module:ident, $name:ident, $postfix:expr, $precision:ident),+ $(,)?) => {
        $(
            $crate::impl_roc_for_existing_primitive!(
                $t,
                $prefix, $module, $name, $postfix,
                $crate::ir::PrimitiveKind::LibraryProvided {
                    precision: $crate::ir::PrimitivePrecision::$precision,
                }
            );
        )*
    };
    ($($t:ty => $prefix:expr, $module:ident, $name:ident, $postfix:expr),+ $(,)?) => {
        $(
            $crate::impl_roc_for_existing_primitive!(
                $t,
                $prefix, $module, $name, $postfix,
                $crate::PrimitiveKind::LibraryProvided {
                    precision: $crate::PrimitivePrecision::PrecisionIrrelevant,
                }
            );
        )*
    };
}

// Roc's builtin primitive types
#[cfg(feature = "enabled")]
impl_roc_for_builtin_primitives! {
    u8   => "core", Builtin, U8,   "_u8",
    u16  => "core", Builtin, U16,  "_u16",
    u32  => "core", Builtin, U32,  "_u32",
    u64  => "core", Builtin, U64,  "_u64",
    u128 => "core", Builtin, U128, "_u128",
    i8   => "core", Builtin, I8,   "_i8",
    i16  => "core", Builtin, I16,  "_i16",
    i32  => "core", Builtin, I32,  "_i32",
    i64  => "core", Builtin, I64,  "_i64",
    i128 => "core", Builtin, I128, "_i128",
    f32  => "core", Builtin, F32,  "_f32",
    f64  => "core", Builtin, F64,  "_f64",
}

// The Roc definitions and impementations of these types are hand-coded in a
// Roc library rather than generated.
#[cfg(feature = "enabled")]
impl_roc_for_library_provided_primitives! {
    usize                         => "core", NativeNum,      Usize,          "_usize", PrecisionIrrelevant,
    nalgebra::Vector2<f32>        => "core", Vector2,        Vector2,        "_32",    SinglePrecision,
    nalgebra::Vector2<f64>        => "core", Vector2,        Vector2,        "_64",    DoublePrecision,
    nalgebra::Vector3<f32>        => "core", Vector3,        Vector3,        "_32",    SinglePrecision,
    nalgebra::Vector3<f64>        => "core", Vector3,        Vector3,        "_64",    DoublePrecision,
    nalgebra::Vector4<f32>        => "core", Vector4,        Vector4,        "_32",    SinglePrecision,
    nalgebra::Vector4<f64>        => "core", Vector4,        Vector4,        "_64",    DoublePrecision,
    nalgebra::Matrix3<f32>        => "core", Matrix3,        Matrix3,        "_32",    SinglePrecision,
    nalgebra::Matrix3<f64>        => "core", Matrix3,        Matrix3,        "_64",    DoublePrecision,
    nalgebra::Matrix4<f32>        => "core", Matrix4,        Matrix4,        "_32",    SinglePrecision,
    nalgebra::Matrix4<f64>        => "core", Matrix4,        Matrix4,        "_64",    DoublePrecision,
    nalgebra::UnitVector3<f32>    => "core", UnitVector3,    UnitVector3,    "_32",    SinglePrecision,
    nalgebra::UnitVector3<f64>    => "core", UnitVector3,    UnitVector3,    "_64",    DoublePrecision,
    nalgebra::UnitQuaternion<f32> => "core", UnitQuaternion, UnitQuaternion, "_32",    SinglePrecision,
    nalgebra::UnitQuaternion<f64> => "core", UnitQuaternion, UnitQuaternion, "_64",    DoublePrecision,
    nalgebra::Point3<f32>         => "core", Point3,         Point3,         "_32",    SinglePrecision,
    nalgebra::Point3<f64>         => "core", Point3,         Point3,         "_64",    DoublePrecision,
}
