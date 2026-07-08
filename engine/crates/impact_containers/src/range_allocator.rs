//! Tracker for a set of free ranges in a buffer.

use impact_alloc::{AVec, arena::ArenaPool};
use std::{collections::BTreeSet, ops::Range};

/// Tracker for a set of free ranges in a buffer.
#[derive(Clone, Debug)]
pub struct RangeAllocator {
    free_ranges: BTreeSet<RangeByStart>,
}

#[derive(Clone, Debug)]
struct RangeByStart(Range<usize>);

impl RangeAllocator {
    /// Creates a new allocator with no free ranges. Call [`Self::free_range`]
    /// to free an initial range.
    pub fn fully_occupied() -> Self {
        Self {
            free_ranges: BTreeSet::new(),
        }
    }

    /// Frees the given range.
    pub fn free_range(&mut self, range: &Range<usize>) {
        if !range.is_empty() {
            self.free_ranges.insert(range.into());
        }
    }

    /// Resets the allocator by removing all free ranges.
    pub fn mark_all_ranges_occupied(&mut self) {
        self.free_ranges.clear();
    }

    /// Finds a range of the required length among the free ranges and marks the
    /// required part of it occupied, returning the allocated range. Returns
    /// [`None`] if no free range of the required length was found.
    ///
    /// # Panics
    /// If `required_len` is zero.
    pub fn allocate_range(&mut self, required_len: usize) -> Option<Range<usize>> {
        assert!(required_len > 0);

        // Search through the free ranges for the smallest range that can fit the
        // required length
        let mut taken_range = None;
        let mut best_len = usize::MAX;
        for range in &self.free_ranges {
            let len = range.0.len();
            if len < best_len && len >= required_len {
                taken_range = Some(range.clone());
                best_len = len;
            }
        }

        taken_range.map(|range| {
            // If we found a range, we remove it from the list and then re-insert the part
            // of the range that we do not need
            self.free_ranges.remove(&range);

            let remaining_range = &((range.0.start + required_len)..range.0.end);
            if !remaining_range.is_empty() {
                self.free_ranges.insert(remaining_range.into());
            }

            range.0.start..range.0.start + required_len
        })
    }

    /// Merges any consecutive free ranges. If this is not done sufficiently
    /// often after freeing ranges, [`Self::allocate_range`] may fail to find
    /// free ranges that are indeed free.
    pub fn merge_consecutive_ranges(&mut self) {
        if self.free_ranges.len() < 2 {
            return;
        }

        let arena = ArenaPool::get_arena();
        let mut consecutive_ranges = AVec::<Range<usize>, _>::new_in(&arena);
        let mut consecutive_range_counts = AVec::new_in(&arena);

        let mut iter = self.free_ranges.iter();
        let mut prev = iter.next().unwrap();
        for curr in iter {
            if curr.0.start == prev.0.end {
                if matches!(consecutive_ranges.last(), Some(last) if last.end == prev.0.end) {
                    *consecutive_range_counts.last_mut().unwrap() += 1;
                } else {
                    consecutive_ranges.push(prev.0.clone());
                    consecutive_range_counts.push(2);
                }
                consecutive_ranges.push(curr.0.clone());
            }
            prev = curr;
        }

        let mut offset = 0;
        for &count in &consecutive_range_counts {
            let ranges = &consecutive_ranges[offset..offset + count];
            for range in ranges {
                self.free_ranges.remove(&range.into());
            }
            let merged_range = &(ranges.first().unwrap().start..ranges.last().unwrap().end);
            self.free_ranges.insert(merged_range.into());
            offset += count;
        }
    }

    #[cfg(test)]
    fn verify(&self) {
        if self.free_ranges.len() < 2 {
            return;
        }
        let mut iter = self.free_ranges.iter();
        let mut prev = iter.next().unwrap();
        for curr in iter {
            assert!(curr.0.start >= prev.0.end, "Found overlapping free ranges");
            prev = curr;
        }
    }
}

