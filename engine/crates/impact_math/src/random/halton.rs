//! Halton sequence generator.

use std::iter::FusedIterator;

/// A Halton sequence for a given base, implemented as an iterator.
///
/// The base should be a prime number greater than 1. If multiple sequences are
/// combined to produce multidimensional points, each sequence should use a
/// different prime base. The smallest primes (2, 3, 5, ..) are good choices.
///
/// The iterator yields quasi-random numbers in the range (0, 1).
#[derive(Clone, Debug)]
pub struct HaltonSequence {
    base: u64,
    n: u64,
    d: u64,
}

impl HaltonSequence {
    /// Creates a new Halton sequence for the given base.
    ///
    /// The base should be a prime number greater than 1. If multiple sequences
    /// are combined to produce multidimensional points, each sequence should
    /// use a different prime base. The smallest primes (2, 3, 5, ..) are good
    /// choices.
    ///
    /// # Panics
    /// If the base does not exceed 1.
    pub fn new(base: u64) -> Self {
        assert!(base > 1);
        Self { base, n: 0, d: 1 }
    }
}

impl Iterator for HaltonSequence {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        let x = self.d - self.n;
        if x == 1 {
            self.n = 1;
            self.d *= self.base;
        } else {
            let mut y = self.d / self.base;
            while x <= y {
                y /= self.base;
            }
            self.n = (self.base + 1) * y - x;
        }
        let out = self.n as f64 / self.d as f64;
        Some(out as f32)
    }
}

impl FusedIterator for HaltonSequence {}

impl Default for HaltonSequence {
    fn default() -> Self {
        Self::new(2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn halton_sequence_for_base_2_is_correct() {
        let halton = HaltonSequence::new(2);
        let expected = [
            1.0 / 2.0,
            1.0 / 4.0,
            3.0 / 4.0,
            1.0 / 8.0,
            5.0 / 8.0,
            3.0 / 8.0,
            7.0 / 8.0,
            1.0 / 16.0,
            9.0 / 16.0,
        ];
        for (i, x) in halton.take(9).enumerate() {
            assert_eq!(x, expected[i]);
        }
    }

    #[test]
    fn halton_sequence_for_base_3_is_correct() {
        let halton = HaltonSequence::new(3);
        let expected = [
            1.0 / 3.0,
            2.0 / 3.0,
            1.0 / 9.0,
            4.0 / 9.0,
            7.0 / 9.0,
            2.0 / 9.0,
            5.0 / 9.0,
            8.0 / 9.0,
            1.0 / 27.0,
        ];
        for (i, x) in halton.take(9).enumerate() {
            assert_eq!(x, expected[i]);
        }
    }
}
