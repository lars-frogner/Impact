use anyhow::Result;
use bytemuck::{AnyBitPattern, NoUninit};
use impact_utils::{AlignedByteVec, Alignment};
use roc_std::RocList;

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

fn roundtrip<T: AnyBitPattern + NoUninit>(bytes: &RocList<u8>) -> Result<RocList<u8>> {
    let bytes = bytes.as_slice().to_vec();
    let bytes = AlignedByteVec::copied_from_slice(Alignment::of::<T>(), &bytes);
    let interpreted = *bytemuck::try_from_bytes::<T>(&bytes)?;
    let return_bytes = RocList::from_slice(bytemuck::bytes_of(&interpreted));
    Ok(return_bytes)
}
