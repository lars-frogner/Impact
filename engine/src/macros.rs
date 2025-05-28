//! Crate-local macros and utility macros.

macro_rules! with_trace_logging {
    ($message:expr $(,$arg:expr)*; $expression:expr) => {{
        log::trace!(concat!("Begin: ", $message)$(,$arg)*);
        let _start_time = ::std::time::Instant::now();
        let _result = $expression;
        let _duration = _start_time.elapsed();
        log::trace!(
            concat!("({:.2} ms) Done: ", $message),
            _duration.as_secs_f64() * 1e3
            $(,$arg)*
        );
        _result
    }};
}

macro_rules! with_timing_info_logging {
    ($message:expr $(,$arg:expr)*; $expression:expr) => {{
        let _start_time = ::std::time::Instant::now();
        let _result = $expression;
        let _duration = _start_time.elapsed();
        log::info!(
            concat!($message, " took {:.2} ms")$(,$arg)*,
            _duration.as_secs_f64() * 1e3,
        );
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
