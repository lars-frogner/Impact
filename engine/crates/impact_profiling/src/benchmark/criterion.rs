//! Benchmarking using `criterion`.

pub use criterion::*;

use crate::benchmark::Benchmarker;

#[macro_export]
macro_rules! define_criterion_target {
    ($group:ident, $name:ident) => {
        pub fn $name(c: &mut $crate::benchmark::criterion::Criterion) {
            $group::$name(
                $crate::benchmark::criterion::CriterionFunctionBenchmarker::new(
                    c,
                    stringify!($name),
                    None,
                ),
            );
        }
    };
    ($group:ident, $name:ident, $sample_count:expr) => {
        pub fn $name(c: &mut $crate::benchmark::criterion::Criterion) {
            $group::$name(
                $crate::benchmark::criterion::CriterionFunctionBenchmarker::new(
                    c,
                    stringify!($name),
                    Some($sample_count),
                ),
            );
        }
    };
}

#[allow(missing_debug_implementations)]
pub struct CriterionFunctionBenchmarker<'a> {
    c: &'a mut Criterion,
    id: &'static str,
    sample_count: Option<usize>,
}

impl<'a> CriterionFunctionBenchmarker<'a> {
    pub fn new(c: &'a mut Criterion, id: &'static str, sample_count: Option<usize>) -> Self {
        Self {
            c,
            id,
            sample_count,
        }
    }
}

impl Benchmarker for CriterionFunctionBenchmarker<'_> {
    fn benchmark<T>(self, mut f: &mut impl FnMut() -> T) {
        let mut benchmark_group = self.c.benchmark_group(self.id);

        if let Some(sample_count) = self.sample_count {
            benchmark_group.sample_size(sample_count);
        }

        benchmark_group.bench_function(self.id, |b| b.iter(&mut f));
        benchmark_group.finish();
    }
}

pub fn config() -> Criterion {
    Criterion::default()
}
