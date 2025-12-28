use impact_math::benchmark::benchmarks::quaternion;
use impact_profiling::{benchmark::criterion, define_criterion_target};

define_criterion_target!(quaternion, unpack_quaternion);
define_criterion_target!(quaternion, pack_quaternion);
define_criterion_target!(quaternion, mul_quaternion_unpacked);
define_criterion_target!(quaternion, mul_quaternion_both_packed_as_unpacked);
define_criterion_target!(quaternion, mul_quaternion_one_packed_as_unpacked);
define_criterion_target!(quaternion, mul_quaternion_one_packed_as_unpacked_to_packed);
define_criterion_target!(quaternion, mul_quaternion_both_packed_as_unpacked_to_packed);

criterion::criterion_group!(
    name = benches;
    config = criterion::config();
    targets =
        unpack_quaternion,
        pack_quaternion,
        mul_quaternion_unpacked,
        mul_quaternion_both_packed_as_unpacked,
        mul_quaternion_one_packed_as_unpacked,
        mul_quaternion_one_packed_as_unpacked_to_packed,
        mul_quaternion_both_packed_as_unpacked_to_packed,
);
criterion::criterion_main!(benches);
