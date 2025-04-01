use anyhow::Result;
use bytemuck::{AnyBitPattern, NoUninit, Pod, Zeroable};
use impact_utils::{AlignedByteVec, Alignment};
use roc_std::RocList;

#[repr(C)]
#[derive(Clone, Copy, Debug, Zeroable, Pod)]
struct RoundtripTestStruct {
    field_1: [f32; 3],
    field_2: f32,
    field_3: [f64; 4],
    field_4: f64,
    field_5: [f64; 3],
    field_6: [f32; 4],
    field_7: u64,
    field_8: u32,
    field_9: i32,
    field_10: i64,
}

pub fn f32_to_bits(value: f32) -> u32 {
    value.to_bits()
}

pub fn f64_to_bits(value: f64) -> u64 {
    value.to_bits()
}

pub fn f32_from_bits(bits: u32) -> f32 {
    f32::from_bits(bits)
}

pub fn f64_from_bits(bits: u64) -> f64 {
    f64::from_bits(bits)
}

pub fn vec3_f32_roundtrip(bytes: &RocList<u8>) -> Result<RocList<u8>> {
    roundtrip::<[f32; 3]>(bytes)
}

pub fn vec4_f32_roundtrip(bytes: &RocList<u8>) -> Result<RocList<u8>> {
    roundtrip::<[f32; 4]>(bytes)
}

pub fn vec3_f64_roundtrip(bytes: &RocList<u8>) -> Result<RocList<u8>> {
    roundtrip::<[f64; 3]>(bytes)
}

pub fn vec4_f64_roundtrip(bytes: &RocList<u8>) -> Result<RocList<u8>> {
    roundtrip::<[f64; 4]>(bytes)
}

pub fn test_struct_roundtrip(bytes: &RocList<u8>) -> Result<RocList<u8>> {
    roundtrip::<RoundtripTestStruct>(bytes)
}

fn roundtrip<T: AnyBitPattern + NoUninit>(bytes: &RocList<u8>) -> Result<RocList<u8>> {
    let bytes = bytes.as_slice().to_vec();
    let bytes = AlignedByteVec::copied_from_slice(Alignment::of::<T>(), &bytes);
    let interpreted = *bytemuck::try_from_bytes::<T>(&bytes)?;
    let return_bytes = RocList::from_slice(bytemuck::bytes_of(&interpreted));
    Ok(return_bytes)
}
