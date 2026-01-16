use impact_math::benchmark::benchmarks::quaternion;
use impact_profiling::{benchmark::criterion, define_criterion_target};

define_criterion_target!(quaternion, unpack_quaternion);
define_criterion_target!(quaternion, pack_quaternion);
define_criterion_target!(quaternion, mul_quaternion_uncompact);
define_criterion_target!(quaternion, mul_quaternion_both_compact_as_uncompact);
define_criterion_target!(quaternion, mul_quaternion_one_compact_as_uncompact);
define_criterion_target!(
    quaternion,
    mul_quaternion_one_compact_as_uncompact_to_compact
);
define_criterion_target!(
    quaternion,
    mul_quaternion_both_compact_as_uncompact_to_compact
);

criterion::criterion_group!(
    name = benches;
    config = criterion::config();
    targets =
        unpack_quaternion,
        pack_quaternion,
        mul_quaternion_uncompact,
        mul_quaternion_both_compact_as_uncompact,
        mul_quaternion_one_compact_as_uncompact,
        mul_quaternion_one_compact_as_uncompact_to_compact,
        mul_quaternion_both_compact_as_uncompact_to_compact,
);
criterion::criterion_main!(benches);
