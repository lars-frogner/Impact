use impact_math::benchmark::benchmarks::isometry;
use impact_profiling::{benchmark::criterion, define_criterion_target};

define_criterion_target!(isometry, unpack_isometry);
define_criterion_target!(isometry, pack_isometry);
define_criterion_target!(isometry, mul_isometry_uncompact);
define_criterion_target!(isometry, mul_isometry_both_compact_as_uncompact);
define_criterion_target!(isometry, mul_isometry_one_compact_as_uncompact);
define_criterion_target!(isometry, mul_isometry_one_compact_as_uncompact_to_compact);
define_criterion_target!(isometry, mul_isometry_both_compact_as_uncompact_to_compact);

criterion::criterion_group!(
    name = benches;
    config = criterion::config();
    targets =
        unpack_isometry,
        pack_isometry,
        mul_isometry_uncompact,
        mul_isometry_both_compact_as_uncompact,
        mul_isometry_one_compact_as_uncompact,
        mul_isometry_one_compact_as_uncompact_to_compact,
        mul_isometry_both_compact_as_uncompact_to_compact,
);
criterion::criterion_main!(benches);
