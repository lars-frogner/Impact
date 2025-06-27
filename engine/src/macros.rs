//! Crate-local macros and utility macros.

macro_rules! instrument_engine_task {
    ($description:expr, $engine:expr, $expression:expr) => {{
        const TIMED_TASK_ID: $crate::instrumentation::timing::TimedTaskID =
            $crate::instrumentation::timing::TimedTaskID::new($description);
        ::impact_log::trace!(concat!("Begin: ", $description));
        let _result = $engine.task_timer().time(TIMED_TASK_ID, || $expression);
        ::impact_log::trace!(concat!("Done: ", $description));
        _result
    }};
}

/// This macro expands to a compile time constant equal
/// to the number of arguments passed to the macro.
#[doc(hidden)]
#[macro_export]
macro_rules! count_ident_args {
    ($($arg:ident),*) => {
        // Ugly hack utilizing that `[]::len` is a `const fn`
        // (the extra "" and -1 are needed for the hack to work for zero args)
        ["", $(stringify!($arg)),*].len() - 1
    };
}
