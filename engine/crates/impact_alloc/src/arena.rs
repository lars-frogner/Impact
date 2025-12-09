//! Arena allocation.

use bumpalo::Bump;
use std::{cell::RefCell, ops::Deref};

pub type Arena = Bump;

thread_local! {
    static THREAD_LOCAL_ARENA_POOL: RefCell<ArenaPool> = RefCell::new(ArenaPool::new());
}

/// Pool of reusable arenas.
#[derive(Debug)]
pub struct ArenaPool {
    // The arenas are boxed so that we can keep a stable pointer to them while
    // pushing to the `Vec` (which may reallocate).
    arenas: Vec<BoxedArena>,
    free_list: Vec<usize>,
}

/// Arena from an [`ArenaPool`]. When this object drops, the arena is reset. The
/// arena can not be moved or shared between threads.
///
/// # Warning
/// Do not let any data allocated from this arena outlive the arena.
#[derive(Debug)]
pub struct PoolArena {
    arena_ptr: *const Arena,
    arena_idx: usize,
}

#[derive(Debug)]
struct BoxedArena {
    arena: Box<Arena>,
    max_bytes_allocated: usize,
}

impl ArenaPool {
    /// Returns a fresh arena from the pool.
    ///
    /// When `PoolArena` drops, the arena will be reset and returned to the
    /// pool.
    ///
    /// The arena can not be moved or shared between threads.
    ///
    /// # Warning
    /// Do not let any data allocated from the arena outlive the arena.
    pub fn get_arena() -> PoolArena {
        // Since `PoolArena` holds a raw pointer rather than a reference, we
        // avoid holding a borrow on the `RefCell`, which would lead to a panic
        // if the closure we are calling tried to acquire a new arena.
        THREAD_LOCAL_ARENA_POOL.with(|pool| pool.borrow_mut().acquire())
    }

    /// Returns a fresh arena from the pool.
    ///
    /// Will use the smallest arena with sufficient allocated space for the
    /// target capacity, or if all arenas are too small, the largest arena.
    ///
    /// When `PoolArena` drops, the arena will be reset and returned to the
    /// pool.
    ///
    /// The arena can not be moved or shared between threads.
    ///
    /// # Warning
    /// Do not let any data allocated from the arena outlive the arena.
    pub fn get_arena_for_capacity(capacity: usize) -> PoolArena {
        // Since `PoolArena` holds a raw pointer rather than a reference, we
        // avoid holding a borrow on the `RefCell`, which would lead to a panic
        // if the closure we are calling tried to acquire a new arena.
        THREAD_LOCAL_ARENA_POOL.with(|pool| pool.borrow_mut().acquire_for_capacity(capacity))
    }

    fn new() -> Self {
        Self {
            arenas: Vec::with_capacity(4),
            free_list: Vec::with_capacity(4),
        }
    }

    /// Returns a the pointer and index of a free arena, creating one if needed.
    fn acquire(&mut self) -> PoolArena {
        let arena_idx = self.free_list.pop().unwrap_or_else(|| {
            let arena_idx = self.arenas.len();
            self.arenas.push(BoxedArena::new());
            arena_idx
        });

        let arena_ptr = self.arenas[arena_idx].pointer();

        PoolArena::new(arena_ptr, arena_idx)
    }

    /// Returns a the pointer and index of a free arena, creating one if needed.
    ///
    /// Will use the smallest arena with sufficient allocated space for the
    /// target capacity, or if all arenas are too small, the largest arena.
    fn acquire_for_capacity(&mut self, target_capacity: usize) -> PoolArena {
        let arena_idx = if self.free_list.is_empty() {
            let arena_idx = self.arenas.len();
            self.arenas.push(BoxedArena::new());
            arena_idx
        } else {
            let mut best_idx = usize::MAX;
            let mut best_capacity = 0;
            let mut found_sufficient = false;

            for (idx, &arena_idx) in self.free_list.iter().enumerate() {
                let capacity = self.arenas[arena_idx].max_bytes_allocated;

                if found_sufficient {
                    if capacity >= target_capacity && capacity < best_capacity {
                        best_idx = idx;
                        best_capacity = capacity;
                    }
                } else if capacity >= target_capacity {
                    best_idx = idx;
                    best_capacity = capacity;
                    found_sufficient = true;
                // Use >= to ensure this branch is still taken if all capacities are 0
                } else if capacity >= best_capacity {
                    best_idx = idx;
                    best_capacity = capacity;
                }
            }

            assert_ne!(best_idx, usize::MAX);

            let arena_idx = self.free_list.swap_remove(best_idx);

            arena_idx
        };

        let arena_ptr = self.arenas[arena_idx].pointer();

        PoolArena::new(arena_ptr, arena_idx)
    }

    /// Resets the arena at the given index and marks it as free.
    fn release(&mut self, arena_idx: usize) {
        self.arenas[arena_idx].reset();
        self.free_list.push(arena_idx);
    }
}

impl PoolArena {
    fn new(arena_ptr: *const Arena, arena_idx: usize) -> Self {
        Self {
            arena_ptr,
            arena_idx,
        }
    }
}

impl Deref for PoolArena {
    type Target = Arena;

