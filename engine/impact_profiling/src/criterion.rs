//! Profiling using `criterion`.

pub use criterion::*;

use crate::Profiler;

#[macro_export]
macro_rules! define_criterion_target {
    ($group:ident, $name:ident) => {
        pub fn $name(c: &mut $crate::criterion::Criterion) {
            $group::$name($crate::criterion::CriterionFunctionProfiler::new(
                c,
                stringify!($name),
            ));
        }
    };
}

#[allow(missing_debug_implementations)]
pub struct CriterionFunctionProfiler<'a> {
    c: &'a mut Criterion,
    id: &'static str,
}

impl<'a> CriterionFunctionProfiler<'a> {
    pub fn new(c: &'a mut Criterion, id: &'static str) -> Self {
        Self { c, id }
    }
}

impl Profiler for CriterionFunctionProfiler<'_> {
    fn profile<T>(self, mut f: &mut impl FnMut() -> T) {
        self.c.bench_function(self.id, |b| b.iter(&mut f));
    }
}

#[cfg(all(feature = "flamegraph", unix))]
pub fn config() -> Criterion {
    Criterion::default().with_profiler(pprof::criterion::PProfProfiler::new(
        100,
        pprof::criterion::Output::Flamegraph(None),
    ))
}

#[cfg(all(feature = "flamegraph", not(unix)))]
pub fn config() -> Criterion {
    eprintln!("Warning (impact_profiling): The `flamegraph` feature is only supported on unix");
    Criterion::default()
}

#[cfg(not(feature = "flamegraph"))]
pub fn config() -> Criterion {
    Criterion::default()
}
