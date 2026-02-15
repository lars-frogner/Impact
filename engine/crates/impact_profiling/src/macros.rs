#[macro_export]
macro_rules! instrument_task {
    ($label:expr, $timer:expr, $expression:expr) => {{
        let _result = $timer.time($label, || {
            let _span = impact_profiling::instrumentation::span!($label);
            $expression
        });
        _result
    }};
}
