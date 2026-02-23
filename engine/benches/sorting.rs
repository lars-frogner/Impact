use impact::benchmark::benchmarks::sorting;
use impact_profiling::{benchmark::criterion, define_criterion_target};

define_criterion_target!(sorting, radix_sort_u64);
define_criterion_target!(sorting, radix_sort_by_u64_keys);
define_criterion_target!(sorting, std_sort_u64);

criterion::criterion_group!(
    name = benches;
    config = criterion::config();
    targets =
        radix_sort_u64,
        radix_sort_by_u64_keys,
        std_sort_u64,
);
criterion::criterion_main!(benches);
