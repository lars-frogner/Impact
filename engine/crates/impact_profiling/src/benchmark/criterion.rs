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
                ),
            );
        }
    };
}

#[allow(missing_debug_implementations)]
pub struct CriterionFunctionBenchmarker<'a> {
    c: &'a mut Criterion,
    id: &'static str,
}

impl<'a> CriterionFunctionBenchmarker<'a> {
    pub fn new(c: &'a mut Criterion, id: &'static str) -> Self {
        Self { c, id }
    }
}

impl Benchmarker for CriterionFunctionBenchmarker<'_> {
    fn benchmark<T>(self, mut f: &mut impl FnMut() -> T) {
        self.c.bench_function(self.id, |b| b.iter(&mut f));
    }
}

pub fn config() -> Criterion {
    Criterion::default()
}
