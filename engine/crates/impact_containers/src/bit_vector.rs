//! Bit vector for storing bits compactly.

use allocator_api2::{
    alloc::{Allocator, Global},
    vec::Vec as AVec,
};

/// A bit vector that stores bits packed into 64-bit words.
#[derive(Clone, Debug)]
pub struct BitVector<A: Allocator = Global> {
    words: AVec<u64, A>,
    len: usize,
}

impl BitVector {
    /// Creates a new empty bit vector.
    pub fn new() -> Self {
        Self::new_in(Global)
    }

    /// Creates a new empty bit vector with the specified bit capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self::with_capacity_in(capacity, Global)
    }

    /// Creates a new bit vector with the specified length and all bits unset.
    pub fn zeroed(len: usize) -> Self {
        Self::zeroed_in(len, Global)
    }
}

impl<A: Allocator> BitVector<A> {
    /// Creates a new empty bit vector with the specified allocator.
    pub fn new_in(alloc: A) -> Self {
        Self {
            words: AVec::new_in(alloc),
            len: 0,
        }
    }

    /// Creates a new empty bit vector with the specified bit capacity and
    /// allocator.
    pub fn with_capacity_in(capacity: usize, alloc: A) -> Self {
        Self {
            words: AVec::with_capacity_in(capacity.div_ceil(64), alloc),
            len: 0,
        }
    }

    /// Creates a new bit vector with the specified length, allocator, and all
    /// bits unset.
    pub fn zeroed_in(len: usize, alloc: A) -> Self {
        let mut words = AVec::new_in(alloc);
        words.resize(len.div_ceil(64), 0);
        Self { words, len }
    }

    /// Returns whether the bit vector contains no bits.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the number of bits in the bit vector.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Whether the bit at the specified index is set.
    ///
    /// # Panics
    /// If `bit_idx` is greater than or equal to the bit vector length.
    pub fn bit_is_set(&self, bit_idx: usize) -> bool {
        self.bounds_check(bit_idx);
        let word = &self.words[get_word_idx(bit_idx)];
        let mask = get_bit_mask(bit_idx);
        (*word & mask) != 0
    }

    /// Sets the bit at the specified index to 1.
    ///
    /// # Returns
    /// `true` if the bit was already set, `false` if it was unset.
    ///
    /// # Panics
    /// If `bit_idx` is greater than or equal to the bit vector length.
    pub fn set_bit(&mut self, bit_idx: usize) -> bool {
        self.bounds_check(bit_idx);
        let word = &mut self.words[get_word_idx(bit_idx)];
        let mask = get_bit_mask(bit_idx);
        let was_set = (*word & mask) != 0;
        *word |= mask;
        was_set
    }

    /// Sets the bit at the specified index to 0.
    ///
    /// # Returns
    /// `true` if the bit was set, `false` if it was already unset.
    ///
    /// # Panics
    /// If `bit_idx` is greater than or equal to the bit vector length.
    pub fn unset_bit(&mut self, bit_idx: usize) -> bool {
        self.bounds_check(bit_idx);
        let word = &mut self.words[get_word_idx(bit_idx)];
        let mask = get_bit_mask(bit_idx);
        let was_set = (*word & mask) != 0;
        *word &= !mask;
        was_set
    }

    /// Resizes the vector to the specified length, with all bits unset.
    pub fn resize_and_unset_all(&mut self, len: usize) {
        self.words.clear();
        self.words.resize(len.div_ceil(64), 0);
        self.len = len;
    }

    fn bounds_check(&self, bit_idx: usize) {
        if bit_idx >= self.len {
            panic!(
                "Bit index {bit_idx} out of bounds for bit vector of length {}",
                self.len
            );
        }
    }
}

impl Default for BitVector {
    fn default() -> Self {
        Self::new()
    }
}

fn get_word_idx(bit_idx: usize) -> usize {
    bit_idx / 64
}

