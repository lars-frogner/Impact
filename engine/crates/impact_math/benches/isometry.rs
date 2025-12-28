use impact_math::benchmark::benchmarks::isometry;
use impact_profiling::{benchmark::criterion, define_criterion_target};

define_criterion_target!(isometry, unpack_isometry);
define_criterion_target!(isometry, pack_isometry);
define_criterion_target!(isometry, mul_isometry_unpacked);
define_criterion_target!(isometry, mul_isometry_both_packed_as_unpacked);
define_criterion_target!(isometry, mul_isometry_one_packed_as_unpacked);
define_criterion_target!(isometry, mul_isometry_one_packed_as_unpacked_to_packed);
define_criterion_target!(isometry, mul_isometry_both_packed_as_unpacked_to_packed);

criterion::criterion_group!(
    name = benches;
    config = criterion::config();
    targets =
        unpack_isometry,
        pack_isometry,
        mul_isometry_unpacked,
        mul_isometry_both_packed_as_unpacked,
        mul_isometry_one_packed_as_unpacked,
        mul_isometry_one_packed_as_unpacked_to_packed,
        mul_isometry_both_packed_as_unpacked_to_packed,
);
criterion::criterion_main!(benches);