impl From<&Range<usize>> for RangeByStart {
    fn from(range: &Range<usize>) -> Self {
        Self(range.clone())
    }
}

impl PartialEq for RangeByStart {
    fn eq(&self, other: &Self) -> bool {
        self.0.start == other.0.start
    }
}

impl Eq for RangeByStart {}

impl Ord for RangeByStart {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.start.cmp(&other.0.start)
    }
}

impl PartialOrd for RangeByStart {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allocates_nothing_before_freed() {
        let mut alloc = RangeAllocator::fully_occupied();
        assert!(alloc.allocate_range(1).is_none());
        alloc.verify();
    }

    #[test]
    fn frees_and_allocates_single_range() {
        let mut alloc = RangeAllocator::fully_occupied();
        alloc.free_range(&(2..6));
        alloc.verify();
        assert_eq!(alloc.allocate_range(4).unwrap(), 2..6);
        alloc.verify();
        assert!(alloc.allocate_range(1).is_none());
        alloc.verify();
    }

    #[test]
    fn allocates_range_in_smallest_slot() {
        let mut alloc = RangeAllocator::fully_occupied();
        alloc.free_range(&(2..6));
        alloc.free_range(&(10..12));
        alloc.verify();
        assert_eq!(alloc.allocate_range(2).unwrap(), 10..12);
        assert_eq!(alloc.allocate_range(4).unwrap(), 2..6);
        alloc.verify();
    }

    #[test]
    fn uses_parts_of_larger_slots() {
        let mut alloc = RangeAllocator::fully_occupied();
        alloc.free_range(&(2..12));
        assert_eq!(alloc.allocate_range(4).unwrap(), 2..6);
        alloc.verify();
        assert_eq!(alloc.allocate_range(4).unwrap(), 6..10);
        alloc.verify();
        assert!(alloc.allocate_range(4).is_none());
        alloc.verify();
        assert_eq!(alloc.allocate_range(2).unwrap(), 10..12);
        alloc.verify();
        assert!(alloc.allocate_range(1).is_none());
        alloc.verify();
    }

    #[test]
    fn does_not_merge_two_disconnected_free_ranges() {
        let mut alloc = RangeAllocator::fully_occupied();
        alloc.free_range(&(2..5));
        alloc.free_range(&(6..9));
        alloc.verify();
        alloc.merge_consecutive_ranges();
        alloc.verify();
        assert!(alloc.allocate_range(6).is_none());
        alloc.verify();
    }

    #[test]
    fn merges_two_consecutive_free_ranges() {
        let mut alloc = RangeAllocator::fully_occupied();
        alloc.free_range(&(2..6));
        alloc.free_range(&(6..8));
        alloc.merge_consecutive_ranges();
        alloc.verify();
        assert_eq!(alloc.allocate_range(6).unwrap(), 2..8);
        assert!(alloc.allocate_range(1).is_none());
        alloc.verify();
    }

    #[test]
    fn merges_three_consecutive_free_ranges() {
        let mut alloc = RangeAllocator::fully_occupied();
        alloc.free_range(&(2..6));
        alloc.free_range(&(6..8));
        alloc.free_range(&(8..42));
        alloc.merge_consecutive_ranges();
        alloc.verify();
        assert_eq!(alloc.allocate_range(40).unwrap(), 2..42);
        assert!(alloc.allocate_range(1).is_none());
        alloc.verify();
    }

    #[test]
    fn merges_four_consecutive_free_ranges() {
        let mut alloc = RangeAllocator::fully_occupied();
        alloc.free_range(&(2..6));
        alloc.free_range(&(6..8));
        alloc.free_range(&(8..42));
        alloc.free_range(&(42..50));
        alloc.merge_consecutive_ranges();
        alloc.verify();
        assert_eq!(alloc.allocate_range(48).unwrap(), 2..50);
        assert!(alloc.allocate_range(1).is_none());
        alloc.verify();
    }
}
