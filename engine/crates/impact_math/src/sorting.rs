//! Sorting algorithms.

use impact_alloc::{AVec, arena::ArenaPool, avec};
use std::{mem, ops::Range};

/// Sorts the given slice of `u64` values in place using Radix sort.
pub fn radix_sort_u64(values: &mut [u64]) {
    radix_sort_lsd_8_bit_u64(values);
}

/// Sorts the given keys using Radix sort and calls the given closure with each
/// `(src_idx, dest_idx)` pair, where `src_idx` is the original index and
/// `dest_idx` is the index in the sorted list.
///
/// The `move_value` closure is guaranteed to be called in sorted order, so it
/// is valid to ignore `dest_idx` and push the value at `src_idx` on each call.
pub fn radix_sort_by_u64_keys(keys: &[u64], move_value: impl FnMut(usize, usize)) {
    radix_sort_lsd_8_bit_by_u64_keys(keys, move_value);
}

/// Sorts the given keys using Radix sort and calls the given closure with each
/// `(src_idx, dest_idx)` pair, where `src_idx` is the original index and
/// `dest_idx` is the index in the sorted list. Takes the pre-determined maximum
/// number of significant bits among the keys.
///
/// The `move_value` closure is guaranteed to be called in sorted order, so it
/// is valid to ignore `dest_idx` and push the value at `src_idx` on each call.
pub fn radix_sort_by_u64_keys_with_max_significant_bits(
    keys: &[u64],
    highest_bit: u32,
    move_value: impl FnMut(usize, usize),
) {
    radix_sort_lsd_8_bit_by_u64_keys_with_max_significant_bits(keys, highest_bit, move_value);
}

pub fn radix_sort_lsd_8_bit_by_u64_keys(keys: &[u64], move_value: impl FnMut(usize, usize)) {
    if keys.len() < 2 {
        return;
    }
    let max_key = keys.iter().copied().max().unwrap();
    let max_significant_bits = 64 - max_key.leading_zeros();
    radix_sort_lsd_8_bit_by_u64_keys_with_max_significant_bits(
        keys,
        max_significant_bits,
        move_value,
    );
}

pub fn radix_sort_lsd_8_bit_by_u64_keys_with_max_significant_bits(
    keys: &[u64],
    max_significant_bits: u32,
    mut move_value: impl FnMut(usize, usize),
) {
    const N_BITS: usize = 8;
    const MAX_PASSES: usize = 64_usize.div_ceil(N_BITS);
    const N_BUCKETS: usize = 1 << N_BITS;
    const MASK: u64 = (N_BUCKETS - 1) as u64;

    #[inline]
    fn right_shift_for_pass(pass_idx: usize) -> u32 {
        pass_idx as u32 * N_BITS as u32
    }

    #[inline]
    fn bucket_idx_for_key(key: u64, right_shift: u32) -> usize {
        ((key >> right_shift) & MASK) as usize
    }

    let n_keys = keys.len();

    if n_keys == 0 {
        return;
    }
    if n_keys == 1 {
        move_value(0, 0);
        return;
    }

    let n_passes = max_significant_bits.div_ceil(N_BITS as u32) as usize;

    let arena = ArenaPool::get_arena();

    let mut entries = AVec::with_capacity_in(n_keys, &arena);
    entries.extend(
        keys.iter()
            .copied()
            .enumerate()
            .map(|(index, key)| ArgSortEntry { key, index }),
    );

    let mut buckets = avec![in &arena; ArgSortEntry::default(); n_keys];

    let mut bucket_sizes_per_pass = [[0_usize; N_BUCKETS]; MAX_PASSES];
    let mut bucket_offsets = [0_usize; N_BUCKETS];

    // Compute number of keys in each bucket for each pass
    for key in keys.iter().copied() {
        for (pass_idx, bucket_sizes) in bucket_sizes_per_pass.iter_mut().enumerate() {
            let key_bucket_idx = bucket_idx_for_key(key, right_shift_for_pass(pass_idx));
            bucket_sizes[key_bucket_idx] += 1;
        }
    }

    for (pass_idx, bucket_sizes) in bucket_sizes_per_pass.iter_mut().enumerate() {
        // Compute where in entry array each bucket must begin
        for bucket_idx in 0..(N_BUCKETS - 1) {
            bucket_offsets[bucket_idx + 1] = bucket_offsets[bucket_idx] + bucket_sizes[bucket_idx];
        }

        let right_shift = right_shift_for_pass(pass_idx);

        for entry in &entries {
            let key_bucket_idx = bucket_idx_for_key(entry.key, right_shift);
            let dest_idx = bucket_offsets[key_bucket_idx];
            unsafe {
                *buckets.get_unchecked_mut(dest_idx) = *entry;
            }
            bucket_offsets[key_bucket_idx] += 1;
        }

        bucket_offsets[0] = 0;

        mem::swap(&mut entries, &mut buckets);
    }

    let sorted_entries = if n_passes.is_multiple_of(2) {
        entries
    } else {
        buckets
    };

    for (idx, entry) in sorted_entries.into_iter().enumerate() {
        move_value(entry.index, idx);
    }
}

