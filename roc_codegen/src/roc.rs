//!

use bytemuck::Pod;
use nalgebra::{Vector3, Vector4};

pub trait Roc: Pod {}

#[derive(Debug)]
pub struct RocTypeDescriptor {
    pub size: usize,
    pub name: &'static str,
    pub definition: Option<&'static str>,
}

inventory::collect!(RocTypeDescriptor);

macro_rules! impl_roc_for_primitives {
    ($($t:ty => $roc_name:expr),+) => {
        $(
            impl $crate::roc::Roc for $t {}

            inventory::submit! {
                $crate::roc::RocTypeDescriptor {
                    size: ::std::mem::size_of::<$t>(),
                    name: $roc_name,
                    definition: None,
                }
            }
        )*
    };
}

impl_roc_for_primitives! {
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
    Vector3<f32> => "Vector3F32",
    Vector4<f32> => "Vector4F32",
    Vector3<f64> => "Vector3F64",
    Vector4<f64> => "Vector3F64"
}
