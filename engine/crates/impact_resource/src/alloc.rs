//! Allocation.

use bumpalo::Bump;
use std::cell::RefCell;

thread_local! {
    static THREAD_LOCAL_ARENA: RefCell<Bump> = RefCell::new(Bump::new());
}

/// Thread-local arenas for allocating memory that will not outlive a resource
/// operation.
#[derive(Debug)]
pub struct ResourceOperationArenas;

impl ResourceOperationArenas {
    /// Calls the given closure with this thread's resource operation arena, and
    /// resets the arena afterwards.
    pub fn with<R>(f: impl FnOnce(&Bump) -> R) -> R {
        let result = THREAD_LOCAL_ARENA.with(|arena| f(&arena.borrow()));
        THREAD_LOCAL_ARENA.with(|arena| {
            let mut arena = arena.borrow_mut();
            impact_log::debug!(
                "Resetting resource operation arena with {} allocated bytes",
                arena.allocated_bytes()
            );
            arena.reset();
        });
        result
    }
}