pub fn radix_sort_lsd_8_bit_u64(values: &mut [u64]) {
    const N_BITS: usize = 8;
    const MAX_PASSES: usize = 64_usize.div_ceil(N_BITS);
    const N_BUCKETS: usize = 1 << N_BITS;
    const MASK: u64 = (N_BUCKETS - 1) as u64;

    #[inline]
    fn right_shift_for_pass(pass_idx: usize) -> u32 {
        pass_idx as u32 * N_BITS as u32
    }

    #[inline]
    fn bucket_idx_for_value(value: u64, right_shift: u32) -> usize {
        ((value >> right_shift) & MASK) as usize
    }

    let n_values = values.len();

    if n_values < 2 {
        return;
    }

    let max_value = values.iter().copied().max().unwrap();
    let max_digits = 64 - max_value.leading_zeros();

    if max_digits == 0 {
        return;
    }

    let n_passes = max_digits.div_ceil(N_BITS as u32) as usize;

    let arena = ArenaPool::get_arena();
    let mut buckets_vec = avec![in &arena; 0_u64; n_values];

    let mut values = values;
    let mut buckets = buckets_vec.as_mut_slice();

    let mut bucket_sizes_per_pass = [[0_usize; N_BUCKETS]; MAX_PASSES];
    let mut bucket_offsets = [0_usize; N_BUCKETS];

    // Compute number of values in each bucket for each pass
    for value in values.iter().copied() {
        for (pass_idx, bucket_sizes) in bucket_sizes_per_pass.iter_mut().enumerate() {
            let value_bucket_idx = bucket_idx_for_value(value, right_shift_for_pass(pass_idx));
            bucket_sizes[value_bucket_idx] += 1;
        }
    }

    for (pass_idx, bucket_sizes) in bucket_sizes_per_pass.iter_mut().enumerate() {
        // Compute where in value array each bucket must begin
        for bucket_idx in 0..(N_BUCKETS - 1) {
            bucket_offsets[bucket_idx + 1] = bucket_offsets[bucket_idx] + bucket_sizes[bucket_idx];
        }

        let right_shift = right_shift_for_pass(pass_idx);

        for value in values.iter().copied() {
            let value_bucket_idx = bucket_idx_for_value(value, right_shift);
            let dest_idx = bucket_offsets[value_bucket_idx];
            unsafe {
                *buckets.get_unchecked_mut(dest_idx) = value;
            }
            bucket_offsets[value_bucket_idx] += 1;
        }

        bucket_offsets[0] = 0;

        (values, buckets) = (buckets, values);
    }

    if !n_passes.is_multiple_of(2) {
        values.copy_from_slice(buckets);
    }
}

pub fn radix_sort_msd_8_bit_u64(values: &mut [u64]) {
    let n_values = values.len();

    if n_values < 2 {
        return;
    }

    let max_value = values.iter().copied().max().unwrap();
    let max_digits = 64 - max_value.leading_zeros();

    if max_digits == 0 {
        return;
    }

    let right_shift = max_digits.saturating_sub(8);

    let mut bucket_sizes = [0_usize; 256];
    let mut bucket_offsets = [0_usize; 256];

    let mut task_stack = [BucketSortTask::default(); 256 * 2];

    task_stack[0] = BucketSortTask {
        bucket_offset: 0,
        bucket_size: n_values,
        right_shift,
    };
    let mut stack_size = 1;

    while stack_size > 0 {
        stack_size -= 1;
        let task = task_stack[stack_size];

        let values = &mut values[task.bucket_range()];

        // Compute number of values in each bucket
        for value in values.iter().copied() {
            let value_bucket_idx = ((value >> task.right_shift) & 0xFF) as usize;
            bucket_sizes[value_bucket_idx] += 1;
        }

        // Compute where in value array each bucket must begin. Also take
        // opportunity to push tasks for the buckets onto the stack.
        for bucket_idx in 0..255 {
            bucket_offsets[bucket_idx + 1] = bucket_offsets[bucket_idx] + bucket_sizes[bucket_idx];

            if task.right_shift > 0 && bucket_sizes[bucket_idx] > 1 {
                task_stack[stack_size] = BucketSortTask {
                    bucket_offset: bucket_offsets[bucket_idx],
                    bucket_size: bucket_sizes[bucket_idx],
                    right_shift: task.right_shift.saturating_sub(8),
                };
                stack_size += 1;
            }
        }

        // Go through values in each bucket, swap them to the bucket they belong
        // in and increment the offset and decrement the count for that bucket
        for bucket_idx in 0..256 {
            while bucket_sizes[bucket_idx] > 0 {
                let value = values[bucket_offsets[bucket_idx]];
                let value_bucket_idx = ((value >> task.right_shift) & 0xFF) as usize;
                // By swapping even if we are in the correct bucket we avoid a
                // branch
                values.swap(bucket_offsets[bucket_idx], bucket_offsets[value_bucket_idx]);
                bucket_offsets[value_bucket_idx] += 1;
                bucket_sizes[value_bucket_idx] -= 1;
            }
        }

        // All counts are now back to zero, so no need to reset them for next
        // task. We do need to reset the first buffer offset though.
        bucket_offsets[0] = 0;
    }
}

