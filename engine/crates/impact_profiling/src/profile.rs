//! Utilities for initiating profiling.

use crate::{BasicProfiler, Delayer};
use std::time::{Duration, Instant};

#[macro_export]
macro_rules! define_target_enum {
(
    $name:ident,
    $benchmarks_mod:path,
    $(
        $module:ident => {
            $($func:ident),* $(,)?
        }
    ),* $(,)?
) => {
    ::pastey::paste! {
        #[allow(clippy::enum_variant_names)]
        #[cfg_attr(feature = "cli", derive(::clap::ValueEnum))]
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
        pub enum $name {
            $(
                $( [<$module:camel $func:camel>] ),*
            ),*
        }

        impl $name {
            fn execute(&self, profiler: impl $crate::Profiler) {
                match self {
                    $(
                        $( Self::[<$module:camel $func:camel>] => $benchmarks_mod::$module::$func(profiler), )*
                    )*
                }
            }
        }
    }};
}

pub fn profile(execute: impl Fn(BasicProfiler), duration: f64, delay: f64) {
    let start = Instant::now();

    let delayer = Delayer::new(start, delay);
    let duration = Duration::from_secs_f64(duration);

    let profiler = BasicProfiler::new(duration, delayer);

    execute(profiler);
}
