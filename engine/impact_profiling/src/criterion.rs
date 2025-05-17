//! Profiling using `criterion`.

use crate::Profiler;
use criterion::Criterion;

#[macro_export]
macro_rules! define_criterion_target {
    ($group:ident, $name:ident) => {
        pub fn $name(c: &mut ::criterion::Criterion) {
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
