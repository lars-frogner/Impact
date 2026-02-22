//! Morton encoding.

use crate::vector::Vector3C;

/// Encoder for converting 3D floating-point coordinates into a 30-bit Morton
/// code.
#[derive(Clone, Debug)]
pub struct MortonEncoder30Bit3D(MortonEncoder3D);

/// Encoder for converting 3D floating-point coordinates into a 63-bit Morton
/// code.
#[derive(Clone, Debug)]
pub struct MortonEncoder63Bit3D(MortonEncoder3D);

#[derive(Clone, Debug)]
struct MortonEncoder3D {
    max_quantized: f32,
    coord_offsets: Vector3C,
    coord_scales: Vector3C,
}

impl MortonEncoder30Bit3D {
    /// Creates a new 30-bit Morton encoder for 3D coordinates within the given
    /// bounds. Input coordinates will be clamped to these bounds during
    /// encoding.
    ///
    /// # Panics
    /// If `min > max`.
    pub fn new(min_coords: &Vector3C, max_coords: &Vector3C) -> Self {
        assert!(
            min_coords.x() <= max_coords.x()
                && min_coords.y() <= max_coords.y()
                && min_coords.z() <= max_coords.z()
        );
        Self(MortonEncoder3D::new_30_bit(min_coords, max_coords))
    }

    /// Computes the 30-bit Morton code of the given coordinates.
    #[inline]
    pub fn encode(&self, coords: &Vector3C) -> u32 {
        self.0.encode_30_bit(coords)
    }
}

impl MortonEncoder63Bit3D {
    /// Creates a new 63-bit Morton encoder for 3D coordinates within the given
    /// bounds. Input coordinates will be clamped to these bounds during
    /// encoding.
    ///
    /// # Panics
    /// If `min > max`.
    pub fn new(min_coords: &Vector3C, max_coords: &Vector3C) -> Self {
        Self(MortonEncoder3D::new_63_bit(min_coords, max_coords))
    }

    /// Computes the 63-bit Morton code of the given coordinates.
    #[inline]
    pub fn encode(&self, coords: &Vector3C) -> u64 {
        self.0.encode_63_bit(coords)
    }
}

impl MortonEncoder3D {
    fn new_30_bit(min_coords: &Vector3C, max_coords: &Vector3C) -> Self {
        Self::new(10, min_coords, max_coords)
    }

    fn new_63_bit(min_coords: &Vector3C, max_coords: &Vector3C) -> Self {
        Self::new(21, min_coords, max_coords)
    }

    fn new(bits_per_dimension: u8, min_coords: &Vector3C, max_coords: &Vector3C) -> Self {
        let max_quantized = ((1_u64 << bits_per_dimension) - 1) as f32;

        let coord_offsets = -min_coords;

        let coord_scales = Vector3C::new(
            Self::scale_for_dimension(max_quantized, min_coords.x(), max_coords.x()),
            Self::scale_for_dimension(max_quantized, min_coords.y(), max_coords.y()),
            Self::scale_for_dimension(max_quantized, min_coords.z(), max_coords.z()),
        );

        Self {
            max_quantized,
            coord_offsets,
            coord_scales,
        }
    }

    fn scale_for_dimension(max_quantized: f32, min: f32, max: f32) -> f32 {
        let range = f64::from(max) - f64::from(min);
        if range == 0.0 {
            0.0
        } else {
            (f64::from(max_quantized) / range) as f32
        }
    }

    #[inline]
    fn quantize(&self, coords: &Vector3C) -> (f32, f32, f32) {
        let quantized = (coords + self.coord_offsets)
            .component_mul(&self.coord_scales)
            .component_clamp(0.0, self.max_quantized);

        (quantized.x(), quantized.y(), quantized.z())
    }

    #[inline]
    fn encode_30_bit(&self, coords: &Vector3C) -> u32 {
        let (x, y, z) = self.quantize(coords);
        morton_encode_3d_30_bits(x as u32, y as u32, z as u32)
    }

    #[inline]
    fn encode_63_bit(&self, coords: &Vector3C) -> u64 {
        let (x, y, z) = self.quantize(coords);
        morton_encode_3d_63_bits(x as u64, y as u64, z as u64)
    }
}

/// Computes the 30-bit Morton code of three integer coordinates where only the
/// lower 10 bits are used.
#[inline]
pub const fn morton_encode_3d_30_bits(x: u32, y: u32, z: u32) -> u32 {
    (u32_spread_bits_by_two(x) << 2) | (u32_spread_bits_by_two(y) << 1) | u32_spread_bits_by_two(z)
}

/// Computes the 63-bit Morton code of three integer coordinates where only the
/// lower 21 bits are used.
#[inline]
pub const fn morton_encode_3d_63_bits(x: u64, y: u64, z: u64) -> u64 {
    (u64_spread_bits_by_two(x) << 2) | (u64_spread_bits_by_two(y) << 1) | u64_spread_bits_by_two(z)
}

#[inline]
const fn u32_spread_bits_by_two(mut x: u32) -> u32 {
    x = (x | (x << 16)) & 0x030000FF;
    x = (x | (x << 8)) & 0x0300F00F;
    x = (x | (x << 4)) & 0x030C30C3;
    x = (x | (x << 2)) & 0x09249249;
    x
}

#[inline]
const fn u64_spread_bits_by_two(mut x: u64) -> u64 {
    x = (x | x << 32) & 0x001F00000000FFFF;
    x = (x | x << 16) & 0x001F0000FF0000FF;
    x = (x | x << 8) & 0x100F00F00F00F00F;
    x = (x | x << 4) & 0x10C30C30C30C30C3;
    x = (x | x << 2) & 0x1249249249249249;
    x
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn u32_spread_bits_by_two_works() {
        let ipt = 0b000000000000000000001111111111;
        let opt = 0b001001001001001001001001001001;
        assert_eq!(u32_spread_bits_by_two(ipt), opt);
    }

    #[test]
    fn u64_spread_bits_by_two_works() {
        let ipt = 0b000000000000000000000000000000000000000000111111111111111111111;
        let opt = 0b001001001001001001001001001001001001001001001001001001001001001;
        assert_eq!(u64_spread_bits_by_two(ipt), opt);
    }
}
