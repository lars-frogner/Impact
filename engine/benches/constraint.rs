use criterion::{Criterion, criterion_group, criterion_main};
use impact::profiling::benchmarks::constraint;
use impact_profiling::define_criterion_target;
use pprof::criterion::{Output, PProfProfiler};

define_criterion_target!(constraint, prepare_contacts);
define_criterion_target!(constraint, solve_contact_velocities);
define_criterion_target!(constraint, correct_contact_configurations);

criterion_group!(
    name = benches;
    config = Criterion::default().with_profiler(PProfProfiler::new(100, Output::Flamegraph(None)));
    targets =
        prepare_contacts,
        solve_contact_velocities,
        correct_contact_configurations,
);
criterion_main!(benches);
