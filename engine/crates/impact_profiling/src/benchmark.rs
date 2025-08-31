#[cfg(feature = "criterion")]
pub mod criterion;

use std::{
    hint::black_box,
    time::{Duration, Instant},
};

pub trait Benchmarker {
    fn benchmark<T>(self, f: &mut impl FnMut() -> T);
}

#[derive(Clone, Debug)]
pub struct BasicBenchmarker {
    duration: Duration,
    delayer: Delayer,
}

#[derive(Clone, Debug)]
pub struct Delayer {
    program_start: Instant,
    delay: Duration,
}

impl BasicBenchmarker {
    pub fn new(duration: Duration, delayer: Delayer) -> Self {
        Self { duration, delayer }
    }
}

impl Benchmarker for BasicBenchmarker {
    fn benchmark<T>(self, f: &mut impl FnMut() -> T) {
        self.delayer.wait();
        let start = Instant::now();
        loop {
            black_box(f());

            if start.elapsed() > self.duration {
                break;
            }
        }
    }
}

impl Delayer {
    pub fn new(program_start: Instant, delay_seconds: f64) -> Self {
        Self {
            program_start,
            delay: Duration::from_secs_f64(delay_seconds),
        }
    }

    fn wait(self) {
        let remaining = self.delay.saturating_sub(self.program_start.elapsed());
        if remaining > Duration::ZERO {
            std::thread::sleep(remaining);
        }
    }
}

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
            fn execute(&self, benchmarker: impl $crate::benchmark::Benchmarker) {
                match self {
                    $(
                        $( Self::[<$module:camel $func:camel>] => $benchmarks_mod::$module::$func(benchmarker), )*
                    )*
                }
            }
        }
    }};
}

pub fn benchmark(execute: impl Fn(BasicBenchmarker), duration: f64, delay: f64) {
    let start = Instant::now();

    let delayer = Delayer::new(start, delay);
    let duration = Duration::from_secs_f64(duration);

    let benchmarker = BasicBenchmarker::new(duration, delayer);

    execute(benchmarker);
}
