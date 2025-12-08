//! Arena allocation.

use bumpalo::Bump;
use std::cell::RefCell;

pub type Arena = Bump;

thread_local! {
    static THREAD_LOCAL_ARENA: RefCell<Bump> = RefCell::new(Arena::new());
}

/// Thread-local arenas for allocating memory that will not outlive the task.
#[derive(Debug)]
pub struct TaskArenas;

impl TaskArenas {
    /// Calls the given closure with this thread's per-task arena, and resets
    /// the arena afterwards.
    pub fn with<R>(f: impl FnOnce(&Arena) -> R) -> R {
        let result = THREAD_LOCAL_ARENA.with(|arena| f(&arena.borrow()));
        THREAD_LOCAL_ARENA.with(|arena| {
            let mut arena = arena.borrow_mut();
            impact_log::debug!(
                "Resetting task arena with {} allocated bytes",
                arena.allocated_bytes()
            );
            arena.reset();
        });
        result
    }
}
