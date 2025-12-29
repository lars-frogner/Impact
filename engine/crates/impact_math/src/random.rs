//! Randomness.

pub mod halton;
pub mod power_law;
pub mod splitmix;

use std::ops::{Range, RangeBounds};

/// A non-cryptographic pseudo-random number generator.
#[derive(Debug)]
pub struct Rng(fastrand::Rng);

impl Rng {
    /// Creates a new RNG from the given seed.
    #[inline]
    pub fn with_seed(seed: u64) -> Self {
        Self(fastrand::Rng::with_seed(seed))
    }

    /// Generates a uniformly random `f32` in the range `0.0..1.0`.
    #[inline]
    pub fn random_f32_fraction(&mut self) -> f32 {
        self.0.f32()
    }

    /// Generates a uniformly random `f32` in the given range.
    #[inline]
    pub fn random_f32_in_range(&mut self, range: Range<f32>) -> f32 {
        let t = self.random_f32_fraction();
        range.start + t * (range.end - range.start)
    }

    /// Generates a uniformly random `u32` in the given range.
    #[inline]
    pub fn random_u32_in_range(&mut self, range: impl RangeBounds<u32>) -> u32 {
        self.0.u32(range)
    }

    /// Generates a uniformly random `u64` in the given range.
    #[inline]
    pub fn random_u64_in_range(&mut self, range: impl RangeBounds<u64>) -> u64 {
        self.0.u64(range)
    }

    /// Copies a uniformly random subset of the items in a source slice over to
    /// a destination slice, overwriting the existing items. If the source slice
    /// is smaller than the destination slice, only the first `source.len()`
    /// items in the destination will be replaced.
    #[inline]
    pub fn clone_random_subset_from_slice<T: Clone>(&mut self, dest: &mut [T], source: &[T]) {
        let count = dest.len();

        if count == 0 {
            // Nothing to do
            return;
        }

        // Fill dest with the first items of source
        dest.clone_from_slice(&source[..count.min(source.len())]);

        // If we exhausted the source, we're done
        if count >= source.len() {
            return;
        }

        // Keep index of current source item
        let mut idx = count;

        for item in &source[count..] {
            // Pick random index in currenly covered range, and if it falls into
            // the destination window, replace the item at that index (reservoir
            // sampling)
            let x = self.0.usize(0..=idx);

            if x < count {
                dest[x] = item.clone();
            }

            idx += 1;
        }
    }

    /// Fills the given slice with uniformly random bytes.
    #[inline]
    pub fn fill_byte_slice(&mut self, slice: &mut [u8]) {
        self.0.fill(slice);
    }
}
