//! Allocation.

use bumpalo::Bump;
use std::cell::RefCell;

thread_local! {
    static THREAD_LOCAL_ARENA: RefCell<Bump> = RefCell::new(Bump::new());
}

/// Thread-local arenas for allocating memory that will not outlive the task.
#[derive(Debug)]
pub struct TaskArenas;

impl TaskArenas {
    /// Calls the given closure with this thread's per-task arena, and resets
    /// the arena afterwards.
    pub fn with<R>(f: impl FnOnce(&Bump) -> R) -> R {
        let result = THREAD_LOCAL_ARENA.with(|arena| f(&arena.borrow()));
        THREAD_LOCAL_ARENA.with(|arena| arena.borrow_mut().reset());
        result
    }
}