fn get_bit_mask(bit_idx: usize) -> u64 {
    1 << (bit_idx & 0b111111)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zeroed_creates_bit_vector_with_correct_length_and_unset_bits() {
        let bit_vec = BitVector::zeroed(100);

        assert_eq!(bit_vec.len(), 100);
        assert!(!bit_vec.is_empty());

        // All bits should be unset
        for i in 0..100 {
            assert!(!bit_vec.bit_is_set(i));
        }
    }

    #[test]
    fn with_capacity_creates_empty_bit_vector() {
        let bit_vec = BitVector::with_capacity(128);

        assert_eq!(bit_vec.len(), 0);
        assert!(bit_vec.is_empty());
    }

    #[test]
    fn is_empty_returns_true_for_zero_length() {
        let bit_vec = BitVector::zeroed(0);
        assert!(bit_vec.is_empty());
    }

    #[test]
    fn is_empty_returns_false_for_non_zero_length() {
        let bit_vec = BitVector::zeroed(1);
        assert!(!bit_vec.is_empty());
    }

    #[test]
    fn len_returns_correct_length() {
        let bit_vec = BitVector::zeroed(42);
        assert_eq!(bit_vec.len(), 42);
    }

    #[test]
    fn set_bit_changes_is_set_to_true() {
        let mut bit_vec = BitVector::zeroed(10);

        assert!(!bit_vec.bit_is_set(5));
        let was_set = bit_vec.set_bit(5);

        assert!(!was_set);
        assert!(bit_vec.bit_is_set(5));
    }

    #[test]
    fn set_already_set_bit_returns_true() {
        let mut bit_vec = BitVector::zeroed(10);

        bit_vec.set_bit(3);
        let was_set = bit_vec.set_bit(3);

        assert!(was_set);
        assert!(bit_vec.bit_is_set(3));
    }

    #[test]
    fn unset_bit_changes_is_set_to_false() {
        let mut bit_vec = BitVector::zeroed(10);
        bit_vec.set_bit(7);

        assert!(bit_vec.bit_is_set(7));
        let was_set = bit_vec.unset_bit(7);

        assert!(was_set);
        assert!(!bit_vec.bit_is_set(7));
    }

    #[test]
    fn unset_already_unset_bit_returns_false() {
        let mut bit_vec = BitVector::zeroed(10);

        let was_set = bit_vec.unset_bit(2);

        assert!(!was_set);
        assert!(!bit_vec.bit_is_set(2));
    }

    #[test]
    fn operations_work_across_word_boundaries() {
        let mut bit_vec = BitVector::zeroed(128);

        // Test bits in different words (64-bit words)
        let test_indices = [0, 1, 63, 64, 65, 127];

        // Set all test bits
        for &idx in &test_indices {
            bit_vec.set_bit(idx);
            assert!(bit_vec.bit_is_set(idx));
        }

        // Verify other bits remain unset
        for i in 0..128 {
            if !test_indices.contains(&i) {
                assert!(!bit_vec.bit_is_set(i));
            }
        }

        // Unset all test bits
        for &idx in &test_indices {
            bit_vec.unset_bit(idx);
            assert!(!bit_vec.bit_is_set(idx));
        }
    }

    #[test]
    fn multiple_set_and_unset_operations_work_correctly() {
        let mut bit_vec = BitVector::zeroed(50);

        // Set some bits
        bit_vec.set_bit(10);
        bit_vec.set_bit(25);
        bit_vec.set_bit(40);

        assert!(bit_vec.bit_is_set(10));
        assert!(bit_vec.bit_is_set(25));
        assert!(bit_vec.bit_is_set(40));

        // Unset one bit
        bit_vec.unset_bit(25);

        assert!(bit_vec.bit_is_set(10));
        assert!(!bit_vec.bit_is_set(25));
        assert!(bit_vec.bit_is_set(40));

        // Set the unset bit again
        bit_vec.set_bit(25);

        assert!(bit_vec.bit_is_set(10));
        assert!(bit_vec.bit_is_set(25));
        assert!(bit_vec.bit_is_set(40));
    }

    #[test]
    #[should_panic]
    fn is_set_with_out_of_bounds_index_panics() {
        let bit_vec = BitVector::zeroed(10);
        bit_vec.bit_is_set(10);
    }

    #[test]
    #[should_panic]
    fn set_with_out_of_bounds_index_panics() {
        let mut bit_vec = BitVector::zeroed(5);
        bit_vec.set_bit(5);
    }

    #[test]
    #[should_panic]
    fn unset_with_out_of_bounds_index_panics() {
        let mut bit_vec = BitVector::zeroed(3);
        bit_vec.unset_bit(3);
    }

    #[test]
    fn resize_and_unset_all_to_larger_size_works() {
        let mut bit_vec = BitVector::zeroed(50);

        // Set some bits
        bit_vec.set_bit(10);
        bit_vec.set_bit(25);
        bit_vec.set_bit(40);

        // Resize to larger size
        bit_vec.resize_and_unset_all(100);

        assert_eq!(bit_vec.len(), 100);

        // All bits should be unset, including previously set ones
        for i in 0..100 {
            assert!(!bit_vec.bit_is_set(i));
        }
    }

    #[test]
    fn resize_and_unset_all_to_smaller_size_works() {
        let mut bit_vec = BitVector::zeroed(100);

        // Set some bits across the range
        bit_vec.set_bit(10);
        bit_vec.set_bit(50);
        bit_vec.set_bit(90);

        // Resize to smaller size
        bit_vec.resize_and_unset_all(20);

        assert_eq!(bit_vec.len(), 20);

        // All bits in the new range should be unset
        for i in 0..20 {
            assert!(!bit_vec.bit_is_set(i));
        }
    }
}