pub fn radix_sort_msd_1_bit_u64(values: &mut [u64]) {
    let n_values = values.len();

    if n_values < 2 {
        return;
    }

    let max_value = values.iter().copied().max().unwrap();
    let max_digits = 64 - max_value.leading_zeros();

    if max_digits == 0 {
        return;
    }

    let right_shift = max_digits - 1;

    let mut task_stack = [BucketSortTask::default(); 64];

    task_stack[0] = BucketSortTask {
        bucket_offset: 0,
        bucket_size: n_values,
        right_shift,
    };
    let mut stack_size = 1;

    while stack_size > 0 {
        stack_size -= 1;
        let task = task_stack[stack_size];

        let mut n_zeros = 0;
        let mut n_ones = 0;

        let bucket = &mut values[task.bucket_range()];

        while n_zeros + n_ones < task.bucket_size {
            let value = bucket[n_zeros];
            if (value >> task.right_shift) & 0b1 == 1 {
                let dest = task.bucket_size - 1 - n_ones;
                if dest != n_zeros {
                    bucket.swap(n_zeros, dest);
                }
                n_ones += 1;
            } else {
                n_zeros += 1;
            }
        }

        if task.right_shift == 0 {
            continue;
        }
        if n_zeros > 1 {
            task_stack[stack_size] = BucketSortTask {
                bucket_offset: task.bucket_offset,
                bucket_size: n_zeros,
                right_shift: task.right_shift - 1,
            };
            stack_size += 1;
        }
        if n_ones > 1 {
            task_stack[stack_size] = BucketSortTask {
                bucket_offset: task.bucket_offset + n_zeros,
                bucket_size: n_ones,
                right_shift: task.right_shift - 1,
            };
            stack_size += 1;
        }
    }
}

#[derive(Copy, Clone, Debug, Default)]
#[repr(C, align(16))]
struct ArgSortEntry {
    pub key: u64,
    pub index: usize,
}

#[derive(Clone, Copy, Default)]
struct BucketSortTask {
    bucket_offset: usize,
    bucket_size: usize,
    right_shift: u32,
}

impl BucketSortTask {
    fn bucket_range(&self) -> Range<usize> {
        self.bucket_offset..self.bucket_offset + self.bucket_size
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn radix_sort_msd_1_bit_u64_works() {
        let mut values = [
            0b100, 0b111, 0b001, 0b011, 0b110, 0b010, 0b101, 0b001, 0b000,
        ];
        radix_sort_msd_1_bit_u64(&mut values);
        assert_eq!(
            &values,
            &[
                0b000, 0b001, 0b001, 0b010, 0b011, 0b100, 0b101, 0b110, 0b111
            ]
        );
    }

    #[test]
    fn radix_sort_msd_8_bit_u64_works() {
        let mut values = [
            0b100,
            0b111,
            0b001,
            0b011,
            0b1111111110,
            0b010,
            0b101,
            0b001,
            0b000,
        ];
        radix_sort_msd_8_bit_u64(&mut values);
        assert_eq!(
            &values,
            &[
                0b000,
                0b001,
                0b001,
                0b010,
                0b011,
                0b100,
                0b101,
                0b111,
                0b1111111110
            ]
        );
    }

    #[test]
    fn radix_sort_lsd_8_bit_u64_works() {
        let mut values = [
            0b100,
            0b111,
            0b001,
            0b101111111110,
            0b011,
            0b1111111110,
            0b010,
            0b101,
            0b001,
            0b000,
        ];
        radix_sort_lsd_8_bit_u64(&mut values);
        assert_eq!(
            &values,
            &[
                0b000,
                0b001,
                0b001,
                0b010,
                0b011,
                0b100,
                0b101,
                0b111,
                0b1111111110,
                0b101111111110,
            ]
        );
    }

    #[test]
    fn radix_sort_lsd_8_bit_by_u64_keys_works() {
        let values = [
            0b100,
            0b111,
            0b001,
            0b101111111110,
            0b011,
            0b1111111110,
            0b010,
            0b101,
            0b001,
            0b000,
        ];
        let mut sorted_indices = vec![0; values.len()];
        radix_sort_by_u64_keys(&values, |src_idx, dest_idx| {
            sorted_indices[dest_idx] = src_idx;
        });
        assert_eq!(&sorted_indices, &[9, 2, 8, 6, 4, 0, 7, 1, 5, 3]);
    }

    #[test]
    fn radix_sort_lsd_8_bit_by_u64_keys_with_only_zeros_works() {
        let values = [0; 5];
        let mut sorted_indices = vec![0; values.len()];
        radix_sort_by_u64_keys(&values, |src_idx, dest_idx| {
            sorted_indices[dest_idx] = src_idx;
        });
        assert_eq!(&sorted_indices, &[0, 1, 2, 3, 4]);
    }
}