    fn deref(&self) -> &Self::Target {
        // SAFETY:
        // - The arena is never mutated (except for internal mutation).
        // - The arena is never removed from the `Vec` in `ArenaPool`.
        // - The pointer stays valid across `Vec` reallocations because `Box`
        //   pins the address.
        // - No other `PoolArena` carries this pointer, so dropping another
        //   `PoolArena` will never reset this arena.
        unsafe { &*self.arena_ptr }
    }
}

impl Drop for PoolArena {
    fn drop(&mut self) {
        THREAD_LOCAL_ARENA_POOL.with(|pool| {
            // Since the pool is thread-local and we only call `borrow_mut` to
            // acquire or release an arena, there is no way the `borrow_mut`
            // call can panic
            pool.borrow_mut().release(self.arena_idx);
        });
    }
}

impl BoxedArena {
    fn new() -> Self {
        Self {
            arena: Box::new(Arena::new()),
            max_bytes_allocated: 0,
        }
    }

    fn pointer(&self) -> *const Arena {
        self.arena.as_ref() as *const Arena
    }

    fn reset(&mut self) {
        let allocated_bytes = self.arena.allocated_bytes();
        self.arena.reset();
        self.max_bytes_allocated = self.max_bytes_allocated.max(allocated_bytes);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test data constants
    const TEST_DATA: &[u8; 16] = b"test data here!!";
    const LARGE_SIZE: usize = 1024;
    const SMALL_SIZE: usize = 64;

    #[test]
    fn getting_arena_from_pool_works() {
        let arena = ArenaPool::get_arena();

        // Arena should be usable
        let allocated = arena.alloc_slice_copy(TEST_DATA);
        assert_eq!(allocated, TEST_DATA);
    }

    #[test]
    fn getting_arena_for_capacity_works() {
        let arena = ArenaPool::get_arena_for_capacity(LARGE_SIZE);

        // Arena should be usable
        let allocated = arena.alloc_slice_copy(TEST_DATA);
        assert_eq!(allocated, TEST_DATA);
    }

    #[test]
    fn getting_arena_for_zero_capacity_works() {
        let arena = ArenaPool::get_arena_for_capacity(0);

        // Arena should still be usable
        let allocated = arena.alloc_slice_copy(TEST_DATA);
        assert_eq!(allocated, TEST_DATA);
    }

    #[test]
    fn multiple_arenas_can_coexist() {
        let arena1 = ArenaPool::get_arena();
        let arena2 = ArenaPool::get_arena();

        // Both arenas should be independent and usable
        let data1 = arena1.alloc_slice_copy(&[1u8; 32]);
        let data2 = arena2.alloc_slice_copy(&[2u8; 32]);

        assert_eq!(data1, &[1u8; 32]);
        assert_eq!(data2, &[2u8; 32]);

        // They should have different addresses
        assert_ne!(data1.as_ptr(), data2.as_ptr());
    }

    #[test]
    fn arena_is_reused_after_drop() {
        let reused_ptr: *const Arena;

        // First allocation to establish some usage
        {
            let arena = ArenaPool::get_arena();
            reused_ptr = &*arena as *const Arena;
        } // Arena drops here and returns to pool

        // Second allocation should reuse the same arena
        {
            let arena = ArenaPool::get_arena();
            assert_eq!(&*arena as *const Arena, reused_ptr);
        }
    }

    #[test]
    fn arena_capacity_selection_prefers_smallest_sufficient() {
        // Get two arenas and allocate different sizes
        let arena1 = ArenaPool::get_arena();
        let small_arena_ptr = &*arena1 as *const Arena;
        let _small_alloc = arena1.alloc_slice_copy(&[1u8; SMALL_SIZE]);

        let arena2 = ArenaPool::get_arena();
        let _large_alloc = arena2.alloc_slice_copy(&[2u8; LARGE_SIZE]);

        // Drop both to return them to pool
        drop(arena1);
        drop(arena2);

        // Request capacity that should prefer the smaller arena (SMALL_SIZE < LARGE_SIZE)
        let selected_arena = ArenaPool::get_arena_for_capacity(SMALL_SIZE);
        let selected_ptr = &*selected_arena as *const Arena;

        // Should have selected the small arena
        assert_eq!(selected_ptr, small_arena_ptr);
    }

    #[test]
    fn arena_capacity_selection_uses_largest_when_none_sufficient() {
        // Get two arenas and allocate different sizes
        let arena1 = ArenaPool::get_arena();
        let _small_alloc = arena1.alloc_slice_copy(&[1u8; SMALL_SIZE]);

        let arena2 = ArenaPool::get_arena();
        let large_arena_ptr = &*arena2 as *const Arena;
        let _large_alloc = arena2.alloc_slice_copy(&[2u8; LARGE_SIZE]);

        // Drop both to return them to pool
        drop(arena1);
        drop(arena2);

        // Request capacity larger than any existing arena
        let selected_arena = ArenaPool::get_arena_for_capacity(LARGE_SIZE * 2);
        let selected_ptr = &*selected_arena as *const Arena;

        // Should have selected the large arena (largest available)
        assert_eq!(selected_ptr, large_arena_ptr);
    }
}
