#[macro_export]
macro_rules! with_timing_info_logging {
    ($message:expr $(,$arg:expr)*; $expression:expr) => {{
        let _start_time = ::std::time::Instant::now();
        let _result = $expression;
        let _duration = _start_time.elapsed();
        $crate::__log::info!(
            concat!($message, " took {:.2} ms")$(,$arg)*,
            _duration.as_secs_f64() * 1e3,
        );
        _result
    }};
}

#[macro_export]
macro_rules! with_trace_logging {
    ($message:expr $(,$arg:expr)*; $expression:expr) => {{
        $crate::__log::trace!(concat!("Begin: ", $message)$(,$arg)*);
        let _start_time = ::std::time::Instant::now();
        let _result = $expression;
        let _duration = _start_time.elapsed();
        $crate::__log::trace!(
            concat!("({:.2} ms) Done: ", $message),
            _duration.as_secs_f64() * 1e3
            $(,$arg)*
        );
        _result
    }};
}

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
