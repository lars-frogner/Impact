//! Crate-local macros and utility macros.

macro_rules! instrument_engine_task {
    ($description:expr, $engine:expr, $expression:expr) => {{
        const TIMED_TASK_ID: $crate::instrumentation::timing::TimedTaskID =
            $crate::instrumentation::timing::TimedTaskID::new($description);
        ::log::trace!(concat!("Begin: ", $description));
        let _result = $engine.task_timer().time(TIMED_TASK_ID, || {
            let _span = impact_profiling::instrumentation::span!($description);
            $expression
        });
        ::log::trace!(concat!("Done: ", $description));
        _result
    }};
}
