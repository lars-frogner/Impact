use impact::profiling::benchmarks::constraint;
use impact_profiling::{criterion, define_criterion_target};

define_criterion_target!(constraint, prepare_contacts);
define_criterion_target!(constraint, solve_contact_velocities);
define_criterion_target!(constraint, correct_contact_configurations);

criterion::criterion_group!(
    name = benches;
    config = criterion::config();
    targets =
        prepare_contacts,
        solve_contact_velocities,
        correct_contact_configurations,
);
criterion::criterion_main!(benches);
